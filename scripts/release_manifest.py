#!/usr/bin/env python3
"""Build the website's download manifest and the in-app updater feed for one release.

    release_manifest.py <assets-dir> <tag> <site-url> <out-dir> [notes-file]

Reads the installers, updater bundles and .sig files that the Release workflow attached
to a GitHub release, and writes:

  <out-dir>/manifest.json   what the website's Download section shows
  <out-dir>/latest.json     the Tauri updater feed (https://v2.tauri.app/plugin/updater/)
  <assets-dir>/SHA256SUMS.txt

All URLs point at <site-url>/downloads/<version>/, because the GitHub repo is private.
"""

import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

# Website download slots: (key, label, filename suffix).
DOWNLOADS = [
    ("mac_arm", "macOS, Apple silicon", "_aarch64.dmg"),
    ("mac_intel", "macOS, Intel", "_x64.dmg"),
    ("win_exe", "Windows installer", "-setup.exe"),
    ("win_msi", "Windows MSI", ".msi"),
    ("linux_appimage", "Linux AppImage", ".AppImage"),
    ("linux_deb", "Ubuntu, Debian", ".deb"),
    ("linux_rpm", "Fedora", ".rpm"),
]

# Updater platforms: (Tauri platform key, filename suffix of the signed update bundle).
# .deb and .rpm installs can't self-update; their users download the new package.
UPDATER = [
    ("darwin-aarch64", "_aarch64.app.tar.gz"),
    ("darwin-x86_64", "_x64.app.tar.gz"),
    ("windows-x86_64", "-setup.exe"),
    ("linux-x86_64", ".AppImage"),
]


def find(files, suffix):
    matches = [f for f in files if f.name.endswith(suffix)]
    return matches[0] if len(matches) == 1 else None


def main():
    assets, tag, site, out = Path(sys.argv[1]), sys.argv[2], sys.argv[3].rstrip("/"), Path(sys.argv[4])
    notes = Path(sys.argv[5]).read_text().strip() if len(sys.argv) > 5 and Path(sys.argv[5]).exists() else ""
    version = tag.lstrip("v")
    base = f"{site}/downloads/{version}"
    files = sorted(p for p in assets.iterdir() if p.is_file() and p.name != "SHA256SUMS.txt")
    installers = [f for f in files if not f.name.endswith(".sig")]
    out.mkdir(parents=True, exist_ok=True)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    with open(assets / "SHA256SUMS.txt", "w") as sums:
        for f in installers:
            sums.write(f"{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}\n")

    downloads = {}
    for key, label, suffix in DOWNLOADS:
        f = find(installers, suffix)
        if f:
            downloads[key] = {"label": label, "name": f.name, "url": f"{base}/{f.name}", "size": f.stat().st_size}
    missing = [k for k, _, _ in DOWNLOADS if k not in downloads]
    if missing:
        print(f"warning: no file for {', '.join(missing)}", file=sys.stderr)
    manifest = {"version": version, "pub_date": now, "checksums": f"{base}/SHA256SUMS.txt", "files": downloads}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    platforms = {}
    for key, suffix in UPDATER:
        bundle = find(installers, suffix)
        sig = bundle and assets / f"{bundle.name}.sig"
        if bundle and sig.exists():
            platforms[key] = {"signature": sig.read_text().strip(), "url": f"{base}/{bundle.name}"}
        else:
            print(f"warning: no signed update bundle for {key}", file=sys.stderr)
    if not platforms:
        sys.exit("error: no signed update bundles found; was TAURI_SIGNING_PRIVATE_KEY set for the build?")
    latest = {"version": version, "notes": notes, "pub_date": now, "platforms": platforms}
    (out / "latest.json").write_text(json.dumps(latest, indent=2) + "\n")
    print(f"{version}: {len(downloads)} downloads, {len(platforms)} update platforms")


if __name__ == "__main__":
    main()
