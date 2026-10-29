use rust_assay::{inspect, render_json_with_gaps};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some((root, json)) = parse_arguments() else {
        eprintln!("usage: rust-assay check <directory> [--json]");
        return ExitCode::from(64);
    };

    let files = match rust_files(&root) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("rust-assay: {error}");
            if json {
                println!(
                    "{}",
                    render_json_with_gaps(
                        &root,
                        0,
                        &[],
                        &["Source scope is missing, unreadable or unsupported.".into()]
                    )
                );
            }
            return ExitCode::from(2);
        }
    };

    let mut findings = Vec::new();
    let mut gaps = Vec::new();
    let mut scanned = 0;
    let mut total_bytes = 0;
    for file in &files {
        let mut bytes = Vec::new();
        match fs::File::open(file)
            .and_then(|input| input.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes))
        {
            Ok(_) => {
                total_bytes += bytes.len();
                if bytes.len() > 4 * 1024 * 1024 || total_bytes > 64 * 1024 * 1024 {
                    gaps.push("Source bytes exceed the supported scope limit.".into());
                    break;
                }
                match String::from_utf8(bytes) {
                    Ok(source) => {
                        findings.extend(inspect(file, &source));
                        scanned += 1;
                    }
                    Err(_) => gaps.push("Source could not be read as UTF-8.".into()),
                }
            }
            Err(error) => {
                eprintln!("rust-assay: cannot read {}: {error}", file.display());
                gaps.push("Source could not be read as UTF-8.".into());
            }
        }
    }
    findings.sort();
    if scanned == 0 && gaps.is_empty() {
        gaps.push("No supported source files were scanned.".into());
    }

    if json {
        println!(
            "{}",
            render_json_with_gaps(&root, scanned, &findings, &gaps)
        );
    } else if findings.is_empty() && gaps.is_empty() {
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
    }

    if findings.is_empty() && gaps.is_empty() {
        ExitCode::SUCCESS
    } else if findings.is_empty() {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    }
}

fn parse_arguments() -> Option<(PathBuf, bool)> {
    let mut arguments = env::args().skip(1);
    if arguments.next()?.as_str() != "check" {
        return None;
    }
    let root = PathBuf::from(arguments.next()?);
    let remaining = arguments.collect::<Vec<_>>();
    if remaining.iter().any(|argument| argument != "--json") {
        return None;
    }
    Some((root, remaining.iter().any(|argument| argument == "--json")))
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
    if depth > 64 {
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
            if files.len() > 10_000 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Source file count exceeds the supported limit.",
                ));
            }
        }
    }
    Ok(())
}
