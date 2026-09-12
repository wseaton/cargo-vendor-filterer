use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::process::{Command, Output};

use anyhow::bail;
use anyhow::Result;
use camino;
use camino::{Utf8Path, Utf8PathBuf};
use cargo_vendor_filterer::{SELF_NAME, VERSIONED_DIRS};

// Return the project root
pub(crate) fn project_root() -> Result<Utf8PathBuf> {
    let mut path = build_root()?;
    while path.exists() && path.is_dir() {
        let found_lock_file = path
            .read_dir_utf8()?
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().eq("Cargo.lock"));
        if found_lock_file {
            return Ok(path);
        }
        if !path.pop() {
            break;
        }
    }
    bail!(io::Error::from(io::ErrorKind::NotFound))
}

// Return the root of the executable's build directory
pub(crate) fn build_root() -> Result<Utf8PathBuf> {
    let mut path: Utf8PathBuf = env::current_exe()?.try_into()?;
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    Ok(path)
}

#[derive(Clone, Copy)]
pub(crate) enum VendorFormat {
    Dir,
    Tar,
    TarGz,
    TarZstd,
}

impl fmt::Display for VendorFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                VendorFormat::Dir => "dir",
                VendorFormat::Tar => "tar",
                VendorFormat::TarGz => "tar.gz",
                VendorFormat::TarZstd => "tar.zstd",
            }
        )
    }
}

#[derive(Default)]
pub(crate) struct VendorOptions<'a, 'b, 'c, 'd, 'e, 'f> {
    pub output: Option<&'a Utf8Path>,
    pub platforms: Option<&'b [&'b str]>,
    pub tier: Option<&'static str>,
    pub exclude_crate_paths: Option<&'c [&'c str]>,
    pub format: Option<VendorFormat>,
    pub manifest_path: Option<&'d Utf8Path>,
    pub sync: Vec<&'e Utf8Path>,
    pub versioned_dirs: bool,
    pub keep_dep_kinds: Option<&'static str>,
    pub current_dir: Option<&'f Utf8Path>,
    pub filter_report: Option<&'f Utf8Path>,
    pub packages: &'static [&'static str],
    pub features: &'static [&'static str],
    pub no_default_features: bool,
}

/// Run a vendoring process
pub(crate) fn vendor(options: VendorOptions) -> Result<Output> {
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;
    // Ensure we only run a vendoring process one at a time to avoid
    // excessive CPU usage.
    static LOCK: OnceCell<Mutex<()>> = OnceCell::new();

    let mut program = build_root()?;
    program.push(format!("cargo-{SELF_NAME}"));
    let mut cmd = Command::new(&program);
    cmd.current_dir(options.current_dir.unwrap_or(project_root()?.as_path())).arg(SELF_NAME);
    if let Some(platforms) = options.platforms {
        cmd.args(platforms.iter().map(|&p| format!("--platform={p}")));
    }
    if let Some(tier) = options.tier {
        cmd.args(["--tier", tier]);
    }
    if let Some(exclude_crate_paths) = options.exclude_crate_paths {
        cmd.args(
            exclude_crate_paths
                .iter()
                .map(|&p| format!("--exclude-crate-path={p}")),
        );
    }
    if let Some(format) = options.format {
        cmd.arg(format!("--format={format}"));
    }
    if let Some(manifest_path) = options.manifest_path {
        cmd.arg(format!("--manifest-path={manifest_path}"));
    }
    for s in options.sync {
        cmd.arg(format!("--sync={s}"));
    }
    if let Some(keep_dep_kinds) = options.keep_dep_kinds {
        cmd.args(["--keep-dep-kinds", keep_dep_kinds]);
    }
    if let Some(filter_report) = options.filter_report {
        cmd.arg(format!("--filter-report={filter_report}"));
    }
    for package in options.packages {
        cmd.args(["--package", package]);
    }
    for feature in options.features {
        cmd.args(["--features", feature]);
    }
    if options.no_default_features {
        cmd.arg("--no-default-features");
    }
    if let Some(output) = options.output {
        cmd.arg(output);
    }
    if options.versioned_dirs {
        cmd.arg(VERSIONED_DIRS);
    }

    Ok({
        let mutex = LOCK.get_or_init(|| Mutex::new(()));
        #[allow(unused)]
        let guard = mutex.lock().unwrap();
        println!("{:?}", cmd.get_args());
        let output = cmd.output()?;
        // io::stdout().write_all(&output.stdout)?;
        // io::stderr().write_all(&output.stderr)?;
        // use std::{thread, time};
        // thread::sleep(time::Duration::from_millis(200));
        output
    })
}

