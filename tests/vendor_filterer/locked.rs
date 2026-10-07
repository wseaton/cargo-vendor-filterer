use std::fs;
use std::process::{Command, Output};

use camino::Utf8PathBuf;

use super::common::{
    tempdir, vendor, verify_crate_is_no_stub, verify_crate_is_stub, write_file_create_parents,
    VendorOptions,
};

const MANIFEST: &str = r#"
[package]
name = "locked-test"
version = "0.1.0"
edition = "2021"

[dependencies]
bitflags = "1.3"
memchr = { version = "2", optional = true }
"#;

/// A crate with a Cargo.lock generated from its manifest.
struct Fixture {
    _td: tempfile::TempDir,
    dir: Utf8PathBuf,
    manifest: Utf8PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let (td, dir) = tempdir().unwrap();
        let manifest = write_file_create_parents(&dir, "Cargo.toml", MANIFEST).unwrap();
        write_file_create_parents(&dir, "src/lib.rs", "").unwrap();
        let status = Command::new("cargo")
            .args(["generate-lockfile", "--manifest-path", manifest.as_str()])
            .status()
            .unwrap();
        assert!(status.success());
        Self {
            _td: td,
            dir,
            manifest,
        }
    }

    /// Add a dependency the lockfile does not know about.
    fn make_lockfile_stale(&self) {
        write_file_create_parents(
            &self.dir,
            "Cargo.toml",
            &format!("{MANIFEST}itoa = \"1\"\n"),
        )
        .unwrap();
    }

    fn lockfile(&self) -> Vec<u8> {
        fs::read(self.dir.join("Cargo.lock")).unwrap()
    }

    fn output(&self) -> Utf8PathBuf {
        self.dir.join("vendor")
    }
}

fn assert_rejected_as_stale(fixture: &Fixture, before: &[u8], output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stale Cargo.lock was accepted");
    assert!(stderr.contains("--locked"), "{stderr}");
    assert_eq!(fixture.lockfile(), before, "Cargo.lock was rewritten");
    assert!(!fixture.output().exists());
}

#[test]
fn stale_lockfile_is_rejected() {
    let fixture = Fixture::new();
    fixture.make_lockfile_stale();
    let before = fixture.lockfile();
    let output_folder = fixture.output();
    let output = vendor(VendorOptions {
        output: Some(&output_folder),
        manifest_path: Some(&fixture.manifest),
        locked: true,
        ..Default::default()
    })
    .unwrap();
    assert_rejected_as_stale(&fixture, &before, &output);
}

#[test]
fn stale_lockfile_is_rejected_with_platform_and_dep_kinds() {
    let fixture = Fixture::new();
    fixture.make_lockfile_stale();
    let before = fixture.lockfile();
    let output_folder = fixture.output();
    let output = vendor(VendorOptions {
        output: Some(&output_folder),
        manifest_path: Some(&fixture.manifest),
        platforms: Some(&["x86_64-unknown-linux-gnu"]),
        keep_dep_kinds: Some("no-dev"),
        locked: true,
        ..Default::default()
    })
    .unwrap();
    assert_rejected_as_stale(&fixture, &before, &output);
}

#[test]
fn stale_lockfile_is_rejected_with_package_selection() {
    let fixture = Fixture::new();
    fixture.make_lockfile_stale();
    let before = fixture.lockfile();
    let output_folder = fixture.output();
    let output = vendor(VendorOptions {
        output: Some(&output_folder),
        manifest_path: Some(&fixture.manifest),
        platforms: Some(&["x86_64-unknown-linux-gnu"]),
        keep_dep_kinds: Some("no-dev"),
        packages: &["locked-test"],
        locked: true,
        ..Default::default()
    })
    .unwrap();
    assert_rejected_as_stale(&fixture, &before, &output);
}

#[test]
fn in_sync_lockfile_is_vendored_with_platform() {
    let fixture = Fixture::new();
    let before = fixture.lockfile();
    let output_folder = fixture.output();
    let output = vendor(VendorOptions {
        output: Some(&output_folder),
        manifest_path: Some(&fixture.manifest),
        platforms: Some(&["x86_64-unknown-linux-gnu"]),
        keep_dep_kinds: Some("no-dev"),
        packages: &["locked-test"],
        locked: true,
        ..Default::default()
    })
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.lockfile(), before);
    verify_crate_is_no_stub(&output_folder, "bitflags");
    verify_crate_is_stub(&output_folder, "memchr");
}
