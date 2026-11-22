use crate::{MAX_DEPTH, MAX_FILE_BYTES, MAX_SOURCE_FILES, MAX_TOTAL_BYTES};
use rust_assay::{Finding, inspect_dependency};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ItemMod, Lit, Macro, Meta, Token};

pub(super) struct SourceAudit<'a> {
    findings: &'a mut Vec<Finding>,
    pub(super) gaps: &'a mut Vec<String>,
    seen: BTreeSet<(PathBuf, PathBuf)>,
    files: usize,
    bytes: u64,
}

impl<'a> SourceAudit<'a> {
    pub(super) const fn new(findings: &'a mut Vec<Finding>, gaps: &'a mut Vec<String>) -> Self {
        Self {
            findings,
            gaps,
            seen: BTreeSet::new(),
            files: 0,
            bytes: 0,
        }
    }

    pub(super) fn scan_library(&mut self, path: &Path) -> Option<bool> {
        self.seen.clear();
        self.scan_file(path, parent_dir(path), 0)
    }

    fn scan_file(&mut self, path: &Path, module_dir: &Path, depth: usize) -> Option<bool> {
        if !self
            .seen
            .insert((path.to_path_buf(), module_dir.to_path_buf()))
        {
            return None;
        }
        let syntax = match self.read_syntax(path) {
            Ok(syntax) => syntax,
            Err(error) => {
                self.gaps
                    .push(format!("Dependency source {}: {error}", path.display()));
                return None;
            }
        };
        self.findings.extend(inspect_dependency(path, &syntax));
        let declares_no_std = syntax
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("no_std") && matches!(attr.meta, Meta::Path(_)));
        Modules {
            audit: self,
            file: path,
            module_dir,
            path_base: parent_dir(path),
            depth,
        }
        .visit_file(&syntax);
        Some(declares_no_std)
    }

    fn read_syntax(&mut self, path: &Path) -> Result<syn::File, String> {
        if self.files >= MAX_SOURCE_FILES || self.bytes >= MAX_TOTAL_BYTES {
            return Err("source scope exceeds the supported file or byte limit".into());
        }
        self.files += 1;
        let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("expected a regular Rust source file".into());
        }
        let limit = MAX_FILE_BYTES.min(MAX_TOTAL_BYTES - self.bytes);
        let mut bytes = Vec::new();
        fs::File::open(path)
            .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
            .map_err(|error| error.to_string())?;
        self.bytes += bytes.len() as u64;
        if bytes.len() as u64 > limit {
            return Err("source exceeds the supported byte limit".into());
        }
        let source = String::from_utf8(bytes).map_err(|error| error.to_string())?;
        syn::parse_file(&source).map_err(|error| {
            let location = error.span().start();
            format!(
                "cannot parse Rust at {}:{} ({error})",
                location.line,
                location.column + 1
            )
        })
    }
}

struct Modules<'a, 'output> {
    audit: &'a mut SourceAudit<'output>,
    file: &'a Path,
    module_dir: &'a Path,
    path_base: &'a Path,
    depth: usize,
}

impl<'ast> Visit<'ast> for Modules<'_, '_> {
    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        let name = module.ident.unraw().to_string();
        let context = format!("Dependency module {name} in {}", self.file.display());
        if self.depth >= MAX_DEPTH {
            self.audit
                .gaps
                .push(format!("{context}: nesting exceeds the supported depth"));
            return;
        }
        if module
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("cfg_attr") && supplies_path(&attr.meta))
        {
            self.audit.gaps.push(format!(
                "{context}: conditional #[path] is outside the supported scope"
            ));
        }
        let explicit = match path_attribute(&module.attrs) {
            Ok(path) => path.map(|path| self.path_base.join(path)),
            Err(error) => {
                self.audit.gaps.push(format!("{context}: {error}"));
                return;
            }
        };
        let depth = self.depth + 1;
        let conventional_dir = self.module_dir.join(&name);
        if let Some((_, items)) = &module.content {
            let directory = explicit.as_deref().unwrap_or(&conventional_dir);
            let mut nested = Modules {
                audit: self.audit,
                file: self.file,
                module_dir: directory,
                path_base: directory,
                depth,
            };
            for item in items {
                nested.visit_item(item);
            }
        } else if let Some(path) = explicit {
            self.audit.scan_file(&path, parent_dir(&path), depth);
        } else {
            let candidates = [
                self.module_dir.join(format!("{name}.rs")),
                conventional_dir.join("mod.rs"),
            ];
            let mut matched = 0;
            for path in candidates {
                if !matches!(path.try_exists(), Ok(false)) {
                    matched += 1;
                    self.audit.scan_file(&path, &conventional_dir, depth);
                }
            }
            if matched != 1 {
                self.audit.gaps.push(format!(
                    "{context}: expected one source file, found {matched}"
                ));
            }
        }
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        if let Some(segment) = invocation.path.segments.last()
            && segment.ident.unraw() == "include"
        {
            let line = segment.ident.span().start().line;
            self.audit.gaps.push(format!("Dependency source {}:{line}: include! expands external Rust outside the supported scope", self.file.display()));
        }
        visit::visit_macro(self, invocation);
    }
}

fn path_attribute(attributes: &[Attribute]) -> Result<Option<String>, &'static str> {
    let mut paths = attributes
        .iter()
        .filter(|attr| attr.path().is_ident("path"));
    let Some(attribute) = paths.next() else {
        return Ok(None);
    };
    if paths.next().is_some() {
        return Err("multiple #[path] attributes");
    }
    if let Meta::NameValue(value) = &attribute.meta
        && let Expr::Lit(expression) = &value.value
        && let Lit::Str(path) = &expression.lit
    {
        return Ok(Some(path.value()));
    }
    Err("expected a literal string in #[path]")
}

fn supplies_path(meta: &Meta) -> bool {
    if meta.path().is_ident("path") {
        return true;
    }
    if let Meta::List(list) = meta
        && list.path.is_ident("cfg_attr")
    {
        return list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_or(true, |arguments| {
                arguments.iter().skip(1).any(supplies_path)
            });
    }
    false
}

fn parent_dir(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}
