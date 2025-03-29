import os
import shutil
import sys
from pathlib import Path

def copy_if_newer(src, dest):
    """Copy a file if the source is newer than the destination or if the destination doesn't exist."""
    if not os.path.exists(dest) or os.path.getmtime(src) > os.path.getmtime(dest):
        print(f"Copying {src} to {dest}")
        shutil.copy2(src, dest)

def copy_bindings():
    """Copy UI related bindings."""

    #d:\Programare\Rust\Github\procmon-rust\crates\procmon-csharp\bindings\dotnet\
    rust_bindings_location = script_dir / f"../crates/procmon-csharp/bindings/dotnet"
    procmon_ui_path = script_dir / f"../ProcmonUI/ProcmonUI"

    for file in ['procmon.cs']:
        src = rust_bindings_location / file
        dst = procmon_ui_path / file
        copy_if_newer(src, dst)

def copy_dll(mode):
    """Copy driver related files."""
    if mode == "debug":
        mode = "dev-abort"
    else:
        raise Exception("Not supported, for now")

    ui_dest = Path(f"./../ProcmonUI/ProcmonUI/bin/{mode}/net8.0-windows")
    rust_lib_path = script_dir / f"../target/{mode}/procmon_package"

    for file in ['prcmon_csharp.dll', 'procmon_csharp.pdb']:
        src = rust_lib_path / file
        dst = ui_dest / file
        copy_if_newer(src, dst)

def main():
    # Parse command-line arguments
    release_arg = "--release" in sys.argv[1:]

    # Determine the build directory based on the release mode
    build_mode = "release" if release_arg else "debug"

    copy_bindings()
    copy_dll(build_mode)

if __name__ == "__main__":
    # Get the directory where the script is located
    script_dir = Path(__file__).resolve().parent
    main()