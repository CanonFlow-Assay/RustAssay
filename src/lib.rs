//! Pure, deterministic observations for the Rust functional profile.

use serde_json::json;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

mod syntax;

pub use syntax::{RULES, inspect, inspect_dependency};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    /// Representative source spellings for display. Matching uses parsed syntax.
    pub forbidden_fragments: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub rule_id: &'static str,
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: &'static str,
}

impl Ord for Finding {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.path.as_os_str(), self.line, self.column, self.rule_id).cmp(&(
            other.path.as_os_str(),
            other.line,
            other.column,
            other.rule_id,
        ))
    }
}

impl PartialOrd for Finding {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct DependencyAudit {
    pub manifest: PathBuf,
    pub packages: Vec<DependencyPackage>,
}

pub struct DependencyPackage {
    pub id: String,
    pub library: PathBuf,
    pub features: Vec<String>,
    /// Whether the crate root contains an unconditional `#![no_std]`, if parsed.
    pub declares_no_std: Option<bool>,
}

#[must_use]
pub fn render_json(root: &Path, scanned_files: usize, findings: &[Finding]) -> String {
    render_json_with_gaps(root, scanned_files, findings, &[])
}

#[must_use]
pub fn render_json_with_gaps(
    root: &Path,
    scanned_files: usize,
    findings: &[Finding],
    missing: &[String],
) -> String {
    render_json_with_audit(root, scanned_files, findings, missing, None)
}

#[must_use]
pub fn render_json_with_audit(
    root: &Path,
    scanned_files: usize,
    findings: &[Finding],
    missing: &[String],
    audit: Option<&DependencyAudit>,
) -> String {
    let mut gaps = missing.to_vec();
    if scanned_files == 0 && gaps.is_empty() {
        gaps.push("No supported source files were scanned.".into());
    }
    gaps.sort();
    gaps.dedup();
    let verdict = if !findings.is_empty() {
        "Fail"
    } else if !gaps.is_empty() {
        "Inconclusive"
    } else {
        "Pass"
    };
    let finding_json = findings
        .iter()
        .map(|finding| {
            format!(
                "{{\"ruleId\":\"{}\",\"path\":\"{}\",\"line\":{},\"column\":{},\"message\":\"{}\"}}",
                finding.rule_id,
                escape_json(&finding.path.display().to_string()),
                finding.line,
                finding.column,
                escape_json(finding.message),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let missing_json = if gaps.is_empty() {
        String::new()
    } else {
        format!(
            ",\"missingEvidence\":[{}]",
            gaps.iter()
                .map(|gap| format!("\"{}\"", escape_json(gap)))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let dependency_json = audit.map_or_else(String::new, |report| {
        let packages = report
            .packages
            .iter()
            .map(|package| {
                json!({
                    "id": package.id,
                    "library": package.library.to_string_lossy(),
                    "features": package.features,
                    "declaresNoStd": package.declares_no_std,
                })
            })
            .collect::<Vec<_>>();
        let evidence = json!({"manifest": report.manifest.to_string_lossy(), "packages": packages});
        format!(",\"dependencyAudit\":{evidence}")
    });
    format!(
        "{{\"schemaVersion\":\"rust-assay/v0.1\",\"root\":\"{}\",\"scannedFiles\":{},\"verdict\":\"{}\",\"findings\":[{}]{}{}}}",
        escape_json(&root.display().to_string()),
        scanned_files,
        verdict,
        finding_json,
        missing_json,
        dependency_json,
    )
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            control if control.is_control() => {
                format!("\\u{:04x}", control as u32).chars().collect()
            }
            ordinary => vec![ordinary],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{inspect, render_json};
    use std::path::Path;

    #[test]
    fn pure_result_domain_is_clean() {
        let source = "pub fn parse(text: &str) -> Result<String, Error> { Ok(text.into()) }";
        assert!(
            inspect("domain/order.rs", source)
                .expect("Rust syntax")
                .is_empty()
        );
    }

    #[test]
    fn scanner_reports_each_rule_at_a_stable_location() {
        let source =
            "fn f() {\nlet x = value.unwrap();\npanic!(\"bad\");\nprintln!(\"effect\");\n}";
        let findings = inspect("domain/order.rs", source).expect("Rust syntax");
        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].rule_id, "RSA-F001");
        assert_eq!(findings[1].rule_id, "RSA-F002");
        assert_eq!(findings[2].rule_id, "RSA-F003");
    }

    #[test]
    fn json_receipt_is_deterministic() {
        let findings = inspect("domain/order.rs", "fn f() { value.expect(\"present\"); }")
            .expect("Rust syntax");
        let first = render_json(Path::new("examples/clean"), 1, &findings);
        let second = render_json(Path::new("examples/clean"), 1, &findings);
        assert_eq!(first, second);
        assert!(first.contains("\"RSA-F001\""));
    }
}
