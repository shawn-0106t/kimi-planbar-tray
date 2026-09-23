#!/usr/bin/env python3
"""Build the Feishu backup zip for a kimi-planbar-tray release.

Snapshot = full source tree (monorepo: wpf/ + rust/ + qt/ + docs/ + scripts/ + root files)
plus the three release binaries at the zip root. Mirrors the layout used by
the v1.3.0-and-earlier archives on Feishu Drive. The source file set comes
from `git ls-files` (tracked + untracked-but-not-ignored), so the zip can
never drift out of sync with .gitignore; a filtered walk is the fallback when
git metadata is unavailable (e.g. running from an unpacked source zip) or when
`git ls-files` answered for some other enclosing repository.

Also writes SHA256SUMS.txt (standard `sha256sum` format) next to the zip,
covering exactly the assets uploaded to a GitHub release: the zip itself
plus every binary flagged as a standalone asset. The WPF exes ship inside
the zip only, so they are not checksummed.

Both outputs are staged as `<name>.part` and only renamed once complete: the
small checksum file first, the zip last, so a failed zip rename can restore
the previous checksum file byte for byte from memory. A failed run therefore
leaves the previous (zip, checksums) pair in place instead of a truncated or
mismatched one. Release-asset names (`KimiPlanbarTray-*.exe`, droppable at the
repo root so a local `sha256sum -c` works in place), `__pycache__` / `.pyc`
leftovers, and those `.part` files themselves are never snapshotted.
"""
import hashlib
import os
import subprocess
import zipfile
from fnmatch import fnmatchcase

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))  # repo root (scripts/release/)
VERSION = "1.7.2"
OUT = os.path.join(ROOT, f"KimiPlanbarTray-v{VERSION}.zip")
PART = OUT + ".part"  # written first, renamed onto OUT last
SUMS = os.path.join(ROOT, "SHA256SUMS.txt")
SUMS_PART = SUMS + ".part"  # written first, renamed onto SUMS before the zip

# (absolute source, arcname in zip, uploaded as a standalone release asset)
BINARIES = [
    ("wpf/publish/KimiPlanbarTray.exe",
     "KimiPlanbarTray-wpf.exe", False),
    ("wpf/publish-sc/KimiPlanbarTray-selfcontained.exe",
     "KimiPlanbarTray-wpf-selfcontained.exe", False),
    ("rust/src-tauri/target/release/kimi-planbar-tray.exe",
     "KimiPlanbarTray-rust.exe", True),
]

# Never shipped even when present, matched case-folded like the rules below (all
# are gitignored; belt and braces)
EXCLUDE_FILES = {n.lower() for n in (
    "kimi logo.webp", os.path.basename(OUT), os.path.basename(PART),
    os.path.basename(SUMS), os.path.basename(SUMS_PART), "ref-v1.3.0.zip")}
# Release-asset names, matched case-folded (fnmatchcase + lower()) so the packed
# set is identical on Windows and Linux regardless of os.path.normcase. They are
# gitignored as well; matching here too keeps the non-git fallback walk from
# packing a binary dropped at the repo root for `sha256sum -c`.
EXCLUDE_PATTERNS = ("kimiplanbartray-*.exe",)
# Archives, the .part intermediates, and bytecode are never snapshot material
EXCLUDE_SUFFIXES = (".zip", ".part", ".pyc")
# Paths that must appear in this repo's own file list: when `git ls-files` resolves
# to a different enclosing repository (e.g. this tree unpacked inside someone else's
# checkout) the answer is not ours, so fall back to the filtered walk instead of
# silently shipping whichever files that other repository happens to track.
REPO_SENTINELS = ("AGENTS.md", "scripts/release/make_release_zip.py")
# Fallback walk exclusions, only used when git is unavailable. The walk cannot read
# .gitignore, so every ignored build-output directory must be named here.
EXCLUDE_DIRS = {"bin", "obj", "build", "publish", "publish-sc", "node_modules",
                "target", "dist", ".git", ".vs", "__pycache__"}


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
    if files is not None and not set(REPO_SENTINELS).issubset(files):
        files = None
    if files is None:
        files = []
        for dirpath, dirnames, filenames in os.walk(ROOT):
            dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
            for name in filenames:
                rel = os.path.relpath(os.path.join(dirpath, name), ROOT)
                files.append(rel.replace(os.sep, "/"))
    return [f for f in files
            if os.path.basename(f).lower() not in EXCLUDE_FILES
            and not any(fnmatchcase(os.path.basename(f).lower(), p) for p in EXCLUDE_PATTERNS)
            and not f.lower().endswith(EXCLUDE_SUFFIXES)
            # --cached can list a path whose worktree file was deleted but not
            # yet staged; skip it instead of aborting mid-zip.
            and os.path.isfile(os.path.join(ROOT, f))]


