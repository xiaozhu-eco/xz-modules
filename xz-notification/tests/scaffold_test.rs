use std::{fs, process::Command};

#[test]
fn crate_compiles_and_namespace_exists() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert_eq!(crate_name, "xz-notification");

    #[allow(unused_imports)]
    use xz_notification::error;
}

#[test]
fn unsafe_code_is_forbidden() {
    let dir = std::env::temp_dir().join("xz_notification_unsafe_check");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let src = dir.join("unsafe_check.rs");
    fs::write(
        &src,
        r#"#![forbid(unsafe_code)]
fn main() {
    unsafe { let _ = 1usize; }
}
"#,
    )
    .unwrap();

    let output = Command::new("rustc")
        .arg("--edition=2024")
        .arg("-Dunsafe_code")
        .arg(&src)
        .current_dir(&dir)
        .output()
        .expect("rustc should run");

    assert!(!output.status.success(), "unsafe code must be rejected");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsafe"), "stderr should mention unsafe");
}
