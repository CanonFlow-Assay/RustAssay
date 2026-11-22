use rust_assay::{Finding, inspect, render_json_with_audit};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod dependencies;

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 10_000;
const MAX_DEPTH: usize = 64;

fn main() -> ExitCode {
    let Some(Arguments {
        root,
        json,
        manifest,
    }) = parse_arguments()
    else {
        eprintln!("usage: rust-assay check <directory> [--json] [--check-deps <Cargo.toml>]");
        return ExitCode::from(64);
    };

    let mut findings = Vec::new();
    let mut gaps = Vec::new();
    let scanned = scan_root(&root, &mut findings, &mut gaps);
    let audit = manifest.map(|path| dependencies::audit(&root, path, &mut findings, &mut gaps));
    findings.sort_unstable();
    findings.dedup();
    gaps.sort_unstable();
    gaps.dedup();

    if json {
        println!(
            "{}",
            render_json_with_audit(&root, scanned, &findings, &gaps, audit.as_ref())
        );
    } else {
        if findings.is_empty() && gaps.is_empty() {
            println!("Pass: {scanned} Rust files scanned.");
        } else if findings.is_empty() {
            println!("Inconclusive: {}", gaps.join("; "));
        } else {
            for finding in &findings {
                println!(
                    "{}:{}:{} {} {}",
                    finding.path.display(),
                    finding.line,
                    finding.column,
                    finding.rule_id,
                    finding.message
                );
            }
            for gap in &gaps {
                println!("Missing evidence: {gap}");
            }
        }
        if let Some(audit) = audit {
            println!(
                "Dependency audit: {} runtime packages in {}.",
                audit.packages.len(),
                audit.manifest.display()
            );
        }
    }

    if findings.is_empty() && gaps.is_empty() {
        ExitCode::SUCCESS
    } else if findings.is_empty() {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    }
}

fn scan_root(root: &Path, findings: &mut Vec<Finding>, gaps: &mut Vec<String>) -> usize {
    let files = match rust_files(root) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("rust-assay: {error}");
            gaps.push("Source scope is missing, unreadable or unsupported.".into());
            return 0;
        }
    };

    let mut scanned = 0;
    let mut total_bytes = 0;
    for file in &files {
        let mut bytes = Vec::new();
        match fs::File::open(file)
            .and_then(|input| input.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        {
            Ok(_) => {
                total_bytes += bytes.len() as u64;
                if bytes.len() as u64 > MAX_FILE_BYTES || total_bytes > MAX_TOTAL_BYTES {
                    gaps.push("Source bytes exceed the supported scope limit.".into());
                    break;
                }
                match String::from_utf8(bytes) {
                    Ok(source) => match inspect(file, &source) {
                        Ok(observed) => {
                            findings.extend(observed);
                            scanned += 1;
                        }
                        Err(error) => {
                            let location = error.span().start();
                            gaps.push(format!(
                                "Cannot parse {} at {}:{} ({error}).",
                                file.display(),
                                location.line,
                                location.column + 1
                            ));
                        }
                    },
                    Err(_) => gaps.push("Source could not be read as UTF-8.".into()),
                }
            }
            Err(error) => {
                eprintln!("rust-assay: cannot read {}: {error}", file.display());
                gaps.push("Source could not be read as UTF-8.".into());
            }
        }
    }
    if scanned == 0 && gaps.is_empty() {
        gaps.push("No supported source files were scanned.".into());
    }
    scanned
}

struct Arguments {
    root: PathBuf,
    json: bool,
    manifest: Option<PathBuf>,
}

fn parse_arguments() -> Option<Arguments> {
    let mut arguments = env::args_os().skip(1);
    if arguments.next()? != "check" {
        return None;
    }
    let root = PathBuf::from(arguments.next()?);
    let mut json = false;
    let mut manifest = None;
    while let Some(argument) = arguments.next() {
        if argument == "--json" {
            json = true;
        } else if argument == "--check-deps" && manifest.is_none() {
            let path = arguments.next()?;
            if path == "--json" || path == "--check-deps" {
                return None;
            }
            manifest = Some(PathBuf::from(path));
        } else {
            return None;
        }
    }
    Some(Arguments {
        root,
        json,
        manifest,
    })
}

fn rust_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    for ancestor in root.ancestors() {
        if !ancestor.as_os_str().is_empty()
            && fs::symlink_metadata(ancestor)?.file_type().is_symlink()
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source root/ancestor is a symbolic link.",
            ));
        }
    }
    let mut files = Vec::new();
    visit(root, &mut files, 0)?;
    files.sort();
    Ok(files)
}

fn visit(directory: &Path, files: &mut Vec<PathBuf>, depth: usize) -> std::io::Result<()> {
    if depth > MAX_DEPTH {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Source depth exceeds the supported limit.",
        ));
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Symbolic links make source scope incomplete.",
            ));
        }
        if kind.is_dir() {
            visit(&path, files, depth + 1)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            if !kind.is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Declare regular source files.",
                ));
            }
            files.push(path);
            if files.len() > MAX_SOURCE_FILES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Source file count exceeds the supported limit.",
                ));
            }
        }
    }
    Ok(())
}
