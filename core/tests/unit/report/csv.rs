use super::*;

#[test]
fn malicious_text_cells_are_exported_as_text_instead_of_formulas() {
    for text in [
        "=1+1", "+1+1", "-1+1", "@SUM(1)", "  =1+1", "\t=1+1", "\r=1+1", "\n=1+1",
    ] {
        let escaped = csv_text(text);
        assert!(
            escaped.starts_with('\'') || escaped.starts_with("\"'"),
            "unprotected cell: {escaped:?}"
        );
    }
    assert_eq!(csv_text("ordinary.flac"), "ordinary.flac");
    assert_eq!(csv_text("=SUM(1,2)"), "\"'=SUM(1,2)\"");
}

#[test]
fn carriage_returns_and_quotes_cannot_create_extra_csv_records() {
    assert_eq!(csv_text("line\rbreak"), "\"line\rbreak\"");
    assert_eq!(csv_text("a,\"b\"\n.flac"), "\"a,\"\"b\"\"\n.flac\"");
}

#[test]
fn every_untrusted_string_column_is_escaped_without_changing_numeric_cells() {
    // Obtain a real analysis shape independently of the CSV formatter.
    let mut file = crate::pipeline::analyze_file(
        std::path::Path::new("missing.flac"),
        &crate::ScanOptions::default(),
    );
    file.file_name = "=1+1".into();
    file.format = "format,with comma".into();
    file.codec = Some("\"quoted\" codec".into());
    file.badge = Some("@SUM(1)".into());
    file.detections.summary = "status\rline".into();
    file.file_md5 = Some("=1+1".into());
    file.file_crc32 = Some("+1+1".into());
    file.integrated_lufs = Some(-23.0);
    let report = FolderReport {
        root: String::new(),
        files: vec![file],
        has_flac: false,
    };
    let csv = build_csv(&report);
    assert!(csv.contains("\n'=1+1,\"format,with comma\",\"\"\"quoted\"\" codec\",'@SUM(1),"));
    assert!(csv.contains(",\"status\rline\","));
    assert!(csv.contains(",-23.0,"));
    assert!(csv.ends_with(",'=1+1,'+1+1\n"));
}
