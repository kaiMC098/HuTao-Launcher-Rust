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

REPO_RELEASES_BASE = "https://api.github.com/repos/PojavLauncherTeam/android-openjdk-build-multiarch/releases"
REPO_TAGS_BASE = "https://api.github.com/repos/PojavLauncherTeam/android-openjdk-build-multiarch/releases/tags"
ADOPTIUM_V3_BASE = "https://api.adoptium.net/v3/binary/latest"

STAGE_ROOT = os.path.abspath("jre_stage")
ASSETS_COMPONENTS = os.path.abspath("app_pojavlauncher/src/main/assets/components")

ABI_TO_POJAV_ARCH = {
    "arm64-v8a": "arm64",
    "armeabi-v7a": "arm",
    "x86_64": "x86_64",
    "x86": "x86",
}

POJAV_TO_ADOPTIUM_ARCH = {
    "arm64": "aarch64",
    "arm": "arm",
    "x86_64": "x64",
    "x86": "x86",
}

JRE_SPECS = [
    ("8", "jre", ["jre8-20210825", "jre8"]),
    ("17", "jre-new", ["jre17", "jre17-20210825"]),
    ("21", "jre-21", ["jre21", "jdk21", "openjdk21"]),
]


def http_get_json(url: str):
    headers = {"User-Agent": "HuTao-Launcher-CI", "Accept": "application/vnd.github+json"}
    token = os.environ.get("GITHUB_TOKEN")
    if token and "github.com" in url:
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


def ensure_xz(path: str):
    """Đảm bảo file thực sự là .xz (đổi từ gzip sang xz nếu cần) vì app giải nén theo bin-*.tar.xz."""
    with open(path, "rb") as f:
        head = f.read(6)
    if head == b"\xfd7zXZ\x00":
        return
    if head[:2] == b"\x1f\x8b":
        tmp = path + ".tmp"
        with tarfile.open(path, "r:gz") as src, tarfile.open(tmp, "w:xz") as dst:
            for m in src:
                dst.addfile(m, src.extractfile(m) if m.isfile() else None)
        os.replace(tmp, path)
        return
    raise RuntimeError(f"{path} is neither XZ nor GZIP")


def detect_arch_from_name(name: str):
    low = name.lower()
    if "arm64" in low or "aarch64" in low:
        return "arm64"
    if "x86_64" in low or "amd64" in low or "x64" in low:
        return "x86_64"
    if re.search(r"[-_](arm32|aarch32|arm)(?:[-_.]|$)", low):
        return "arm"
    if re.search(r"[-_](x86|i386|i686)(?:[-_.]|$)", low):
        return "x86"
    return None


def fetch_all_releases():
    all_rels = []
    for page in range(1, 8):
        try:
            batch = http_get_json(f"{REPO_RELEASES_BASE}?per_page=100&page={page}")
        except Exception:
            break
        if not batch:
            break
        all_rels.extend(batch)
        if len(batch) < 100:
            break
    return all_rels


def try_adoptium_fallback(major: str, arch: str, standalone_dest: str):
    adopt_arch = POJAV_TO_ADOPTIUM_ARCH.get(arch)
    if not adopt_arch:
        return None
    url = f"{ADOPTIUM_V3_BASE}/{major}/ga/linux/{adopt_arch}/jdk?jvm_impl=hotspot&heap_size=normal&image_type=jre&vendor=eclipse"
    try:
        print(f"[JRE {major}] Trying Eclipse Temurin fallback for {arch} ({adopt_arch})...")
        download_file(url, standalone_dest)
        if os.path.getsize(standalone_dest) > 1024 * 1024:
            return f"temurin-{major}-ga"
    except Exception as e:
        print(f"[JRE {major}] Adoptium fallback failed for {arch}: {e}")
    if os.path.exists(standalone_dest):
        os.remove(standalone_dest)
    return None


