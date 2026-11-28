#!/usr/bin/env python3
"""
Fetches verified 2026 Java Runtimes (JRE 8, 17, 21, 25) and modern Android GL/Vulkan
renderers (MobileGlues, NG-GL4ES Krypton Wrapper, Kopper Zink, OpenAL Soft) directly
from ZalithLauncher/ZalithLauncher2 using fast Git sparse checkout, then stages only
the requested architecture's binaries & binpacks into HuTao Launcher Rust.
"""
import argparse
import os
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ZALITH_REPO = "https://github.com/ZalithLauncher/ZalithLauncher2.git"
CACHE_DIR = os.path.abspath(".zalith2_cache")
ASSETS_COMPONENTS = os.path.abspath("app_pojavlauncher/src/main/assets/components")
JNILIBS_DIR = os.path.abspath("app_pojavlauncher/src/main/jniLibs")
STANDALONE_DIR = os.path.abspath("out/standalone_jres")

ARCH_TO_ABI = {
    "arm64": "arm64-v8a",
    "arm": "armeabi-v7a",
    "x86_64": "x86_64",
    "x86": "x86",
}

JRE_MAP = [
    ("jre-8", "jre", "8"),
    ("jre-17", "jre-new", "17"),
    ("jre-21", "jre-21", "21"),
    ("jre-25", "jre-25", "25"),
]

RENDERER_AARS = [
    "MobileGlues-release.aar",
    "NG-GL4ES-release.aar",
    "kopper-zink-release.aar",
    "openal-soft-release.aar",
]


def run(cmd, cwd=None, check=True):
    print("+", " ".join(cmd), flush=True)
    return subprocess.run(cmd, cwd=cwd, check=check)


def is_lfs_pointer(path: str) -> bool:
    try:
        if os.path.getsize(path) < 1024:
            with open(path, "rb") as f:
                return f.read(40).startswith(b"version https://git-lfs.github.com")
    except OSError:
        pass
    return False


def ensure_zalith_sparse_checkout():
    if os.path.isdir(os.path.join(CACHE_DIR, ".git")):
        print("Reusing existing ZalithLauncher2 sparse cache.")
        return

    if os.path.exists(CACHE_DIR):
        shutil.rmtree(CACHE_DIR)

    run([
        "git", "clone",
        "--depth=1",
        "--filter=blob:none",
        "--sparse",
        ZALITH_REPO,
        CACHE_DIR,
    ])
    run([
        "git", "sparse-checkout", "set", "--no-cone",
        "/ZalithLauncher/src/main/assets/runtimes/jre-8",
        "/ZalithLauncher/src/main/assets/runtimes/jre-17",
        "/ZalithLauncher/src/main/assets/runtimes/jre-21",
        "/ZalithLauncher/src/main/assets/runtimes/jre-25",
        "/ZalithLauncher/libs/MobileGlues-release.aar",
        "/ZalithLauncher/libs/NG-GL4ES-release.aar",
        "/ZalithLauncher/libs/kopper-zink-release.aar",
        "/ZalithLauncher/libs/openal-soft-release.aar",
    ], cwd=CACHE_DIR)

    sample = os.path.join(CACHE_DIR, "ZalithLauncher/src/main/assets/runtimes/jre-17/universal.tar.xz")
    if os.path.isfile(sample) and is_lfs_pointer(sample):
        run(["git", "lfs", "pull"], cwd=CACHE_DIR, check=False)


