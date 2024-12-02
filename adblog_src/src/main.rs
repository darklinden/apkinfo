#!/usr/bin/env rust-script
//! * <https://github.com/fornwall/rust-script>
//! * cargo install rust-script
//! Dependencies can be specified in the script file itself as follows:
//!
//! ```cargo
//! [dependencies]
//! clap = { version = "4.5.21", features = ["derive"] }
//! serde_json = "1.0.133"
//! tracing = "0.1.41"
//! tracing-subscriber = { version = "0.3.18", features = [
//!     "env-filter",
//!     "registry",
//!     "time",
//! ] }
//! tracing-appender = "0.2.3"
//! time = { version = "0.3.36", features = ["local-offset"] }
//! anyhow = "1.0.93"
//! tokio = { version = "1.41.1", features = ["full"] }
//! serde = { version = "1.0.215", features = ["derive"] }
//! zip = "2.2.1"
//! dialoguer = { version = "0.11.0", features = ["fuzzy-select"] }
//! ```

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::ffi::OsStr;
use std::fs;
use std::process::Stdio;
use std::{io::stdout, path::Path};
use time::{format_description, UtcOffset};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, time::OffsetTime},
    layer::SubscriberExt,
};

pub fn init_log(project_path: &Path) -> (WorkerGuard, WorkerGuard) {
    std::env::set_var("RUST_BACKTRACE", "1");
    let time_zone_offset = UtcOffset::from_hms(8, 0, 0).expect("should get UTC+8 offset!");

    let format = format_description::parse("[year][month][day]_[hour][minute][second]").unwrap();
    let now = time::OffsetDateTime::now_local()
        .unwrap_or(time::OffsetDateTime::now_utc().to_offset(time_zone_offset))
        .format(&format)
        .unwrap();
    let log_file_name = format!("adblog_{}.log", now);
    let exec_folder = project_path.to_str().unwrap();
    println!(
        "log file path: {}/{}",
        exec_folder.replace("\\", "/"),
        log_file_name
    );
    let log_file = tracing_appender::rolling::never(exec_folder, log_file_name);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(log_file);
    let (non_blocking_stdout, _stdout_guard) = tracing_appender::non_blocking(stdout());

    let timer = OffsetTime::new(
        time_zone_offset,
        time::format_description::well_known::Rfc3339,
    );

    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .without_time()
            .with_writer(non_blocking_stdout)
            .finish()
            .with(
                fmt::Layer::default()
                    .with_timer(timer)
                    .with_writer(non_blocking_file),
            ),
    )
    .expect("Unable to set global tracing subscriber");

    (_stdout_guard, _file_guard)
}

#[cfg(target_os = "windows")]
pub(crate) fn cyg_to_win(path: &str) -> String {
    path.replace("/cygdrive/c", "C:")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn cyg_to_win(path: &str) -> String {
    path.to_string()
}

async fn run_cmd<S, I>(
    work: &str,
    program: S,
    args: I,
    require_output: bool,
) -> Result<(std::process::ExitStatus, Vec<String>)>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = Command::new(program);

    cmd.args(args);

    // Specify that we want the command's standard output piped back to us.
    // By default, standard input/output/error will be inherited from the
    // current process (for example, this means that standard input will
    // come from the keyboard and standard output/error will go directly to
    // the terminal if this process is invoked from the command line).
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn command");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let mut reader = BufReader::new(stdout).lines();

    let (tx, mut rx) = mpsc::channel(2);

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    tokio::spawn(async move {
        let output = child
            .wait_with_output()
            .await
            .expect("child process encountered an error");

        tx.send(output.status).await.unwrap();
    });

    let mut stdout = Vec::new();
    while let Some(line) = reader.next_line().await? {
        tracing::info!("[{}] {}", work, line);
        if require_output {
            stdout.push(line);
        }
    }

    let output_result = rx.recv().await;

    if output_result.is_none() {
        anyhow::bail!("run_cmd: output_result is none");
    }

    Ok((output_result.unwrap(), stdout))
}

pub(crate) fn extract_single_file(src_zip: &Path, sub_file: &str, des_file: &Path) -> Result<()> {
    let file = std::fs::File::open(src_zip)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut file = archive.by_name(sub_file)?;
    if file.is_dir() {
        anyhow::bail!("extract_single_file: {} is a directory", sub_file);
    } else {
        tracing::info!(
            "File {} extracted to \"{}\" ({} bytes)",
            sub_file,
            des_file.display(),
            file.size()
        );
        if let Some(p) = des_file.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }
        let mut out_file = std::fs::File::create(des_file)?;
        std::io::copy(&mut file, &mut out_file)?;
    }

    // Get and Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Some(mode) = file.unix_mode() {
            fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
        }
    }

    Ok(())
}

