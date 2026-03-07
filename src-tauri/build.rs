use std::path::PathBuf;

fn ensure_sidecar_binary_present() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let target = std::env::var("TARGET").unwrap_or_default();
    if manifest_dir.as_os_str().is_empty() || target.is_empty() {
        return;
    }

    let profile = std::env::var("PROFILE").unwrap_or_default();
    let is_windows_target = target.contains("windows");
    let exe_suffix = if is_windows_target { ".exe" } else { "" };

    let sidecar_name = format!("catet-cli-{}{}", target, exe_suffix);
    let sidecar_path = manifest_dir.join("binaries").join(&sidecar_name);
    if sidecar_path.exists() {
        return;
    }

    if let Some(parent) = sidecar_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Try to copy a previously built sidecar from the sibling crate.
    let workspace_root = manifest_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.clone());
    let release_src = workspace_root
        .join("catet-cli")
        .join("target")
        .join(&target)
        .join("release")
        .join(format!("catet-cli{}", exe_suffix));
    let debug_src = workspace_root
        .join("catet-cli")
        .join("target")
        .join(&target)
        .join("debug")
        .join(format!("catet-cli{}", exe_suffix));

    if release_src.exists() {
        let _ = std::fs::copy(release_src, &sidecar_path);
        return;
    }
    if debug_src.exists() {
        let _ = std::fs::copy(debug_src, &sidecar_path);
        return;
    }

    // In release/CI builds, require a real sidecar.
    if profile == "release" || std::env::var("CI").is_ok() {
        panic!(
            "Missing sidecar binary at {}. Build catet-cli first and copy it to src-tauri/binaries/{}",
            sidecar_path.display(),
            sidecar_name
        );
    }

    // In local debug builds, create a placeholder to keep `cargo check` working.
    let _ = std::fs::write(&sidecar_path, b"");
    println!(
        "cargo:warning=catet-cli sidecar not found; created placeholder at {}",
        sidecar_path.display()
    );
}

fn main() {
    ensure_sidecar_binary_present();
    tauri_build::build();
}
