# Builds the Release exe and stages a self-contained runtime directory into
# qt/dist/ via windeployqt (dynamic linking — the LGPL-compliant route, see
# docs/archive/QT-MIGRATION.md 4.8). Run from anywhere: PYTHONUTF8=1 python qt/package_release.py

import os
import shutil
import subprocess
import sys
from pathlib import Path

# All three machine-specific paths can be overridden via environment
# variables; the defaults are this repo's documented dev machine.
QT_DIR = Path(os.environ.get("KPT_QT_DIR", "C:/Qt/6.9.3/msvc2022_64"))
CMAKE = Path(os.environ.get(
    "KPT_CMAKE",
    "C:/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/Common7/IDE/"
    "CommonExtensions/Microsoft/CMake/CMake/bin/cmake.exe"))
REDIST = Path(os.environ.get(
    "KPT_MSVC_REDIST",
    "C:/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/VC/Redist/MSVC"))
QT_ROOT = Path(__file__).resolve().parent  # qt/
BUILD = QT_ROOT / "build"
DIST = QT_ROOT / "dist"
EXE = BUILD / "Release" / "kimi-planbar-tray.exe"


def run(cmd):
    print("+ " + " ".join(str(c) for c in cmd), flush=True)
    proc = subprocess.run([str(c) for c in cmd], encoding="utf-8", errors="replace")
    if proc.returncode != 0:
        sys.exit(f"command failed with exit code {proc.returncode}")


def main():
    if not CMAKE.is_file():
        sys.exit(f"cmake not found at {CMAKE}")
    if not (QT_DIR / "bin" / "windeployqt.exe").is_file():
        sys.exit(f"Qt kit not found at {QT_DIR}")

    run([CMAKE, "--build", BUILD, "--config", "Release"])
    if not EXE.is_file():
        sys.exit(f"release exe missing: {EXE}")

    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True)
    shutil.copy2(EXE, DIST / EXE.name)

    # Widgets/Network/Svg land automatically from the exe's imports; the
    # tls/ (schannel) and imageformats plugins come along with Network/Gui.
    # No translations (UI is English-only). --compiler-runtime is requested
    # but windeployqt cannot locate the runtime under a VS Build Tools layout
    # (no VCINSTALLDIR), so the CRT DLLs are copied explicitly below.
    run([QT_DIR / "bin" / "windeployqt.exe", "--release", "--no-translations",
         "--compiler-runtime", "--dir", DIST, EXE])

    redist = REDIST
    crt = sorted(redist.glob("*/x64/Microsoft.VC14*.CRT"))
    if crt:
        copied = 0
        for dll in crt[-1].glob("*.dll"):
            shutil.copy2(dll, DIST / dll.name)
            copied += 1
        print(f"copied {copied} MSVC runtime DLLs from {crt[-1]}")
    else:
        print("warning: MSVC redist CRT not found; dist relies on the target "
              "machine having the VC++ 2022 runtime installed")

    # Offline smoke check: the staged exe must run its skills-parser self-check
    # with zero Qt directories on PATH (fails loudly if staging is broken).
    dist_exe = DIST / EXE.name
    print("+ " + str(dist_exe) + " --test-skills", flush=True)
    env = {k: v for k, v in os.environ.items() if "PATH" != k.upper()}
    env["PATH"] = os.environ.get("SystemRoot", "C:/Windows") + "/System32"
    smoke = subprocess.run([str(dist_exe), "--test-skills"], encoding="utf-8",
                           errors="replace", capture_output=True, env=env)
    sys.stdout.write(smoke.stdout or "")
    if smoke.returncode != 0:
        sys.exit(f"smoke check failed with exit code {smoke.returncode}")

    total = sum(f.stat().st_size for f in DIST.rglob("*") if f.is_file())
    count = sum(1 for f in DIST.rglob("*") if f.is_file())
    print(f"staged {count} files, {total / 1024 / 1024:.1f} MB -> {DIST}")
    print("done")


if __name__ == "__main__":
    main()
