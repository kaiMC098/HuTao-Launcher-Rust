#!/usr/bin/env python3
import argparse
import json
import os
import re
import shutil
import tarfile
import tempfile
import urllib.request
import zipfile

REPO_RELEASES_API = "https://api.github.com/repos/PojavLauncherTeam/android-openjdk-build-multiarch/releases?per_page=100"

STAGE_ROOT = os.path.abspath("jre_stage")
ASSETS_COMPONENTS = os.path.abspath("app_pojavlauncher/src/main/assets/components")

ABI_TO_POJAV_ARCH = {
    "arm64-v8a": "arm64",
    "armeabi-v7a": "arm",
    "x86_64": "x86_64",
    "x86": "x86",
}

JRE_SPECS = [
    ("8", "jre"),
    ("17", "jre-new"),
    ("21", "jre-21"),
]


def http_get_json(url: str):
    headers = {"User-Agent": "HuTao-Launcher-CI", "Accept": "application/vnd.github+json"}
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.loads(resp.read().decode("utf-8"))


def download_file(url: str, dest: str):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    headers = {"User-Agent": "HuTao-Launcher-CI"}
    token = os.environ.get("GITHUB_TOKEN")
    if token and "api.github.com" in url:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=180) as resp, open(dest, "wb") as out:
        shutil.copyfileobj(resp, out)


def make_stub_universal_tar_xz(dest: str, label: str):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    with tempfile.TemporaryDirectory() as td:
        marker = os.path.join(td, ".hutao_jre_marker")
        with open(marker, "w", encoding="utf-8") as f:
            f.write(label + "\n")
        with tarfile.open(dest, "w:xz") as tar:
            tar.add(marker, arcname=".hutao_jre_marker")


def detect_arch_from_name(name: str):
    low = name.lower()
    if "arm64" in low or "aarch64" in low:
        return "arm64"
    if "x86_64" in low or "amd64" in low:
        return "x86_64"
    if re.search(r"[-_](arm32|aarch32|arm)(?:[-_.]|$)", low):
        return "arm"
    if re.search(r"[-_](x86|i386|i686)(?:[-_.]|$)", low):
        return "x86"
    return None


