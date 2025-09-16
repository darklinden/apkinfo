use anyhow::Result;
use std::fs;
use std::io::stdout;
use std::path::Path;
use std::process::Stdio;
use time::{format_description, UtcOffset};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, OnceCell};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::{self, time::OffsetTime};
use tracing_subscriber::layer::SubscriberExt;

pub fn init_log(prefix: &str, project_path: &Path) -> (WorkerGuard, WorkerGuard) {
    // std::env::set_var("RUST_BACKTRACE", "1");
    let time_zone_offset = UtcOffset::from_hms(8, 0, 0).expect("should get UTC+8 offset!");

    let format = format_description::parse("[year][month][day]_[hour][minute][second]").unwrap();
    let now = time::OffsetDateTime::now_local()
        .unwrap_or(time::OffsetDateTime::now_utc().to_offset(time_zone_offset))
        .format(&format)
        .unwrap();
    let log_file_name = format!("{}_{}.log", prefix, now);
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

async fn has_cygpath() -> &'static bool {
    static HAS_CYGPATH: OnceCell<bool> = OnceCell::const_new();
    HAS_CYGPATH
        .get_or_init(async || {
            let output = run_cmd("cygpath_check", "which", &["cygpath"], false).await;
            match output {
                Err(_) => false,
                Ok(o) => o.0.success(),
            }
        })
        .await
}

pub async fn resolve_cygpath(path: &str) -> Result<String> {
    if !*has_cygpath().await {
        return Ok(path.to_string());
    }
    let result = run_cmd("cygpath-convert", "cygpath", &["-wa", path], true).await?;
    Ok(result.1.join("").trim().to_string())
}

pub async fn assets_path() -> &'static str {
    static ASSETS_PATH: OnceCell<String> = OnceCell::const_new();
    ASSETS_PATH
        .get_or_init(async || {
            let base_folder = match std::env::var("CARGO_MANIFEST_DIR") {
                Ok(script_folder) => script_folder,
                Err(_) => {
                    let exe_path = std::env::current_exe().unwrap_or_default();
                    exe_path
                        .parent()
                        .map_or_else(|| "".to_string(), |p| p.to_string_lossy().to_string())
                }
            };
            let base_folder_path = Path::new(&base_folder);
            if !base_folder_path.exists() {
                tracing::info!("script_folder not exists: {}", base_folder_path.display());
            }

            let assets_path = base_folder_path.join("assets");
            if !assets_path.is_dir() {
                tracing::info!("assets_path not exists: {}", assets_path.display());
            }
            assets_path.to_string_lossy().to_string()
        })
        .await
}

pub async fn run_cmd(
    work: &str,
    program: &str,
    args: &[&str],
    require_output: bool,
) -> Result<(std::process::ExitStatus, Vec<String>)> {
    tracing::info!("{} start: {} {}", work, program, args.join(" "));

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

    tracing::info!("{} finished code {}", work, output_result.as_ref().unwrap());
    Ok((output_result.unwrap(), stdout))
}

pub fn extract_single_file(src_zip: &Path, sub_file: &str, des_file: &Path) -> Result<()> {
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
            fs::set_permissions(des_file, fs::Permissions::from_mode(mode))?;
        }
    }

    Ok(())
}
