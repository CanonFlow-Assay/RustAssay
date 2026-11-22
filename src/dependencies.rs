use rust_assay::{DependencyAudit, DependencyPackage, Finding};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

mod sources;

const METADATA_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_METADATA_BYTES: u64 = 32 * 1024 * 1024;

pub fn audit(
    root: &Path,
    manifest: PathBuf,
    findings: &mut Vec<Finding>,
    gaps: &mut Vec<String>,
) -> DependencyAudit {
    let packages = match resolved_packages(root, &manifest) {
        Ok((metadata, root_id)) => audit_packages(&metadata, &root_id, findings, gaps),
        Err(error) => {
            gaps.push(format!(
                "Dependency audit for {}: {error}",
                manifest.display()
            ));
            Vec::new()
        }
    };
    DependencyAudit { manifest, packages }
}

#[derive(Deserialize)]
struct Metadata {
    version: u32,
    packages: Vec<Package>,
    resolve: Option<Resolve>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    manifest_path: PathBuf,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    kind: Vec<String>,
    src_path: PathBuf,
}

impl Target {
    fn is_library(&self) -> bool {
        self.kind.iter().any(|kind| {
            matches!(
                kind.as_str(),
                "lib" | "rlib" | "dylib" | "staticlib" | "cdylib"
            )
        })
    }

    fn is_proc_macro(&self) -> bool {
        self.kind.iter().any(|kind| kind == "proc-macro")
    }
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<Edge>,
    features: Vec<String>,
}

#[derive(Deserialize)]
struct Edge {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    kind: Option<String>,
}

fn resolved_packages(root: &Path, manifest: &Path) -> Result<(Metadata, String), String> {
    if manifest.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml") {
        return Err("supply a package Cargo.toml".into());
    }
    let manifest = manifest
        .canonicalize()
        .map_err(|error| format!("manifest unavailable ({error})"))?;
    let package_dir = manifest
        .parent()
        .ok_or("manifest needs a parent directory")?;
    if !root
        .canonicalize()
        .is_ok_and(|path| path.starts_with(package_dir))
    {
        return Err("the Rust source directory must be inside the selected package".into());
    }
    let metadata = cargo_metadata(&manifest, package_dir)?;
    if metadata.version != 1 {
        return Err("Cargo returned an unsupported metadata version".into());
    }
    let root_id = metadata
        .packages
        .iter()
        .find(|package| {
            package.manifest_path == manifest
                || package
                    .manifest_path
                    .canonicalize()
                    .is_ok_and(|path| path == manifest)
        })
        .ok_or("the manifest must identify one Cargo package")?
        .id
        .clone();
    Ok((metadata, root_id))
}

fn cargo_metadata(manifest: &Path, package_dir: &Path) -> Result<Metadata, String> {
    let mut child = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--frozen",
            "--all-features",
            "--manifest-path",
        ])
        .arg(manifest)
        .current_dir(package_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start Cargo metadata ({error})"))?;
    let result = read_metadata(&mut child);
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn read_metadata(child: &mut Child) -> Result<Metadata, String> {
    let deadline = Instant::now() + METADATA_TIMEOUT;
    let stdout = child
        .stdout
        .take()
        .ok_or("Cargo metadata output is unavailable")?;
    let (send, receive) = mpsc::channel();
    thread::Builder::new()
        .spawn(move || {
            let mut bytes = Vec::new();
            let result = stdout.take(MAX_METADATA_BYTES + 1).read_to_end(&mut bytes);
            let _ = send.send(result.map(|_| bytes));
        })
        .map_err(|error| format!("could not read Cargo metadata ({error})"))?;
    let bytes = receive
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| format!("Cargo metadata output is unavailable ({error})"))?
        .map_err(|error| format!("could not read Cargo metadata ({error})"))?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err("Cargo metadata exceeds the supported size limit".into());
    }
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("could not wait for Cargo metadata ({error})"))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err("Cargo metadata timed out".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        return Err(format!(
            "Cargo metadata failed ({status}); a current Cargo.lock and cached dependencies are required"
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not decode Cargo metadata ({error})"))
}

fn audit_packages(
    metadata: &Metadata,
    root: &str,
    findings: &mut Vec<Finding>,
    gaps: &mut Vec<String>,
) -> Vec<DependencyPackage> {
    let Some(resolve) = &metadata.resolve else {
        gaps.push("Cargo did not provide a resolved dependency graph.".into());
        return Vec::new();
    };
    let packages: BTreeMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect();
    let nodes: BTreeMap<_, _> = resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut audit = sources::SourceAudit::new(findings, gaps);
    let mut pending = BTreeSet::from([root]);
    let mut visited = BTreeSet::new();
    let mut checked = Vec::new();
    while let Some(id) = pending.pop_first() {
        if !visited.insert(id) {
            continue;
        }
        let Some(package) = packages.get(id) else {
            audit
                .gaps
                .push(format!("Cargo metadata is missing package {id}."));
            continue;
        };
        if id != root && package.targets.iter().any(Target::is_proc_macro) {
            continue;
        }
        let Some(node) = nodes.get(id) else {
            audit.gaps.push(format!(
                "Cargo metadata is missing dependency edges for {id}."
            ));
            continue;
        };
        pending.extend(
            node.deps
                .iter()
                .filter(|dep| dep.dep_kinds.iter().any(|kind| kind.kind.is_none()))
                .map(|dep| dep.pkg.as_str()),
        );
        if id == root {
            continue;
        }
        let Some(target) = package.targets.iter().find(|target| target.is_library()) else {
            audit.gaps.push(format!(
                "No Rust library target was supplied by runtime package {id}."
            ));
            continue;
        };
        let mut features = node.features.clone();
        features.sort_unstable();
        checked.push(DependencyPackage {
            id: id.into(),
            library: target.src_path.clone(),
            features,
            declares_no_std: audit.scan_library(&target.src_path),
        });
    }
    checked.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    checked
}
