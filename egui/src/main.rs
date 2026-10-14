// Entry point of the egui edition. Self-check args (--test-fetch /
// --test-update / --test-ui) run BEFORE the single-instance check, mirroring
// rust/src-tauri/src/main.rs (SPEC 19). The GUI itself is a Phase 2
// placeholder; see docs/EGUI-MIGRATION.md for the phase plan.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kimi_planbar_tray_egui::{quota, update};

// Named mutex identical to the Rust/Qt/WPF editions (SPEC 20): guarantees
// mutual exclusion across all four editions. Do not rename.
fn acquire_single_instance_mutex() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        match CreateMutexW(None, true, w!("KimiPlanbarTray.SingleInstance")) {
            Ok(h) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    return false;
                }
                // Leak the handle on purpose: it must stay open until process
                // exit, or the mutex would be released early.
                let _keep_alive: &'static _ = Box::leak(Box::new(h));
                true
            }
            Err(_) => true, // mutex creation failure must not block startup
        }
    }
}

fn dotnet_bool(b: bool) -> &'static str {
    // C# string interpolation prints True/False
    if b {
        "True"
    } else {
        "False"
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    rt.block_on(fut)
}

// GUI-subsystem exes (release builds) have no console: print via the inherited
// STD_OUTPUT_HANDLE — a valid pipe under Git Bash, the parent's console buffer
// under cmd/PowerShell. This is the qt/ edition's WriteFile pattern
// (qt/src/main.cpp printStdout); AttachConsole + CONOUT$ is the fallback and a
// plain println! the last resort (debug builds are console-subsystem, where
// println! always works).
fn print_stdout(text: &str) {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Console::{
        AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
    };

    let write = |h: HANDLE| -> bool {
        let bytes = text.as_bytes();
        let mut written: u32 = 0;
        unsafe { WriteFile(h, Some(bytes), Some(&mut written), None).is_ok() }
    };
    unsafe {
        if let Ok(h) = GetStdHandle(STD_OUTPUT_HANDLE) {
            if !h.is_invalid() && !h.0.is_null() && write(h) {
                return;
            }
        }
        if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() {
            if let Ok(h) = CreateFileW(
                w!("CONOUT$"),
                GENERIC_WRITE.0,
                FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                let owned = write(h);
                let _ = CloseHandle(h); // only a handle we opened gets closed
                if owned {
                    return;
                }
            }
        }
    }
    println!("{text}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Headless quota self-check: fetch once, print indented JSON, exit
    if args.iter().any(|a| a == "--test-fetch") {
        let r = block_on(quota::fetch());
        let line = match serde_json::to_string_pretty(&r) {
            Ok(j) => j,
            Err(e) => format!("serialize error: {e}"),
        };
        print_stdout(&format!("{line}\n"));
        return;
    }

    // Headless update-check self-check: single-line summary, exit
    if args.iter().any(|a| a == "--test-update") {
        let st = block_on(update::check());
        print_stdout(&format!(
            "local={} latest={} updateAvailable={} checkFailed={}\n",
            st.local_version.as_deref().unwrap_or(""),
            st.latest_version.as_deref().unwrap_or(""),
            dotnet_bool(st.update_available),
            dotnet_bool(st.check_failed),
        ));
        return;
    }

    // Headless UI self-check: real window construction lands in Phase 2
    // (docs/EGUI-MIGRATION.md); the line follows the rust edition's
    // UI-FAIL: {type}: {message} format.
    if args.iter().any(|a| a == "--test-ui") {
        print_stdout("UI-FAIL: NotImplementedError: Phase 2\n");
        return;
    }

    if !acquire_single_instance_mutex() {
        return; // another instance (Rust, Qt, or WPF edition) is already running
    }

    // Placeholder until Phase 2 wires eframe + tray-icon (docs/EGUI-MIGRATION.md)
    eprintln!("egui edition: GUI not implemented yet (Phase 2)");
}
