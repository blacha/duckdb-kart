import argparse
import os
import platform
import shutil
import subprocess
import sys

def pad32(b: bytes) -> bytes:
    return b + b"\x00" * (32 - len(b))

def detect_platform() -> str:
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        if machine in ("arm64", "aarch64"):
            return "osx_arm64"
        return "osx_amd64"
    elif system == "linux":
        if machine in ("arm64", "aarch64"):
            return "linux_arm64"
        return "linux_amd64"
    elif system == "windows":
        return "windows_amd64"
    return f"{system}_{machine}"

def make_footer(platform_tag: str, duckdb_version: str = "v1.2.0", ext_version: str = "v0.1.0") -> bytes:
    chunk0 = b"\x00" * 32
    chunk1 = b"\x00" * 32
    chunk2 = b"\x00" * 32
    chunk3 = pad32(b"C_STRUCT")
    chunk4 = pad32(ext_version.encode("ascii"))
    chunk5 = pad32(duckdb_version.encode("ascii"))
    chunk6 = pad32(platform_tag.encode("ascii"))
    chunk7 = pad32(b"4")
    signature = b"\x00" * 256
    return chunk0 + chunk1 + chunk2 + chunk3 + chunk4 + chunk5 + chunk6 + chunk7 + signature

def main():
    parser = argparse.ArgumentParser(description="Package DuckDB extension")
    parser.add_argument("--platform", default=None, help="DuckDB platform tag (e.g. osx_arm64, linux_amd64)")
    parser.add_argument("--duckdb-version", default="v1.2.0", help="Target DuckDB version")
    parser.add_argument("--ext-version", default="v0.1.0", help="Extension version")
    parser.add_argument("--target", default=None, help="Cargo target triple")
    parser.add_argument("--output", default=None, help="Output extension path")
    parser.add_argument("--no-build", action="store_true", help="Skip running cargo build")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    plat = args.platform or detect_platform()
    print(f"Packaging for platform: {plat}, DuckDB version: {args.duckdb_version}")

    if not args.no_build:
        cmd = ["cargo", "build", "--release"]
        if args.target:
            cmd.extend(["--target", args.target])
        print(f"Running {' '.join(cmd)}...")
        subprocess.check_call(cmd)

    target_dir = os.path.join(script_dir, "target")
    if args.target:
        release_dir = os.path.join(target_dir, args.target, "release")
    else:
        release_dir = os.path.join(target_dir, "release")

    candidates = [
        os.path.join(release_dir, "libkart.dylib"),
        os.path.join(release_dir, "libkart.so"),
        os.path.join(release_dir, "kart.dll"),
        os.path.join(release_dir, "libkart.dll"),
    ]
    src_lib = next((p for p in candidates if os.path.exists(p)), None)
    if not src_lib:
        sys.exit(f"Error: Could not find compiled library in {release_dir}")

    dest_ext = args.output or os.path.join(script_dir, "kart.duckdb_extension")

    print(f"Copying {src_lib} -> {dest_ext}...")
    shutil.copyfile(src_lib, dest_ext)

    print("Appending DuckDB extension metadata footer...")
    footer = make_footer(plat, args.duckdb_version, args.ext_version)
    assert len(footer) == 512
    with open(dest_ext, "ab") as f:
        f.write(footer)

    print(f"Successfully packaged {dest_ext} ({os.path.getsize(dest_ext)} bytes)")

if __name__ == "__main__":
    main()
