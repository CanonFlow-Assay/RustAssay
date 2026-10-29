use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scope(PathBuf);
impl Scope {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rust-assay-scope-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("fresh test scope");
        Self(root)
    }
    fn run(&self, child: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_rust-assay"))
            .args([
                "check",
                self.0.join(child).to_str().expect("UTF-8 fixture"),
                "--json",
            ])
            .output()
            .expect("execute local checker")
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_or_empty_scope_cannot_pass() {
    let scope = Scope::new();
    for name in ["", "missing"] {
        let result = scope.run(name);
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stdout).contains("\"verdict\":\"Inconclusive\""));
    }
}

#[test]
fn invalid_utf8_is_incomplete_but_known_findings_are_retained() {
    let scope = Scope::new();
    fs::write(scope.0.join("invalid.rs"), [0xff]).expect("fixture");
    assert_eq!(scope.run("").status.code(), Some(2));
    fs::write(scope.0.join("bad.rs"), "panic!(\"bad\");").expect("fixture");
    let result = scope.run("");
    assert_eq!(result.status.code(), Some(1));
    let receipt = String::from_utf8_lossy(&result.stdout);
    assert!(receipt.contains("\"verdict\":\"Fail\""));
    assert!(receipt.contains("missingEvidence"));
    assert!(receipt.contains("RSA-F002"));
}

#[test]
fn nonempty_clean_and_violating_scopes_retain_exit_meanings() {
    let scope = Scope::new();
    let source = scope.0.join("domain.rs");
    fs::write(&source, "fn decide() -> u8 { 1 }").expect("fixture");
    assert_eq!(scope.run("").status.code(), Some(0));
    fs::write(&source, "panic!(\"bad\");").expect("fixture");
    assert_eq!(scope.run("").status.code(), Some(1));
}

#[cfg(unix)]
#[test]
fn root_file_and_directory_links_are_incomplete() {
    use std::os::unix::fs::symlink;
    let scope = Scope::new();
    let source = scope.0.join("source");
    fs::create_dir(&source).expect("fixture");
    let outside = scope.0.join("outside");
    fs::create_dir(&outside).expect("fixture");
    fs::write(outside.join("good.rs"), "fn decide() -> u8 { 1 }").expect("fixture");
    for (name, target) in [
        ("linked.rs", outside.join("good.rs")),
        ("directory", outside.clone()),
    ] {
        let link = source.join(name);
        symlink(target, &link).expect("fixture");
        assert_eq!(scope.run("source").status.code(), Some(2));
        fs::remove_file(link).expect("restore fixture");
    }
    symlink(&outside, scope.0.join("linked-root")).expect("fixture");
    assert_eq!(scope.run("linked-root").status.code(), Some(2));
}
