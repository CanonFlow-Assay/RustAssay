use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
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
        self.run_with_manifest(child, None)
    }
    fn run_with_manifest(&self, child: &str, manifest: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rust-assay"));
        command.arg("check").arg(self.0.join(child)).arg("--json");
        if let Some(manifest) = manifest {
            command.arg("--check-deps").arg(self.0.join(manifest));
        }
        command.output().expect("execute local checker")
    }
    fn write(&self, name: &str, contents: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
        fs::write(path, contents).expect("fixture contents");
    }
    fn lock(&self, package_dir: &str) {
        let result = Command::new(env!("CARGO"))
            .current_dir(self.0.join(package_dir))
            .args(["generate-lockfile", "--offline"])
            .output()
            .expect("generate local fixture lockfile");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
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

#[test]
fn malformed_rust_is_incomplete_and_other_files_still_report_findings() {
    let scope = Scope::new();
    scope.write("broken.rs", "fn broken( {");
    let result = scope.run("");
    assert_eq!(result.status.code(), Some(2));
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    assert_eq!(receipt["scannedFiles"], 0);
    assert_eq!(receipt["verdict"], "Inconclusive");
    assert!(
        receipt["missingEvidence"][0]
            .as_str()
            .expect("parse diagnostic")
            .contains("broken.rs")
    );

    scope.write("observed.rs", "fn decide(a: &[u8]) -> u8 { a[0] }");
    let result = scope.run("");
    assert_eq!(result.status.code(), Some(1));
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    assert_eq!(receipt["scannedFiles"], 1);
    assert_eq!(receipt["verdict"], "Fail");
    assert_eq!(receipt["findings"][0]["ruleId"], "RSA-F005");
    assert!(receipt["missingEvidence"].is_array());
}

#[test]
fn dependency_audit_follows_optional_renamed_and_target_specific_runtime_edges() {
    let scope = Scope::new();
    scope.write(
        "core/Cargo.toml",
        r#"
        [package]
        name = "assay-fixture-core"
        version = "0.1.0"
        edition = "2024"
        [dependencies]
        renamed = { package = "bridge", path = "../bridge", optional = true }
    "#,
    );
    scope.write("core/src/lib.rs", "pub fn decide() -> u8 { 1 }");
    scope.write(
        "bridge/Cargo.toml",
        r#"
        [package]
        name = "bridge"
        version = "0.1.0"
        edition = "2024"
        [target.'cfg(windows)'.dependencies]
        leaf = { path = "../leaf" }
    "#,
    );
    scope.write("bridge/src/lib.rs", "#![no_std]\npub fn bridge() {}");
    scope.write(
        "leaf/Cargo.toml",
        "[package]\nname = \"leaf\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    scope.write(
        "leaf/src/lib.rs",
        "mod inner { #[path = \"platform.rs\"] mod leaf_io; }",
    );
    scope.write("leaf/src/inner/platform.rs", "use std :: { fs, net };\n");
    scope.lock("core");

    assert_eq!(scope.run("core/src").status.code(), Some(0));
    let result = scope.run_with_manifest("core/src", Some("core/Cargo.toml"));
    assert_eq!(
        result.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    let packages = receipt["dependencyAudit"]["packages"]
        .as_array()
        .expect("observed packages");
    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0]["declaresNoStd"], true);
    assert_eq!(packages[1]["declaresNoStd"], false);
    let findings = receipt["findings"].as_array().expect("findings");
    assert_eq!(findings.len(), 2);
    for finding in findings {
        assert_eq!(finding["ruleId"], "RSA-F006");
        assert!(
            Path::new(finding["path"].as_str().expect("finding path"))
                .ends_with("leaf/src/inner/platform.rs")
        );
    }
    assert!(receipt.get("missingEvidence").is_none());
    assert_eq!(
        result.stdout,
        scope
            .run_with_manifest("core/src", Some("core/Cargo.toml"))
            .stdout
    );
}

#[test]
fn dependency_audit_accepts_clean_std_sources_and_excludes_host_edges() {
    let scope = Scope::new();
    scope.write(
        "core/Cargo.toml",
        r#"
        [package]
        name = "assay-fixture-core"
        version = "0.1.0"
        edition = "2024"
        [dependencies]
        pure = { path = "../pure" }
        macros = { path = "../macros" }
        [build-dependencies]
        host = { path = "../host" }
        [dev-dependencies]
        host = { path = "../host" }
    "#,
    );
    scope.write("core/src/lib.rs", "pub fn decide() -> u8 { 1 }");
    scope.write(
        "core/build.rs",
        "fn main() { panic!(\"metadata must not execute this\"); }",
    );
    scope.write(
        "pure/Cargo.toml",
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    scope.write(
        "pure/src/lib.rs",
        "pub fn value() -> String { String::new() }",
    );
    scope.write(
        "pure/src/bin/shell.rs",
        "fn main() { let _ = std::fs::read(\"unused\"); }",
    );
    scope.write(
        "macros/Cargo.toml",
        r#"
        [package]
        name = "macros"
        version = "0.1.0"
        edition = "2024"
        [lib]
        proc-macro = true
        [dependencies]
        host = { path = "../host" }
    "#,
    );
    scope.write(
        "macros/src/lib.rs",
        "use std::fs; use host as host_dependency;",
    );
    scope.write(
        "host/Cargo.toml",
        "[package]\nname = \"host\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    scope.write("host/src/lib.rs", "use std::fs; use std::net;");
    scope.lock("core");

    let result = scope.run_with_manifest("core/src", Some("core/Cargo.toml"));
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    let packages = receipt["dependencyAudit"]["packages"]
        .as_array()
        .expect("observed packages");
    assert_eq!(packages.len(), 1);
    assert!(
        Path::new(packages[0]["library"].as_str().expect("library path"))
            .ends_with("pure/src/lib.rs")
    );
    assert_eq!(packages[0]["declaresNoStd"], false);
}

#[test]
fn dependency_audit_requires_a_locked_graph_and_observable_sources() {
    let scope = Scope::new();
    scope.write(
        "core/Cargo.toml",
        r#"
        [package]
        name = "assay-fixture-core"
        version = "0.1.0"
        edition = "2024"
        [dependencies]
        incomplete = { path = "../incomplete" }
    "#,
    );
    scope.write("core/src/lib.rs", "pub fn decide() -> u8 { 1 }");
    scope.write(
        "incomplete/Cargo.toml",
        "[package]\nname = \"incomplete\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    scope.write(
        "incomplete/src/lib.rs",
        "#![no_std]\ninclude!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\nmod missing;\n",
    );

    let result = scope.run_with_manifest("core/src", Some("core/Cargo.toml"));
    assert_eq!(result.status.code(), Some(2));
    assert!(!scope.0.join("core/Cargo.lock").exists());
    scope.lock("core");
    let result = scope.run_with_manifest("core/src", Some("core/Cargo.toml"));
    assert_eq!(result.status.code(), Some(2));
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    let gaps = receipt["missingEvidence"].to_string();
    assert!(gaps.contains("include!"));
    assert!(gaps.contains("module missing"));

    scope.write("core/src/lib.rs", "fn decide(a: &[u8]) -> u8 { a[0] }");
    let result = scope.run_with_manifest("core/src", Some("core/Cargo.toml"));
    assert_eq!(result.status.code(), Some(1));
    let receipt: Value = serde_json::from_slice(&result.stdout).expect("JSON receipt");
    assert_eq!(receipt["findings"][0]["ruleId"], "RSA-F005");
    assert!(receipt["missingEvidence"].is_array());
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
