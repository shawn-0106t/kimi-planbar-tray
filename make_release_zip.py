#!/usr/bin/env python3
"""Build the Feishu backup zip for a kimi-planbar-tray release.

Snapshot = full source tree (monorepo: wpf/ + rust/ + docs/ + root files)
plus the three release binaries at the zip root. Mirrors the layout used by
the v1.3.0-and-earlier archives on Feishu Drive. The source file set comes
from `git ls-files` (tracked + untracked-but-not-ignored), so the zip can
never drift out of sync with .gitignore; a filtered walk is the fallback when
git metadata is unavailable (e.g. running from an unpacked source zip).

Also writes SHA256SUMS.txt (standard `sha256sum` format, asset names as
uploaded to the GitHub release) next to the zip, ready to be attached as a
release asset.
"""
import hashlib
import os
import subprocess
import zipfile

ROOT = os.path.dirname(os.path.abspath(__file__))
VERSION = "1.7.2"
OUT = os.path.join(ROOT, f"KimiPlanbarTray-v{VERSION}.zip")

# (absolute source, arcname in zip)
BINARIES = [
    ("wpf/publish/KimiPlanbarTray.exe", "KimiPlanbarTray-wpf.exe"),
    ("wpf/publish-sc/KimiPlanbarTray-selfcontained.exe",
     "KimiPlanbarTray-wpf-selfcontained.exe"),
    ("rust/src-tauri/target/release/kimi-planbar-tray.exe",
     "KimiPlanbarTray-rust.exe"),
]

# Never shipped even when present (all are gitignored; belt and braces)
EXCLUDE_FILES = {"kimi logo.webp", os.path.basename(OUT),
                 "SHA256SUMS.txt", "ref-v1.3.0.zip"}
# Fallback walk exclusions, only used when git is unavailable
EXCLUDE_DIRS = {"bin", "obj", "publish", "publish-sc", "node_modules",
                "target", "dist", ".git", ".vs"}


def source_files():
    """Repo-relative paths of every file belonging in the source snapshot."""
    files = None
    try:
        out = subprocess.run(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=ROOT, capture_output=True, check=True,
        )
        files = [n for n in out.stdout.decode("utf-8", errors="replace").split("\0") if n]
    except (OSError, subprocess.CalledProcessError):
        pass
    if files is None:
        files = []
        for dirpath, dirnames, filenames in os.walk(ROOT):
            dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
            for name in filenames:
                rel = os.path.relpath(os.path.join(dirpath, name), ROOT)
                files.append(rel.replace(os.sep, "/"))
    return [f for f in files
            if os.path.basename(f) not in EXCLUDE_FILES and not f.endswith(".zip")
            # --cached can list a path whose worktree file was deleted but not
            # yet staged; skip it instead of aborting mid-zip.
            and os.path.isfile(os.path.join(ROOT, f))]


def add_tree(zf):
    for rel in source_files():
        zf.write(os.path.join(ROOT, rel), rel)


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_checksums():
    out = os.path.join(ROOT, "SHA256SUMS.txt")
    lines = [f"{sha256_of(os.path.join(ROOT, src))}  {arc}"
             for src, arc in BINARIES]
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print(f"written: {out}")


def main():
    for src, _ in BINARIES:
        p = os.path.join(ROOT, src)
        if not os.path.isfile(p):
            raise SystemExit(f"missing binary: {src} (build first)")
    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zf:
        add_tree(zf)
        for src, arc in BINARIES:
            zf.write(os.path.join(ROOT, src), arc)
    size_mb = os.path.getsize(OUT) / 1024 / 1024
    print(f"written: {OUT} ({size_mb:.1f} MB)")
    write_checksums()


if __name__ == "__main__":
    main()