fn get_value_by_key(src: &str, prefix: &str, key: &str) -> String {
    let src = &src[prefix.len() + 1..];
    let list: Vec<&str> = src.split(' ').collect();
    for kv_pair in list {
        let kv_pair = kv_pair.trim();
        let kv_list: Vec<&str> = kv_pair.split('=').collect();
        if kv_list.len() == 2 {
            let tmp_key = kv_list[0].trim_matches('\'').trim();
            if tmp_key == key {
                return kv_list[1].trim_matches('\'').trim().to_string();
            }
        }
    }
    String::new()
}

async fn adb_get_pid(adb_path: &Path, selected_device: &str, package_name: &str) -> Result<u32> {
    let mut pid = 0;
    let mut limit = 100;
    while pid == 0 && limit > 0 {
        limit -= 1;
        let query_ps = run_cmd(
            "query ps",
            adb_path.to_str().unwrap(),
            [
                "-s",
                selected_device,
                "shell",
                "ps",
                "|",
                "grep",
                package_name,
            ],
            true,
        )
        .await?;

        if !query_ps.0.success() {
            tracing::error!("adb shell ps failed");
            continue;
        }

        if !query_ps.1.is_empty() {
            let ps_list = query_ps.1;
            for ps_line_str in ps_list {
                let ps_num_list: Vec<&str> = ps_line_str.split_whitespace().collect();
                let has_package_name = ps_num_list.iter().any(|&s_str| {
                    s_str
                        .trim_matches('\'')
                        .trim_matches('"')
                        .eq_ignore_ascii_case(package_name)
                });
                if has_package_name {
                    for ps_num_str in ps_num_list {
                        if let Ok(num) = ps_num_str.parse::<u32>() {
                            pid = num;
                            break;
                        }
                    }
                }
                if pid != 0 {
                    tracing::info!("get pid: {}", ps_line_str);
                    break;
                }
            }
        }
    }
    Ok(pid)
}

/// AdbLog: start apk and logcat
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Command [i: install apk; r: reinstall apk; mem: watch memory; cpu: watch cpu;]
    #[arg(short, long, default_value = "i")]
    cmd: String,

    /// Apk or Apks or Aab File Path
    #[arg(short, long)]
    file_path: String,
}

