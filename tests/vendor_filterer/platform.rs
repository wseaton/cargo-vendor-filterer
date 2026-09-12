use super::common::{tempdir, vendor, verify_no_windows, VendorOptions};
use cargo_vendor_filterer::FilterReport;

#[test]
fn linux() {
    let (_td, mut test_folder) = tempdir().unwrap();
    test_folder.push("vendor");
    let output = vendor(VendorOptions {
        output: Some(&test_folder),
        platforms: Some(&["x86_64-unknown-linux-gnu"]),
        ..Default::default()
    })
    .unwrap();
    assert!(output.status.success());
    verify_no_windows(&test_folder);
}

#[test]
fn filter_report_lists_kept_and_stubbed() {
    let (_td, mut test_folder) = tempdir().unwrap();
    let report_path = test_folder.join("report.json");
    test_folder.push("vendor");
    let output = vendor(VendorOptions {
        output: Some(&test_folder),
        platforms: Some(&["x86_64-unknown-linux-gnu"]),
        filter_report: Some(&report_path),
        ..Default::default()
    })
    .unwrap();
    assert!(output.status.success());
    verify_no_windows(&test_folder);

    let report: FilterReport =
        serde_json::from_reader(std::fs::File::open(&report_path).unwrap()).unwrap();
    assert_eq!(report.version, 1);
    assert!(report.stubbed.iter().any(|c| c.name == "windows-sys"));
    assert!(report.kept.iter().any(|c| c.name == "anyhow"));
    for stub in &report.stubbed {
        assert!(
            !report.kept.contains(stub),
            "{} is both kept and stubbed",
            stub.name
        );
    }
}

#[test]
fn linux_multiple() {
    let (_td, mut test_folder) = tempdir().unwrap();
    test_folder.push("vendor");
    let output = vendor(VendorOptions {
        output: Some(&test_folder),
        platforms: Some(&["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]),
        ..Default::default()
    })
    .unwrap();
    assert!(output.status.success());
    verify_no_windows(&test_folder);
}

#[test]
fn linux_glob() {
    let (_td, mut test_folder) = tempdir().unwrap();
    test_folder.push("vendor");
    let output = vendor(VendorOptions {
        output: Some(&test_folder),
        platforms: Some(&["*-unknown-linux-gnu"]),
        tier: Some("2"),
        ..Default::default()
    })
    .unwrap();
    assert!(output.status.success());
    verify_no_windows(&test_folder);
}
