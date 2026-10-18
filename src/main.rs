use rust_assay::{inspect, render_json};
use std::env;
use std::fs;
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
            return ExitCode::from(2);
        }
    };

    let mut findings = Vec::new();
    for file in &files {
        match fs::read_to_string(file) {
            Ok(source) => findings.extend(inspect(file, &source)),
            Err(error) => {
                eprintln!("rust-assay: cannot read {}: {error}", file.display());
                return ExitCode::from(2);
            }
        }
    }
    findings.sort();

    if json {
        println!("{}", render_json(&root, files.len(), &findings));
    } else if findings.is_empty() {
        println!("Pass: {} Rust files scanned.", files.len());
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

    if findings.is_empty() {
        ExitCode::SUCCESS
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
    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn visit(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            visit(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}
