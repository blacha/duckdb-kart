import os
import shutil
import subprocess
import sys

def pad32(b: bytes) -> bytes:
    return b + b"\x00" * (32 - len(b))

def make_footer() -> bytes:
    chunk0 = b"\x00" * 32
    chunk1 = b"\x00" * 32
    chunk2 = b"\x00" * 32
    chunk3 = pad32(b"C_STRUCT")
    chunk4 = pad32(b"v0.1.0")
    chunk5 = pad32(b"v1.2.0")
    chunk6 = pad32(b"osx_arm64")
    chunk7 = pad32(b"4")
    signature = b"\x00" * 256
    return chunk0 + chunk1 + chunk2 + chunk3 + chunk4 + chunk5 + chunk6 + chunk7 + signature

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    print("Building release cdylib...")
    subprocess.check_call(["cargo", "build", "--release", "--offline"])

    src_dylib = os.path.join(script_dir, "target", "release", "libkart.dylib")
    dest_ext = os.path.join(script_dir, "kart.duckdb_extension")

    print(f"Copying {src_dylib} -> {dest_ext}...")
    shutil.copyfile(src_dylib, dest_ext)

    print("Appending DuckDB extension metadata footer...")
    footer = make_footer()
    assert len(footer) == 512
    with open(dest_ext, "ab") as f:
        f.write(footer)

    print(f"Successfully packaged {dest_ext} ({os.path.getsize(dest_ext)} bytes)")

if __name__ == "__main__":
    main()
