use std::path::{Path, PathBuf};

fn is_usable_binary(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.len() > 0,
        Err(_) => false,
    }
}

fn candidate_sidecar_sources(
    workspace_root: &Path,
    target: &str,
    profile: &str,
    exe_suffix: &str,
) -> Vec<PathBuf> {
    let base = workspace_root.join("catet-cli").join("target");
    let bin_name = format!("catet-cli{}", exe_suffix);

    vec![
        base.join(target).join(profile).join(&bin_name),
        base.join(profile).join(&bin_name),
        base.join(target).join("release").join(&bin_name),
        base.join("release").join(&bin_name),
        base.join(target).join("debug").join(&bin_name),
        base.join("debug").join(&bin_name),
    ]
}

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
    let existing_sidecar_usable = is_usable_binary(&sidecar_path);

    if let Some(parent) = sidecar_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Try to copy a previously built sidecar from the sibling crate.
    let workspace_root = manifest_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.clone());
    for src in candidate_sidecar_sources(&workspace_root, &target, &profile, exe_suffix) {
        if !is_usable_binary(&src) {
            continue;
        }
        if std::fs::copy(&src, &sidecar_path).is_ok() && is_usable_binary(&sidecar_path) {
            return;
        }
    }

    // Keep an existing usable sidecar if no source binary can be copied.
    if existing_sidecar_usable {
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
