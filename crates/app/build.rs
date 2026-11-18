//! Stamps the build with the git commit it was built from, so the screen
//! can show exactly which rcnc is running, and gives the Windows .exe its icon.

use std::path::Path;
use std::process::Command;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest = Path::new(&manifest);

    // GIT_OPTIONAL_LOCKS=0: `git status` must not rewrite the index, or every
    // build would see the index change and run this script again.
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(manifest)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".into());
    // Uncommitted changes to tracked files (new untracked files don't count).
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=RCNC_COMMIT={commit}");
    println!("cargo:rustc-env=RCNC_DIRTY={}", if dirty { "1" } else { "" });

    // Run again when the checkout moves to another commit, or any source changes.
    if let Some(dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        let dir = Path::new(&dir);
        println!("cargo:rerun-if-changed={}", dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", dir.join("index").display());
        println!("cargo:rerun-if-changed={}", dir.join("refs").display());
        println!("cargo:rerun-if-changed={}", dir.join("packed-refs").display());
    }
    if let Some(crates) = manifest.parent() {
        println!("cargo:rerun-if-changed={}", crates.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Without the Windows resource compiler the build still works; the
        // .exe just shows the default icon (the window still gets its icon).
        let icon = manifest.join("assets").join("icon.ico");
        let rc = Path::new(&std::env::var("OUT_DIR").unwrap()).join("rcnc.rc");
        let icon = icon.display().to_string().replace('\\', "/");
        std::fs::write(&rc, format!("1 ICON \"{icon}\"\n")).unwrap();
        if let Err(e) = embed_resource::compile(&rc, embed_resource::NONE).manifest_optional() {
            println!("cargo:warning=no icon on cnc-controller.exe: {e}");
        }
    }
}
