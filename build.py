#!/usr/bin/env python3
"""
跨平台构建脚本
仅使用 Python 标准库，无需 pip install 任何依赖

前置要求（需用户自行安装）：
  - Python 3.7+
  - Docker
  - rclone
"""

import subprocess
import sys
from pathlib import Path

# 配置
DIST_DIR = Path("./dist")
IMAGE_NAME = "static-server"
REMOTE_PATH = "tos:muelsyse/static-server/static-server"


def run(cmd: str, capture_output: bool = False) -> subprocess.CompletedProcess:
    """
    执行 shell 命令，失败时自动退出
    
    Args:
        cmd: 要执行的命令
        capture_output: 是否捕获输出
        
    Returns:
        CompletedProcess 对象
    """
    print(f">>> {cmd}", file=sys.stderr)
    
    result = subprocess.run(
        cmd, 
        shell=True, 
        capture_output=capture_output, 
        text=True,
        encoding='utf-8'
    )
    
    if result.returncode != 0:
        print(f"错误: 命令失败 (exit {result.returncode})", file=sys.stderr)
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        sys.exit(result.returncode)
    
    return result


def build():
    """执行 Docker 构建并提取产物"""
    # 1. 创建 dist 目录（如果不存在）
    DIST_DIR.mkdir(parents=True, exist_ok=True)
    print(f"确保目录存在: {DIST_DIR}")
    
    # 2. 构建 Docker 镜像
    print("\n构建 Docker 镜像...")
    run(f"docker build -t {IMAGE_NAME} .")
    
    # 3. 创建临时容器并获取 ID
    print("\n提取构建产物...")
    result = run(f"docker create {IMAGE_NAME}", capture_output=True)
    container = result.stdout.strip()
    
    try:
        # 4. 复制构建产物
        source = f"{container}:/root/static-server"
        dest = DIST_DIR / "static-server"
        run(f'docker cp "{source}" "{dest}"')
        print(f"已提取到: {dest}")
    finally:
        # 5. 清理临时容器（无论成功与否都执行）
        print(f"清理临时容器: {container[:12]}...")
        run(f'docker rm "{container}"')


def upload():
    """上传构建产物到远程存储"""
    print(f"\n上传到远程存储...")
    local = DIST_DIR / "static-server"
    run(f'rclone copyto "{local}" {REMOTE_PATH}')
    print(f"上传完成: {REMOTE_PATH}")


def main():
    """主入口"""
    print("=" * 50)
    print("开始构建流程")
    print("=" * 50)
    
    # 执行构建和上传
    build()
    upload()
    
    print("\n" + "=" * 50)
    print(f"全部完成！构建产物: {DIST_DIR}/static-server")
    print("=" * 50)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n用户中断", file=sys.stderr)
        sys.exit(130)