def add_tree(zf):
    rels = source_files()
    if not rels:
        raise SystemExit("no source files found (wrong repo root, or .gitignore too broad)")
    for rel in rels:
        zf.write(os.path.join(ROOT, rel), rel)


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def write_checksums(zip_path):
    """Stage SHA256SUMS.txt.part: the uploaded assets' hashes, the zip included.

    Staged instead of written in place so that the checksum file is renamed into
    position together with the zip; hashing the `.part` zip is safe because it
    already holds exactly the bytes the renamed zip will hold.
    """
    assets = [(arc, os.path.join(ROOT, src)) for src, arc, uploaded in BINARIES if uploaded]
    assets.append((os.path.basename(OUT), zip_path))
    lines = [f"{sha256_of(path)}  {name}" for name, path in assets]
    with open(SUMS_PART, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def read_bytes_or_none(path):
    """The file's bytes, or None when it is missing/unreadable."""
    try:
        with open(path, "rb") as f:
            return f.read()
    except OSError:
        return None


def restore_file(path, data):
    """Put `path` back the way it was; `data is None` means it did not exist."""
    if data is None:
        try:
            os.remove(path)
        except OSError:
            pass  # the failure that triggered the rollback is the actionable one
        return
    with open(path, "wb") as f:
        f.write(data)


def ensure_replaceable(paths):
    """Fail before building anything if a destination is obviously unusable.

    Catches a read-only attribute and a directory squatting on the target name,
    including on the `.part` staging paths (a read-only leftover from an earlier
    crash would otherwise surface as a bare PermissionError on the temp file).
    An exclusive lock held by another process is only discoverable when the
    rename is actually attempted, which is why the checksum file goes first:
    if the zip rename then fails, the previous checksum file is restored from
    memory and the previous pair survives.
    """
    for p in paths:
        if os.path.isdir(p):
            raise SystemExit(f"cannot replace {p}: is a directory")
        if os.path.exists(p) and not os.access(p, os.W_OK):
            raise SystemExit(f"cannot replace {p}: not writable")


def main():
    try:
        # Inside the try on purpose: a stale .part left by an earlier crash is
        # swept by the cleanup below even when this run fails its pre-flight.
        for src, _, _ in BINARIES:
            p = os.path.join(ROOT, src)
            if not os.path.isfile(p):
                raise SystemExit(f"missing binary: {src} (build first)")
        ensure_replaceable((OUT, SUMS, PART, SUMS_PART))
        with zipfile.ZipFile(PART, "w", zipfile.ZIP_DEFLATED) as zf:
            add_tree(zf)
            for src, arc, _ in BINARIES:
                zf.write(os.path.join(ROOT, src), arc)
        write_checksums(PART)
        previous_sums = read_bytes_or_none(SUMS)
        os.replace(SUMS_PART, SUMS)  # small file first: restorable from memory
        try:
            os.replace(PART, OUT)  # zip last, so its failure can be rolled back
        except OSError:
            try:
                restore_file(SUMS, previous_sums)
            except OSError:
                pass  # surface the zip-rename error rather than the rollback one
            raise
    except BaseException:  # incl. KeyboardInterrupt: never leave a half-product behind
        for tmp in (PART, SUMS_PART):
            try:
                os.remove(tmp)
            except OSError:
                pass  # keep the original failure as the surfaced error
        raise
    size_mb = os.path.getsize(OUT) / 1024 / 1024
    print(f"written: {OUT} ({size_mb:.1f} MB)")
    print(f"written: {SUMS}")


if __name__ == "__main__":
    main()
