//! 运行方式: cargo run --bin push

use anyhow::{bail, Result};
use std::io::{self, Write};
use std::process::{Command, Stdio};

const DIST_DIR: &str = "./dist";
const IMAGE_NAME: &str = "static-server";
const REMOTE_PATH: &str = "tos:muelsyse/static-server/static-server";

fn main() -> Result<()> {
    // 创建 dist 目录
    std::fs::create_dir_all(DIST_DIR)?;

    println!("📦 开始构建...");
    io::stdout().flush()?;

    // Docker 构建 (继承 stdout/stderr，实时输出)
    let status = Command::new("docker")
        .args(["build", "-t", IMAGE_NAME, "."])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!("Docker 构建失败");
    }

    // 创建临时容器 (仅捕获 stdout 获取容器 ID)
    let output = Command::new("docker")
        .args(["create", IMAGE_NAME])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    let container = String::from_utf8(output.stdout)?.trim().to_string();

    // 复制构建产物
    let status = Command::new("docker")
        .args([
            "cp",
            &format!("{}:/root/static-server", container),
            &format!("{}/static-server", DIST_DIR),
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    // 清理容器
    let _ = Command::new("docker")
        .args(["rm", &container])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    if !status.success() {
        bail!("复制构建产物失败");
    }

    println!("✅ 构建完成: {}/static-server", DIST_DIR);
    io::stdout().flush()?;

    println!("☁️  开始上传...");
    io::stdout().flush()?;

    // rclone 上传 (继承 stdout/stderr，实时输出)
    let status = Command::new("rclone")
        .args([
            "copyto",
            &format!("{}/static-server", DIST_DIR),
            REMOTE_PATH,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!("rclone 上传失败");
    }

    println!("✅ 上传完成: {} -> {}", DIST_DIR, REMOTE_PATH);
    Ok(())
}