async fn run_adb_log() -> Result<()> {
    let args = Args::parse();
    let file_path = cyg_to_win(&args.file_path);
    let file_path = std::path::absolute(file_path)?;
    if !file_path.exists() {
        anyhow::bail!("adblog: file not exist!");
    }

    let folder = file_path.parent().context("working folder not found")?;
    let _guards = init_log(&folder);

    let script_folder = match std::env::var("RUST_SCRIPT_BASE_PATH") {
        Ok(script_folder) => {
            tracing::info!("RUST_SCRIPT_BASE_PATH exists {}", script_folder);
            script_folder
        }
        Err(_) => {
            tracing::info!("RUST_SCRIPT_BASE_PATH not exists, use CARGO_MANIFEST_DIR");
            std::env::var("CARGO_MANIFEST_DIR")?
        }
    };
    tracing::info!("script_folder: {}", script_folder);
    let script_folder = std::path::absolute(&script_folder)?;
    if !script_folder.is_dir() {
        anyhow::bail!("script_folder not found");
    }

    let android_home = match std::env::var("ANDROID_HOME") {
        Ok(android_home) => {
            tracing::info!("ANDROID_HOME exists {}", android_home);
            android_home
        }
        Err(_) => {
            tracing::info!("ANDROID_HOME not exists, use ANDROID_SDK_ROOT");
            std::env::var("ANDROID_SDK_ROOT")?
        }
    };
    tracing::info!("android_home: {}", android_home);
    let android_home = cyg_to_win(&android_home);
    let android_home = std::path::absolute(&android_home)?;
    if !android_home.is_dir() {
        anyhow::bail!("android_home not found");
    }

    let bundle_tool_path = script_folder.join("bundletool-all-1.16.0.jar");
    if !bundle_tool_path.exists() {
        anyhow::bail!("bundletool-all-1.16.0.jar not found");
    }

    let platform_tools_path = android_home.join("platform-tools");
    let adb_path = fs::read_dir(platform_tools_path)?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let file_path = entry.path();
                if file_path.is_file()
                    && file_path
                        .file_name()
                        .is_some_and(|f| f.to_str().is_some_and(|ff| ff.starts_with("adb")))
                {
                    Some(file_path)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::error!("Error: {}", e);
                None
            }
        })
        .next()
        .context("no adb found")?;
    tracing::info!("adb_path: {}", adb_path.display());
    if !adb_path.is_file() {
        anyhow::bail!("adb not found");
    }

    let build_tools_path = android_home.join("build-tools");
    let build_tools = fs::read_dir(build_tools_path)?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                if entry.path().is_dir() {
                    Some(entry.path())
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::info!("Error: {}", e);
                None
            }
        })
        .collect::<Vec<_>>();
    let latest_build_tools = build_tools.iter().max().context("no build tools found")?;
    tracing::info!("latest_build_tools: {}", latest_build_tools.display());

    let aapt2_path = fs::read_dir(latest_build_tools)?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let file_path = entry.path();
                if file_path.is_file()
                    && file_path
                        .file_name()
                        .is_some_and(|f| f.to_str().is_some_and(|ff| ff.starts_with("aapt2")))
                {
                    Some(file_path)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::info!("Error: {}", e);
                None
            }
        })
        .next()
        .context("no aapt2 found")?;
    tracing::info!("aapt2_path: {}", aapt2_path.display());
    if !aapt2_path.is_file() {
        anyhow::bail!("aapt2 not found");
    }

    let refresh_devices = run_cmd(
        "refresh devices",
        adb_path.to_str().unwrap(),
        ["kill-server"],
        false,
    )
    .await?;
    if !refresh_devices.0.success() {
        anyhow::bail!("adb kill-server failed");
    }

    let devices_str = run_cmd(
        "list devices",
        adb_path.to_str().unwrap(),
        ["devices"],
        true,
    )
    .await?;
    if !devices_str.0.success() {
        anyhow::bail!("adb devices failed");
    }

    let mut device_list = Vec::new();
    let mut device_info_list = Vec::new();

    for l in devices_str.1 {
        if l.trim().is_empty() {
            continue;
        }
        let dl: Vec<&str> = l.split('\t').collect();
        if dl.len() == 2 {
            tracing::info!("device: {}", dl[0]);
            device_list.push(dl[0].to_string());
            device_info_list.push(l.to_string());
        }
    }

    let selected_device = if device_list.is_empty() {
        anyhow::bail!("no device found");
    } else if device_list.len() == 1 {
        device_list.first().unwrap().to_string()
    } else {
        tracing::info!("please select device:");

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Pick Device:")
            .default(0)
            .items(&device_info_list[..])
            .interact()?;

        device_list[selection].to_string()
    };

    tracing::info!("selected device: {}", selected_device);

    let exec_path = if "aab"
        == file_path
            .extension()
            .context("file_path no extension")?
            .to_str()
            .unwrap_or("")
    {
        let apks_path = file_path.with_extension("").with_extension("apks");
        if apks_path.is_file() {
            fs::remove_file(&apks_path)?
        }

        let aab_to_apks = run_cmd(
            "aab to apks",
            "java",
            [
                "-jar",
                "bundletool-all-1.16.0.jar",
                "build-apks",
                &format!("--bundle={}", file_path.to_str().unwrap()),
                &format!("--output={}", apks_path.to_str().unwrap()),
            ],
            false,
        )
        .await?;
        if !aab_to_apks.0.success() {
            anyhow::bail!("bundletool build-apks failed");
        }
        apks_path
    } else {
        file_path
    };

    let apk_to_read = if "apks"
        == exec_path
            .extension()
            .context("exec_path no extension")?
            .to_str()
            .unwrap_or("")
    {
        let apks_unzip = exec_path.with_extension("").with_extension("unzip");
        if apks_unzip.is_dir() {
            fs::remove_dir_all(&apks_unzip)?;
        }
        fs::create_dir(&apks_unzip)?;

        let apk_to_read = apks_unzip.join("base-master.apk");
        extract_single_file(&exec_path, "splits/base-master.apk", &apk_to_read)?;

        apk_to_read
    } else {
        exec_path.clone()
    };

    let aapt_dump = run_cmd(
        "get package and activity",
        aapt2_path.to_str().unwrap(),
        ["dump", "badging", apk_to_read.to_str().unwrap()],
        true,
    )
    .await?;
    if !aapt_dump.0.success() {
        anyhow::bail!("aapt2 dump badging failed");
    }

    let mut package_str = String::new();
    let mut activity_str = String::new();

    for line in aapt_dump.1 {
        if line.starts_with("package:") {
            package_str = line.to_string();
        }
        if line.starts_with("launchable-activity:") {
            activity_str = line.to_string();
        }
        if !package_str.is_empty() && !activity_str.is_empty() {
            break;
        }
    }

    let package_name = get_value_by_key(&package_str, "package:", "name");
    let activity_name = get_value_by_key(&activity_str, "launchable-activity:", "name");

    tracing::info!(
        "get package name {} activity name {}",
        package_name,
        activity_name
    );

    let cmd = args.cmd.as_str();

    if "r" == cmd {
        let get_installed = run_cmd(
            "get installed",
            adb_path.to_str().unwrap(),
            ["-s", &selected_device, "shell", "pm", "list", "packages"],
            true,
        )
        .await?;
        if !get_installed.0.success() {
            anyhow::bail!("adb shell pm list packages failed");
        }

        let installed = get_installed
            .1
            .iter()
            .any(|l| l.trim().ends_with(&package_name));
        if installed {
            let uninstall = run_cmd(
                "uninstall",
                adb_path.to_str().unwrap(),
                ["-s", &selected_device, "uninstall", &package_name],
                false,
            )
            .await?;
            if !uninstall.0.success() {
                anyhow::bail!("adb uninstall failed");
            }
        }
    }

    let install_result = if exec_path.ends_with(".apks") {
        let install = run_cmd(
            "install apks",
            "java",
            [
                "-jar",
                "bundletool-all-1.16.0.jar",
                "install-apks",
                &format!("--adb=adb"),
                &format!("--device-id={}", selected_device),
                &format!("--apks={}", exec_path.to_str().unwrap()),
            ],
            false,
        )
        .await?;
        install
    } else {
        let install = run_cmd(
            "install apk",
            adb_path.to_str().unwrap(),
            [
                "-s",
                &selected_device,
                "install",
                &exec_path.to_str().unwrap(),
            ],
            false,
        )
        .await?;
        install
    };

    if !install_result.0.success() {
        anyhow::bail!("adb install failed");
    }

    tracing::info!("starting process ...");

    let start = run_cmd(
        "start process",
        adb_path.to_str().unwrap(),
        [
            "-s",
            &selected_device,
            "shell",
            "am",
            "start",
            "-S",
            &format!("{}/{}", package_name, activity_name),
        ],
        false,
    )
    .await?;

    if !start.0.success() {
        anyhow::bail!("adb start failed");
    }

    let pid = adb_get_pid(&adb_path, &selected_device, &package_name).await?;

    if pid != 0 {
        match cmd {
            "i" | "r" => {
                tracing::info!("starting logcat ...");
                // os.system(G_ADB + " -s " + G_DEVICE + ' logcat -P "" ')
                // ps_cmd = (
                //     G_ADB + " -s " + G_DEVICE + " logcat | grep --color=auto " + str(pid)
                // )
                // print(ps_cmd)
                // os.system(ps_cmd)

                let mut cmd = Command::new(adb_path);
                cmd.args(["-s", &selected_device, "logcat"]);

                // Specify that we want the command's standard output piped back to us.
                // By default, standard input/output/error will be inherited from the
                // current process (for example, this means that standard input will
                // come from the keyboard and standard output/error will go directly to
                // the terminal if this process is invoked from the command line).
                cmd.stdout(Stdio::piped());

                let mut child = cmd.spawn().expect("failed to spawn command");

                let stdout = child
                    .stdout
                    .take()
                    .expect("child did not have a handle to stdout");

                let mut reader = BufReader::new(stdout);

                // Ensure the child process is spawned in the runtime so it can
                // make progress on its own while we await for any output.
                let join = tokio::spawn(async move {
                    let result = child
                        .wait_with_output()
                        .await
                        .expect("child process encountered an error");

                    tracing::info!("logcat process finished {}", result.status);
                });

                let mut buf = vec![];
                let process_pid_str = format!(" {} ", pid);
                while let Ok(_) = reader.read_until(b'\n', &mut buf).await {
                    if buf.is_empty() {
                        break;
                    }
                    let line = String::from_utf8_lossy(&buf);
                    if line.contains(&process_pid_str) {
                        tracing::info!("[log] {}", line.trim());
                    }
                    buf.clear();
                }

                join.await?;
            }

            "mem" => loop {
                let dumpsys_meminfo = run_cmd(
                    "dumpsys meminfo",
                    adb_path.to_str().unwrap(),
                    [
                        "-s",
                        &selected_device,
                        "shell",
                        "dumpsys",
                        "meminfo",
                        &pid.to_string(),
                    ],
                    true,
                )
                .await?;
                if !dumpsys_meminfo.0.success() {
                    anyhow::bail!("adb dumpsys meminfo failed");
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            },

            "cpu" => loop {
                let dumpsys_cpuinfo = run_cmd(
                    "dumpsys cpuinfo",
                    adb_path.to_str().unwrap(),
                    [
                        "-s",
                        &selected_device,
                        "shell",
                        "dumpsys",
                        "cpuinfo",
                        "|",
                        "grep",
                        &package_name,
                    ],
                    true,
                )
                .await?;
                if !dumpsys_cpuinfo.0.success() {
                    anyhow::bail!("adb dumpsys cpuinfo failed");
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            },

            _ => {
                anyhow::bail!("unknown command");
            }
        }
    } else {
        anyhow::bail!("get pid failed");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match run_adb_log().await {
        Ok(_) => {
            println!("Done!");
        }
        Err(e) => {
            tracing::error!("adblog error: {:?}", e);
            eprintln!("adblog error: {:?}", e);
        }
    }
}