def download_all_jres():
    print("Querying PojavLauncherTeam/android-openjdk-build-multiarch releases...")
    releases = fetch_all_releases()
    print(f"Fetched {len(releases)} release entries from GitHub.")

    all_dir = os.path.join(STAGE_ROOT, "all")
    standalone_dir = os.path.join(STAGE_ROOT, "standalone")
    os.makedirs(all_dir, exist_ok=True)
    os.makedirs(standalone_dir, exist_ok=True)

    for major, comp_name, fallback_tags in JRE_SPECS:
        comp_out = os.path.join(all_dir, comp_name)
        os.makedirs(comp_out, exist_ok=True)

        seen_ids = {r.get("id") for r in releases}
        extra_rels = []
        for ftag in fallback_tags:
            try:
                rel_obj = http_get_json(f"{REPO_TAGS_BASE}/{ftag}")
                if rel_obj.get("id") not in seen_ids:
                    extra_rels.append(rel_obj)
                    seen_ids.add(rel_obj.get("id"))
            except Exception:
                pass
        candidate_releases = releases + extra_rels

        # 1) Try finding an official Pojav binpack zip (e.g. jre8-pojav.zip, jre17-pojav.zip, jre21-pojav.zip)
        pojav_zip_asset = None
        pojav_tag = f"jre{major}-hutao"
        for rel in candidate_releases:
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

        # 2) Fetch standalone release archives (.tar.xz / .tar.gz) across candidate releases
        standalone_found = {}
        for rel in candidate_releases:
            if rel.get("draft"):
                continue
            tag = rel.get("tag_name") or f"jre{major}"
            rname = (rel.get("name") or "").lower()
            rtag_low = tag.lower()
            for asset in rel.get("assets", []):
                aname = asset["name"]
                low = aname.lower()
                if not (low.endswith(".tar.xz") or low.endswith(".tar.gz") or low.endswith(".tgz")):
                    continue
                if "debug" in low or "minimal" in low or "source" in low:
                    continue
                matches_major = (
                    re.match(rf"^(jre|jdk|openjdk)[-_]?{major}(?:[-_.]|$)", low)
                    or f"jre{major}" in low
                    or f"jdk{major}" in low
                    or f"jre{major}" in rtag_low
                    or f"jdk{major}" in rtag_low
                    or f"jre {major}" in rname
                    or f"jdk {major}" in rname
                )
                if not matches_major:
                    continue
                arch = detect_arch_from_name(low)
                if arch and arch not in standalone_found:
                    standalone_found[arch] = (tag, aname, asset["browser_download_url"])

        for arch, (tag, aname, url) in sorted(standalone_found.items()):
            standalone_dest = os.path.join(standalone_dir, f"jre{major}-{arch}.tar.xz")
            print(f"[JRE {major}] Downloading {aname} -> {os.path.basename(standalone_dest)}")
            try:
                download_file(url, standalone_dest)
                if arch not in found_arches:
                    shutil.copy2(standalone_dest, os.path.join(comp_out, f"bin-{arch}.tar.xz"))
                    ensure_xz(os.path.join(comp_out, f"bin-{arch}.tar.xz"))
                    found_arches[arch] = os.path.join(comp_out, f"bin-{arch}.tar.xz")
                    pojav_tag = tag
            except Exception as e:
                print(f"[JRE {major}] Download failed for {aname}: {e}")
                if os.path.exists(standalone_dest):
                    os.remove(standalone_dest)

        # 3) Fallback for any arch still missing (especially JRE 21)
        for arch in ["arm64", "arm", "x86_64", "x86"]:
            if arch not in found_arches:
                standalone_dest = os.path.join(standalone_dir, f"jre{major}-{arch}.tar.xz")
                # Temurin/Adoptium là bản glibc cho Linux desktop -> KHÔNG chạy được trên Android (bionic),
                # nên không dùng làm fallback nữa. Thiếu arch nào thì bỏ qua arch đó.
                ftag = None
                if ftag:
                    shutil.copy2(standalone_dest, os.path.join(comp_out, f"bin-{arch}.tar.xz"))
                    found_arches[arch] = os.path.join(comp_out, f"bin-{arch}.tar.xz")
                    pojav_tag = ftag

        print(f"[JRE {major}] Ready arches: {sorted(found_arches.keys())}")
        if not found_arches:
            if major == "21":
                print("[JRE 21] Warning: No JRE 21 archive available upstream yet; skipping JRE 21 bundle cleanly.")
                shutil.rmtree(comp_out, ignore_errors=True)
                continue
            raise RuntimeError(f"Failed to locate any release archive for JRE {major}!")

        if not os.path.exists(os.path.join(comp_out, "universal.tar.xz")):
            make_stub_universal_tar_xz(os.path.join(comp_out, "universal.tar.xz"), f"jre{major}-{pojav_tag}")

        if not os.path.exists(os.path.join(comp_out, "version")):
            with open(os.path.join(comp_out, "version"), "w", encoding="utf-8") as vf:
                vf.write(pojav_tag)


def stage_for_target(target: str):
    all_dir = os.path.join(STAGE_ROOT, "all")
    for _, comp_name, _ in JRE_SPECS:
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
