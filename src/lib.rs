//! Pure, deterministic observations for the Rust functional profile.
//! The scanner is intentionally lexical: it does not claim AST or type truth.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    pub forbidden_fragments: &'static [&'static str],
}

pub const RULES: &[Rule] = &[
    Rule {
        id: "RSA-F001",
        title: "Expected domain failures use Result rather than unwrap or expect",
        forbidden_fragments: &[".unwrap()", ".expect("],
    },
    Rule {
        id: "RSA-F002",
        title: "Domain code does not terminate control flow with panic macros",
        forbidden_fragments: &["panic!(", "unreachable!(", "todo!(", "unimplemented!("],
    },
    Rule {
        id: "RSA-F003",
        title: "Domain code does not perform named effects",
        forbidden_fragments: &[
            "std::fs",
            "std::net",
            "std::process",
            "println!(",
            "eprintln!(",
        ],
    },
];

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

/// Inspects one UTF-8 Rust source file without reading the filesystem.
/// Keeping this function pure makes the rule behavior easy to test and reuse.
pub fn inspect(path: impl Into<PathBuf>, source: &str) -> Vec<Finding> {
    let path = path.into();
    let mut findings = Vec::new();

    for (line_offset, line) in source.lines().enumerate() {
        for rule in RULES {
            for fragment in rule.forbidden_fragments {
                if let Some(column_offset) = line.find(fragment) {
                    findings.push(Finding {
                        rule_id: rule.id,
                        path: path.clone(),
                        line: line_offset + 1,
                        column: column_offset + 1,
                        message: rule.title,
                    });
                    break;
                }
            }
        }
    }

    findings.sort();
    findings
}

#[must_use]
pub fn render_json(root: &Path, scanned_files: usize, findings: &[Finding]) -> String {
    let verdict = if findings.is_empty() { "Pass" } else { "Fail" };
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

    format!(
        "{{\"schemaVersion\":\"rust-assay/v0.1\",\"root\":\"{}\",\"scannedFiles\":{},\"verdict\":\"{}\",\"findings\":[{}]}}",
        escape_json(&root.display().to_string()),
        scanned_files,
        verdict,
        finding_json,
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
        assert!(inspect("domain/order.rs", source).is_empty());
    }

    #[test]
    fn scanner_reports_each_rule_at_a_stable_location() {
        let source = "let x = value.unwrap();\npanic!(\"bad\");\nprintln!(\"effect\");";
        let findings = inspect("domain/order.rs", source);
        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].rule_id, "RSA-F001");
        assert_eq!(findings[1].rule_id, "RSA-F002");
        assert_eq!(findings[2].rule_id, "RSA-F003");
    }

    #[test]
    fn json_receipt_is_deterministic() {
        let findings = inspect("domain/order.rs", "value.expect(\"present\")");
        let first = render_json(Path::new("examples/clean"), 1, &findings);
        let second = render_json(Path::new("examples/clean"), 1, &findings);
        assert_eq!(first, second);
        assert!(first.contains("\"RSA-F001\""));
    }
}