def stage_jres(arch: str):
    runtimes_src = os.path.join(CACHE_DIR, "ZalithLauncher/src/main/assets/runtimes")
    target_arches = list(ARCH_TO_ABI.keys()) if arch == "all" else [arch]

    for src_name, dst_comp, major in JRE_MAP:
        src_dir = os.path.join(runtimes_src, src_name)
        dst_dir = os.path.join(ASSETS_COMPONENTS, dst_comp)
        if os.path.exists(dst_dir):
            shutil.rmtree(dst_dir)

        if not os.path.isdir(src_dir):
            print(f"[JRE {major}] Warning: {src_dir} missing in ZalithLauncher2 checkout")
            continue

        os.makedirs(dst_dir, exist_ok=True)
        for fixed in ("version", "universal.tar.xz"):
            sp = os.path.join(src_dir, fixed)
            if os.path.isfile(sp):
                shutil.copy2(sp, os.path.join(dst_dir, fixed))

        copied = []
        for a in target_arches:
            bp = os.path.join(src_dir, f"bin-{a}.tar.xz")
            if os.path.isfile(bp):
                shutil.copy2(bp, os.path.join(dst_dir, f"bin-{a}.tar.xz"))
                copied.append(a)

        if not copied:
            shutil.rmtree(dst_dir)
            print(f"[JRE {major}] No binpack for arch={arch}; skipped.")
        else:
            ver = open(os.path.join(dst_dir, "version"), encoding="utf-8").read().strip()
            print(f"[JRE {major}] Staged v{ver} into components/{dst_comp} for arches: {copied}")


def stage_renderers(arch: str):
    libs_src = os.path.join(CACHE_DIR, "ZalithLauncher/libs")
    target_abis = set(ARCH_TO_ABI.values()) if arch == "all" else {ARCH_TO_ABI[arch]}

    for aar_name in RENDERER_AARS:
        aar_path = os.path.join(libs_src, aar_name)
        if not os.path.isfile(aar_path):
            print(f"[Renderer] Warning: {aar_name} not found in ZalithLauncher2 libs")
            continue

        extracted = 0
        with zipfile.ZipFile(aar_path, "r") as zf:
            for member in zf.namelist():
                if not member.startswith("jni/") or not member.endswith(".so"):
                    continue
                parts = member.split("/")
                if len(parts) != 3:
                    continue
                abi, so_name = parts[1], parts[2]
                if abi not in target_abis:
                    continue
                out_dir = os.path.join(JNILIBS_DIR, abi)
                os.makedirs(out_dir, exist_ok=True)
                out_path = os.path.join(out_dir, so_name)
                with zf.open(member) as src, open(out_path, "wb") as dst:
                    shutil.copyfileobj(src, dst)
                extracted += 1
        print(f"[Renderer] Extracted {extracted} .so file(s) from {aar_name} for {sorted(target_abis)}")


def build_standalone_tarballs():
    runtimes_src = os.path.join(CACHE_DIR, "ZalithLauncher/src/main/assets/runtimes")
    os.makedirs(STANDALONE_DIR, exist_ok=True)

    for src_name, _, major in JRE_MAP:
        src_dir = os.path.join(runtimes_src, src_name)
        uni_p = os.path.join(src_dir, "universal.tar.xz")
        bin_p = os.path.join(src_dir, "bin-arm64.tar.xz")
        if not (os.path.isfile(uni_p) and os.path.isfile(bin_p)):
            continue
        ver = open(os.path.join(src_dir, "version"), encoding="utf-8").read().strip()
        out_p = os.path.join(STANDALONE_DIR, f"HuTao-JRE{major}-arm64-{ver}.tar.xz")
        print(f"[Standalone] Merging {out_p} ...")
        with tempfile.TemporaryDirectory() as td:
            for archive in (uni_p, bin_p):
                with tarfile.open(archive, "r:xz") as tf:
                    try:
                        tf.extractall(td, filter="tar")
                    except TypeError:
                        tf.extractall(td)
            with tarfile.open(out_p, "w:xz") as out_tar:
                for entry in sorted(os.listdir(td)):
                    out_tar.add(os.path.join(td, entry), arcname=entry)
        print(f"[Standalone] Created {os.path.basename(out_p)} ({os.path.getsize(out_p):,} B)")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--arch", choices=["arm64", "arm", "x86_64", "x86", "all"], required=True)
    parser.add_argument("--standalone", action="store_true", help="Also build standalone arm64 .tar.xz runtimes")
    args = parser.parse_args()

    ensure_zalith_sparse_checkout()
    stage_jres(args.arch)
    stage_renderers(args.arch)
    if args.standalone:
        build_standalone_tarballs()


if __name__ == "__main__":
    main()
