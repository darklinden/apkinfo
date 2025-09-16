use anyhow::Result;
use clap::Parser;
use std::fs;
use utils_lib::{assets_path, init_log, resolve_cygpath, run_cmd};

/// ApkInfo: A tool to extract information from APK files
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Apk File Path
    #[arg(short, long)]
    apk_path: String,
}

async fn run_apk_info() -> Result<()> {
    let _guards = init_log("apkinfo", std::path::Path::new("."));

    let args = Args::parse();

    let apk_path = resolve_cygpath(&args.apk_path).await?;
    tracing::info!("apk_path: {}", apk_path);
    let apk_path = std::path::absolute(&apk_path)?;
    if !apk_path.is_file() {
        tracing::info!("apk file not found");
        return Ok(());
    }

    let assets_path = assets_path().await;
    tracing::info!("assets_path: {}", assets_path);
    let assets_path = std::path::absolute(assets_path)?;
    if !assets_path.is_dir() {
        anyhow::bail!("assets_path not found");
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
    let android_home = resolve_cygpath(&android_home).await?;
    let android_home = std::path::absolute(&android_home)?;
    if !android_home.is_dir() {
        anyhow::bail!("android_home not found");
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
    let latest_build_tools = build_tools.iter().max().unwrap();
    tracing::info!("latest_build_tools: {}", latest_build_tools.display());

    let aapt2_path = fs::read_dir(latest_build_tools)?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let file_path = entry.path();
                if file_path.is_file()
                    && file_path
                        .file_name()
                        .is_some_and(|f| f.to_str().unwrap().starts_with("aapt2"))
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
        .unwrap();
    tracing::info!("aapt2_path: {}", aapt2_path.display());

    let apksigner_path = fs::read_dir(latest_build_tools.join("lib"))?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let file_path = entry.path();
                if file_path.is_file()
                    && file_path
                        .file_name()
                        .is_some_and(|f| f.to_str().unwrap().starts_with("apksigner"))
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
        .unwrap();
    tracing::info!("apksigner_path: {}", apksigner_path.display());

    let vasdolly_path = assets_path.join("VasDolly_3_0_4.jar");
    if !vasdolly_path.is_file() {
        tracing::info!("vasdolly not found");
        return Ok(());
    }

    tracing::info!("aapt2 apk info ...");
    let info_out = run_cmd(
        "aapt2",
        aapt2_path.to_str().unwrap(),
        &["dump", "badging", apk_path.to_str().unwrap()],
        false,
    )
    .await?;

    if !info_out.0.success() {
        anyhow::bail!("extract apk failed");
    }

    let apk_results = run_cmd(
        "apksigner",
        "java",
        &[
            "-jar",
            apksigner_path.to_str().unwrap(),
            "verify",
            "--print-certs",
            "-v",
            apk_path.to_str().unwrap(),
        ],
        false,
    )
    .await?;
    if !apk_results.0.success() {
        anyhow::bail!("verify apk failed");
    }

    let channel = run_cmd(
        "vasdolly",
        "java",
        &[
            "-jar",
            vasdolly_path.to_str().unwrap(),
            "get",
            "-c",
            apk_path.to_str().unwrap(),
        ],
        false,
    )
    .await?;
    if !channel.0.success() {
        anyhow::bail!("get channel failed");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match run_apk_info().await {
        Ok(_) => {}
        Err(e) => {
            tracing::info!("{}", e);
        }
    }
}