def download_all_jres():
    print("Querying PojavLauncherTeam/android-openjdk-build-multiarch releases...")
    releases = http_get_json(REPO_RELEASES_API)

    all_dir = os.path.join(STAGE_ROOT, "all")
    standalone_dir = os.path.join(STAGE_ROOT, "standalone")
    os.makedirs(all_dir, exist_ok=True)
    os.makedirs(standalone_dir, exist_ok=True)

    for major, comp_name in JRE_SPECS:
        comp_out = os.path.join(all_dir, comp_name)
        os.makedirs(comp_out, exist_ok=True)

        # 1) Try finding an official Pojav binpack zip (e.g. jre17-pojav.zip)
        pojav_zip_asset = None
        pojav_tag = f"jre{major}-hutao"
        for rel in releases:
            if rel.get("draft"):
                continue
            for asset in rel.get("assets", []):
                aname = asset["name"].lower()
                if aname == f"jre{major}-pojav.zip" or re.match(rf"^jre{major}[-_].*pojav.*\.zip$", aname):
                    pojav_zip_asset = asset
                    pojav_tag = rel.get("tag_name") or pojav_tag
                    break
            if pojav_zip_asset:
                break

        found_arches = {}
        if pojav_zip_asset:
            print(f"[JRE {major}] Downloading binpack {pojav_zip_asset['name']} ({pojav_tag})...")
            zpath = os.path.join(STAGE_ROOT, pojav_zip_asset["name"])
            download_file(pojav_zip_asset["browser_download_url"], zpath)
            with zipfile.ZipFile(zpath, "r") as zf:
                for member in zf.namelist():
                    base = os.path.basename(member)
                    if not base:
                        continue
                    if base in ("version", "universal.tar.xz") or base.startswith("bin-"):
                        with zf.open(member) as src, open(os.path.join(comp_out, base), "wb") as dst:
                            shutil.copyfileobj(src, dst)
                        if base.startswith("bin-") and base.endswith(".tar.xz"):
                            arch_key = base[len("bin-"):-len(".tar.xz")]
                            found_arches[arch_key] = os.path.join(comp_out, base)
            os.remove(zpath)

        # 2) Also fetch standalone release tar.xz files for any missing arch & for standalone Runtime Artifact
        standalone_found = {}
        for rel in releases:
            if rel.get("draft"):
                continue
            tag = rel.get("tag_name") or f"jre{major}"
            for asset in rel.get("assets", []):
                aname = asset["name"]
                low = aname.lower()
                if not low.endswith(".tar.xz"):
                    continue
                if "debug" in low or "minimal" in low:
                    continue
                if not re.match(rf"^(jre|jdk|openjdk)[-_]?{major}[-_]", low):
                    continue
                arch = detect_arch_from_name(low)
                if arch and arch not in standalone_found:
                    standalone_found[arch] = (tag, aname, asset["browser_download_url"])
            if len(standalone_found) == 4:
                break

        for arch, (tag, aname, url) in sorted(standalone_found.items()):
            standalone_dest = os.path.join(standalone_dir, f"jre{major}-{arch}.tar.xz")
            print(f"[JRE {major}] Downloading standalone {aname} -> {os.path.basename(standalone_dest)}")
            download_file(url, standalone_dest)
            if arch not in found_arches:
                shutil.copy2(standalone_dest, os.path.join(comp_out, f"bin-{arch}.tar.xz"))
                found_arches[arch] = os.path.join(comp_out, f"bin-{arch}.tar.xz")
                pojav_tag = tag

        if not os.path.exists(os.path.join(comp_out, "universal.tar.xz")) and found_arches:
            make_stub_universal_tar_xz(os.path.join(comp_out, "universal.tar.xz"), f"jre{major}-{pojav_tag}")

        if not os.path.exists(os.path.join(comp_out, "version")) and found_arches:
            with open(os.path.join(comp_out, "version"), "w", encoding="utf-8") as vf:
                vf.write(pojav_tag)

        print(f"[JRE {major}] Ready arches: {sorted(found_arches.keys())}")


def stage_for_target(target: str):
    all_dir = os.path.join(STAGE_ROOT, "all")
    for _, comp_name in JRE_SPECS:
        dest_dir = os.path.join(ASSETS_COMPONENTS, comp_name)
        if os.path.exists(dest_dir):
            shutil.rmtree(dest_dir)
        src_dir = os.path.join(all_dir, comp_name)
        if not os.path.exists(src_dir):
            continue

        os.makedirs(dest_dir, exist_ok=True)
        for fixed in ("version", "universal.tar.xz"):
            sp = os.path.join(src_dir, fixed)
            if os.path.exists(sp):
                shutil.copy2(sp, os.path.join(dest_dir, fixed))

        if target == "all":
            arches = list(ABI_TO_POJAV_ARCH.values())
        else:
            arches = [ABI_TO_POJAV_ARCH[target]]

        copied = 0
        for arch in arches:
            bp = os.path.join(src_dir, f"bin-{arch}.tar.xz")
            if os.path.exists(bp):
                shutil.copy2(bp, os.path.join(dest_dir, f"bin-{arch}.tar.xz"))
                copied += 1

        if copied == 0:
            shutil.rmtree(dest_dir)
        else:
            print(f"[Stage:{target}] {comp_name}: bundled {copied} arch binpack(s)")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--download", action="store_true", help="Download JRE 8, 17, 21 multiarch packs")
    parser.add_argument("--stage", choices=["arm64-v8a", "armeabi-v7a", "x86_64", "x86", "all"], help="Stage JRE assets for APK build")
    args = parser.parse_args()

    if args.download:
        download_all_jres()
    if args.stage:
        stage_for_target(args.stage)


if __name__ == "__main__":
    main()
