import os
import shutil
import sys
from pathlib import Path

def copy_if_newer(src, dest):
    """Copy a file if the source is newer than the destination or if the destination doesn't exist."""
    if not os.path.exists(dest) or os.path.getmtime(src) > os.path.getmtime(dest):
        print(f"Copying {src} to {dest}")
        shutil.copy2(src, dest)

def copy_csharp(mode, destination):
    """Copy C# related files."""
    rust_lib_path = script_dir / f"../target/{mode}/"

    for file in ['procmon_csharp.dll', 'procmon_csharp.pdb', 'procmon_csharp.dll.exp', 'procmon_csharp.dll.lib']:
        src = rust_lib_path / file
        dst = destination / file
        copy_if_newer(src, dst)

def copy_driver(mode, destination):
    """Copy driver related files."""
    if mode == "debug":
        mode = "dev-abort"
    else:
        raise Exception("Not supported, for now")

    rust_lib_path = script_dir / f"../target/{mode}/procmon_package"

    for file in ['procmon.cat', 'procmon.sys', 'procmon.inf', 'procmon.pdb']:
        src = rust_lib_path / file
        dst = destination / file
        copy_if_newer(src, dst)

def copy_ui(mode, destination):
    """Copy UI related files."""
    rust_lib_path = script_dir / f"../ProcmonUI/ProcmonUI/bin/{mode}/net8.0-windows/"

    for file in ['ProcmonUI.exe', 'ProcmonUI.pdb']:
        src = rust_lib_path / file
        dst = destination / file
        copy_if_newer(src, dst)

def main():
    # Parse command-line arguments
    release_arg = "--release" in sys.argv[1:]

    # Determine the build directory based on the release mode
    build_mode = "release" if release_arg else "debug"

    # Construct the directory path
    dist_dir = script_dir / f"../bin/{build_mode}"

    # Try to create the directory if it doesn't exist
    try:
        dist_dir.mkdir(parents=True, exist_ok=True)
        print(f"Directory created: {dist_dir}")
    except Exception as e:
        print(f"Failed to create directory: {e}")

    # Copy files
    copy_csharp(build_mode, dist_dir)
    copy_driver(build_mode, dist_dir)
    copy_ui(build_mode, dist_dir)

if __name__ == "__main__":
    # Get the directory where the script is located
    script_dir = Path(__file__).resolve().parent
    main()