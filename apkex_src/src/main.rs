use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use utils_lib::{assets_path, init_log, resolve_cygpath, run_cmd};

async fn unpack(apktool_path: &str, src_name: &Path) -> Result<String> {
    let des = src_name.with_extension("");
    if des.is_file() {
        fs::remove_file(&des).unwrap();
    }
    if des.is_dir() {
        fs::remove_dir_all(&des).unwrap();
    }

    let output = run_cmd(
        "unpack",
        "java",
        &[
            "-jar",
            "-Xms512m",
            "-Xmx1024m",
            apktool_path,
            "--only-main-classes",
            "d",
            "-f",
            src_name.to_str().unwrap(),
            "-o",
            des.to_str().unwrap(),
        ],
        false,
    )
    .await?;

    if !output.0.success() {
        anyhow::bail!("Failed to unpack apk");
    }

    Ok(des.to_str().unwrap().to_string())
}

async fn pack(apktool_path: &str, src_path: &Path) -> Result<String> {
    let des_path_str = format!("{}.repacked.apk", src_path.to_string_lossy());
    let des_path = Path::new(&des_path_str);
    if des_path.is_file() {
        fs::remove_file(des_path).unwrap();
    }

    let output = run_cmd(
        "pack",
        "java",
        &[
            "-jar",
            "-Xms512m",
            "-Xmx1024m",
            apktool_path,
            "--only-main-classes",
            "b",
            "-f",
            src_path.to_str().unwrap(),
            "-o",
            &des_path_str,
        ],
        false,
    )
    .await?;

    if !output.0.success() {
        anyhow::bail!("Failed to pack apk");
    }

    Ok(des_path_str.to_string())
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct KeyConfig {
    pub key_path: String,
    pub alias_name: String,
    pub store_pwd: String,
    pub key_pwd: String,
}

fn read_config(conf_file_path: &Path) -> Result<KeyConfig> {
    println!("read config: {}", conf_file_path.display());
    Ok(if conf_file_path.is_file() {
        let file_content = fs::read_to_string(conf_file_path)?;
        serde_json::from_str(&file_content)?
    } else {
        KeyConfig::default()
    })
}

async fn sign(apksigner_path: &str, apk_path: &str, conf: &KeyConfig) -> Result<()> {
    let output = run_cmd(
        "sign",
        "java",
        &[
            "-jar",
            apksigner_path,
            "--allowResign",
            "--overwrite",
            "-ks",
            &conf.key_path,
            "--ksPass",
            &conf.store_pwd,
            "--ksAlias",
            &conf.alias_name,
            "--ksKeyPass",
            &conf.key_pwd,
            "-a",
            apk_path,
        ],
        false,
    )
    .await?;

    if !output.0.success() {
        anyhow::bail!("Failed to sign apk");
    }

    Ok(())
}

/// Apkex: Apk tool command line tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Command [u: unpack; p: pack;]
    #[arg(short, long)]
    cmd: String,

    /// File Or Folder Path
    #[arg(short, long)]
    file_or_folder_path: String,

    /// Keystore config file path
    #[arg(short, long, default_value = "")]
    key_config: String,
}

async fn run_apktool_ex() -> Result<()> {
    let args: Args = Args::parse();
    let cmd = args.cmd.trim();

    let file_or_folder_path = resolve_cygpath(&args.file_or_folder_path).await?;
    let file_or_folder_path = std::path::absolute(Path::new(&file_or_folder_path))?;
    if !file_or_folder_path.exists() {
        anyhow::bail!(
            "file_or_folder_path not exists: {}",
            file_or_folder_path.display()
        );
    }

    let file_or_folder_parent = file_or_folder_path.parent().unwrap();
    let _guards = init_log("apkex", file_or_folder_parent);

    let assets_path = assets_path().await;
    let assets_path = Path::new(&assets_path);
    if !assets_path.exists() {
        anyhow::bail!("assets_path not exists: {}", assets_path.display());
    }

    tracing::info!("assets_path: {}", assets_path.display());

    let apktool_path = assets_path.join("apktool_2_8_1.jar");
    if !apktool_path.exists() {
        anyhow::bail!("apktool_path not exists: {}", apktool_path.display());
    }
    let apktool_path = apktool_path.to_str().unwrap();
    tracing::info!("apktool_path: {}", apktool_path);

    let apksigner_path = assets_path.join("uber-apk-signer-1.3.0.jar");
    if !apksigner_path.exists() {
        anyhow::bail!("apksigner_path not exists: {}", apksigner_path.display());
    }
    let apksigner_path = apksigner_path.to_str().unwrap();
    tracing::info!("apksigner_path: {}", apksigner_path);

    match cmd {
        "u" => {
            unpack(apktool_path, &file_or_folder_path).await?;
        }
        "p" => {
            let key_config_path = if args.key_config.is_empty() {
                tracing::info!("key_config is empty, use default");
                assets_path.join("default_key_config.json")
            } else {
                let key_config_path = resolve_cygpath(&args.key_config).await?;
                std::path::absolute(Path::new(&key_config_path))?
            };
            tracing::info!("key_config_path: {}", key_config_path.display());
            let key_config_folder = key_config_path.parent().context(format!(
                "key_config_path parent not exists: {}",
                key_config_path.display()
            ))?;

            let mut conf = read_config(&key_config_path)?;
            let key_path = key_config_folder.join(&conf.key_path);
            if !key_path.is_file() {
                anyhow::bail!("key_path not exists: {}", key_path.display());
            }
            conf.key_path = key_path.to_str().unwrap().to_string();

            let packed = pack(apktool_path, &file_or_folder_path).await?;
            tracing::info!("\n\npacked to: {}", packed);
            sign(apksigner_path, &packed, &conf).await?;
            tracing::info!("\n\nsigned to: {}", packed);
        }
        _ => {
            anyhow::bail!("unknown cmd: {}", cmd);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match run_apktool_ex().await {
        Ok(_) => {
            println!("Done");
        }
        Err(e) => {
            tracing::error!("Failed: {:?}", e);
            println!("Failed: {:?}", e);
        }
    }
}
