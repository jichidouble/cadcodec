//! Compiles `c_smoke.c` with the host C compiler and links it against the
//! built `cdylib`, proving the exported ABI works from a real C consumer.
//!
//! The `cc` build dependency is deliberately avoided so the crate builds
//! offline; the host compiler (gcc/clang/cl) is located at test time.

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn c_smoke_links_and_runs() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dll = find_cdylib(&manifest).expect("acadrust cdylib not found in target/");

    let cc = find_c_compiler();
    let out_exe = std::env::temp_dir().join("acadrust_ffi_c_smoke.exe");
    let src = manifest.join("tests/ffi/c_smoke.c");

    let mut cmd = Command::new(&cc);
    cmd.arg(src).arg("-o").arg(&out_exe);
    if cfg!(windows) {
        // Explicitly link the import library next to the DLL.
        let import_lib = dll.with_extension("dll.lib");
        if import_lib.exists() {
            cmd.arg(&import_lib);
        } else {
            cmd.arg(&dll);
        }
    } else {
        cmd.arg(&dll).arg("-ldl");
    }

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
    assert!(
        status.success(),
        "C smoke test failed to compile (compiler: {cc})"
    );

    // Windows DLL resolution at run time follows PATH.
    let mut run = Command::new(&out_exe);
    if cfg!(windows) {
        if let Some(dir) = dll.parent() {
            let path = format!("{};{}", dir.display(), std::env::var("PATH").unwrap_or_default());
            run.env("PATH", path);
        }
    }
    let status = run.status().expect("failed to run C smoke binary");
    assert!(status.success(), "C smoke test exited with {status}");

    let _ = std::fs::remove_file(&out_exe);
}

fn find_cdylib(manifest: &Path) -> Option<PathBuf> {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("target"));
    for dir in [target_dir.join("debug"), target_dir.join("debug/deps")] {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dll = cfg!(windows) && name.starts_with("acadrust") && name.ends_with(".dll");
                let is_so = cfg!(not(windows))
                    && (name.starts_with("libacadrust.so") || name.starts_with("libacadrust.dylib"));
                if is_dll || is_so {
                    return Some(entry.path());
                }
            }
        }
    }
    None
}

fn find_c_compiler() -> &'static str {
    for cc in ["gcc", "clang", "cl"] {
        if Command::new(cc).arg("--version").output().is_ok() {
            return cc;
        }
    }
    panic!("no C compiler found (tried gcc, clang, cl)");
}