/// Allocate a temporary directory and also gather its UTF-8 path.
pub(crate) fn tempdir() -> Result<(tempfile::TempDir, Utf8PathBuf)> {
    let td = tempfile::tempdir()?;
    let path = Utf8Path::from_path(td.path()).unwrap();
    let path = path.to_owned();
    Ok((td, path))
}

pub(crate) fn write_file_create_parents(
    dir: &Utf8Path,
    path: &str,
    contents: &str,
) -> Result<Utf8PathBuf> {
    let path = dir.join(path);
    println!("writing {path}");
    fs::create_dir_all(
        path.parent()
            .ok_or(io::Error::from(io::ErrorKind::NotFound))?,
    )?;
    std::fs::write(&path, contents.as_bytes())?;
    Ok(path)
}

pub(crate) fn verify_no_windows(dir: &Utf8Path) {
    let mut windows_lib = dir.join("windows-sys/src/lib.rs");
    assert!(windows_lib.exists());
    assert!(fs::read_to_string(&windows_lib)
        .unwrap()
        .contains("compile_error!"));

    // check that only one file exists
    windows_lib.pop();
    assert_eq!(windows_lib.read_dir_utf8().unwrap().count(), 1);
}

pub(crate) fn verify_no_macos(dir: &Utf8Path) {
    let mut macos_lib = dir.join("core-foundation-sys/src/lib.rs");
    assert!(macos_lib.exists());
    assert!(fs::read_to_string(&macos_lib)
        .unwrap()
        .contains("compile_error!"));

    // check that only one file exists
    macos_lib.pop();
    assert_eq!(macos_lib.read_dir_utf8().unwrap().count(), 1);
}

pub(crate) fn verify_crate_is_stub(output_folder: &Utf8Path, name: &str) {
    let crate_dir = output_folder.join(name);
    assert!(
        crate_dir.exists(),
        "Package {name} does not show up in the vendor dir"
    );
    assert!(
        is_stub_source(&crate_dir),
        "Package {name} was kept, when it should have been filtered out!"
    );
}

pub(crate) fn verify_crate_is_no_stub(output_folder: &Utf8Path, name: &str) {
    let crate_dir = output_folder.join(name);
    assert!(
        crate_dir.exists(),
        "Package {name} does not show up in the vendor dir"
    );
    let crate_lib = crate_dir.join("src/lib.rs");
    assert!(
        crate_lib.exists(),
        "Package {name} has no src/lib.rs-file in the vendor dir"
    );
    assert!(
        !is_stub_source(&crate_dir),
        "Package {name} was filtered out, when it shouldn't have been!"
    );
}

/// Tells whether a crate directory holds a stub: `src` contains only the
/// generated `lib.rs`, and that file is exactly the stub body. Matching on the
/// contents alone would misread real crates such as `memchr`, whose own sources
/// contain `compile_error!`.
fn is_stub_source(crate_dir: &Utf8Path) -> bool {
    let src = crate_dir.join("src");
    let only_lib_rs = src
        .read_dir_utf8()
        .map(|entries| {
            let names: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            names.len() == 1 && names[0].file_name() == "lib.rs"
        })
        .unwrap_or(false);
    only_lib_rs
        && fs::read_to_string(src.join("lib.rs"))
            .map(|s| s.trim().is_empty() || s.contains("compile_error!"))
            .unwrap_or(false)
}
