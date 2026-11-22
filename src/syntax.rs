use crate::{Finding, Rule};
use proc_macro2::Span;
use std::path::PathBuf;
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, ExprIndex, ExprMethodCall, Ident, ItemUse, Macro, Path, Token, UseTree};

const FALLIBLE_METHODS: Rule = Rule {
    id: "RSA-F001",
    title: "Expected domain failures use Result rather than unwrap or expect",
    forbidden_fragments: &[".unwrap()", ".expect("],
};
const PANIC_MACROS: Rule = Rule {
    id: "RSA-F002",
    title: "Domain code does not terminate control flow with panic macros",
    forbidden_fragments: &["panic!(", "unreachable!(", "todo!(", "unimplemented!("],
};
const NAMED_EFFECTS: Rule = Rule {
    id: "RSA-F003",
    title: "Domain code does not perform named effects",
    forbidden_fragments: &[
        "std::fs",
        "std::net",
        "std::process",
        "println!(",
        "eprintln!(",
    ],
};
const AMBIENT_INPUTS: Rule = Rule {
    id: "RSA-F004",
    title: "Pass timestamps and environment-derived configuration as inputs",
    forbidden_fragments: &["std::time", "std::env", "env!(", "option_env!("],
};
const DIRECT_INDEXING: Rule = Rule {
    id: "RSA-F005",
    title: "Use get or get_mut and handle a missing index",
    forbidden_fragments: &[],
};
const DEPENDENCY_IO: Rule = Rule {
    id: "RSA-F006",
    title: "Runtime dependency source references std::fs or std::net",
    forbidden_fragments: &["std::fs", "std::net"],
};

pub const RULES: &[Rule] = &[
    FALLIBLE_METHODS,
    PANIC_MACROS,
    NAMED_EFFECTS,
    AMBIENT_INPUTS,
    DIRECT_INDEXING,
    DEPENDENCY_IO,
];

/// Inspects one UTF-8 Rust source file without reading the filesystem.
///
/// # Errors
///
/// Returns the parser diagnostic when `source` cannot be parsed as a Rust file.
pub fn inspect(path: impl Into<PathBuf>, source: &str) -> syn::Result<Vec<Finding>> {
    let syntax = syn::parse_file(source)?;
    Ok(inspect_file(path.into(), &syntax, Profile::Core))
}

/// Inspects parsed runtime dependency source for named filesystem and network paths.
#[must_use]
pub fn inspect_dependency(path: impl Into<PathBuf>, syntax: &syn::File) -> Vec<Finding> {
    inspect_file(path.into(), syntax, Profile::Dependency)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    Core,
    Dependency,
}

fn inspect_file(path: PathBuf, syntax: &syn::File, profile: Profile) -> Vec<Finding> {
    let mut inspector = Inspector {
        path,
        profile,
        findings: Vec::new(),
    };
    inspector.visit_file(syntax);
    inspector.findings.sort_unstable();
    inspector.findings.dedup();
    inspector.findings
}

struct Inspector {
    path: PathBuf,
    profile: Profile,
    findings: Vec<Finding>,
}

impl Inspector {
    fn report(&mut self, rule: Rule, span: Span) {
        let start = span.start();
        self.findings.push(Finding {
            rule_id: rule.id,
            path: self.path.clone(),
            line: start.line,
            column: start.column + 1,
            message: rule.title,
        });
    }

    fn inspect_std_module(&mut self, module: &Ident, span: Span) {
        let name = module.unraw().to_string();
        let rule = match (self.profile, name.as_str()) {
            (Profile::Core, "fs" | "net" | "process") => NAMED_EFFECTS,
            (Profile::Core, "env" | "time") => AMBIENT_INPUTS,
            (Profile::Dependency, "fs" | "net") => DEPENDENCY_IO,
            _ => return,
        };
        self.report(rule, span);
    }

    fn inspect_use(&mut self, tree: &UseTree, inside_std: bool) {
        match tree {
            UseTree::Path(path) if inside_std => {
                self.inspect_std_module(&path.ident, path.ident.span());
            }
            UseTree::Path(path) if path.ident.unraw() == "std" => {
                self.inspect_use(&path.tree, true);
            }
            UseTree::Name(name) if inside_std => {
                self.inspect_std_module(&name.ident, name.ident.span());
            }
            UseTree::Rename(rename) if inside_std => {
                self.inspect_std_module(&rename.ident, rename.ident.span());
            }
            UseTree::Group(group) => {
                for branch in &group.items {
                    self.inspect_use(branch, inside_std);
                }
            }
            _ => {}
        }
    }

    fn inspect_macro_arguments(&mut self, invocation: &Macro, name: &str) {
        // These macros accept Rust expressions. Other token grammars remain opaque.
        let mut segments = invocation.path.segments.iter();
        if let (Some(root), Some(_)) = (segments.next(), segments.next())
            && (segments.next().is_some()
                || !matches!(
                    root.ident.unraw().to_string().as_str(),
                    "std" | "core" | "alloc"
                ))
        {
            return;
        }
        if !matches!(
            name,
            "assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert"
                | "debug_assert_eq"
                | "debug_assert_ne"
                | "dbg"
                | "print"
                | "println"
                | "eprint"
                | "eprintln"
                | "format"
                | "format_args"
                | "write"
                | "writeln"
                | "panic"
                | "unreachable"
                | "todo"
                | "unimplemented"
                | "vec"
        ) {
            return;
        }
        if let Ok(arguments) =
            Punctuated::<Expr, Token![,]>::parse_terminated.parse2(invocation.tokens.clone())
        {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        } else if name == "vec"
            && let Ok(arguments) = Punctuated::<Expr, Token![;]>::parse_separated_nonempty
                .parse2(invocation.tokens.clone())
        {
            for argument in &arguments {
                self.visit_expr(argument);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Inspector {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if self.profile == Profile::Core
            && matches!(
                call.method.unraw().to_string().as_str(),
                "unwrap" | "expect"
            )
        {
            self.report(FALLIBLE_METHODS, call.dot_token.span);
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_index(&mut self, index: &'ast ExprIndex) {
        if self.profile == Profile::Core {
            self.report(DIRECT_INDEXING, index.bracket_token.span.open());
        }
        visit::visit_expr_index(self, index);
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        if let Some(segment) = invocation.path.segments.last() {
            let name = segment.ident.unraw().to_string();
            let rule = match name.as_str() {
                "panic" | "unreachable" | "todo" | "unimplemented" => Some(PANIC_MACROS),
                "println" | "eprintln" => Some(NAMED_EFFECTS),
                "env" | "option_env" => Some(AMBIENT_INPUTS),
                _ => None,
            };
            if self.profile == Profile::Core
                && let Some(rule) = rule
            {
                self.report(rule, segment.ident.span());
            }
            self.inspect_macro_arguments(invocation, &name);
        }
        // Two-segment macro paths such as std::env! name an item at the crate root.
        if invocation.path.segments.len() > 2 {
            self.visit_path(&invocation.path);
        }
    }

    fn visit_path(&mut self, path: &'ast Path) {
        let mut segments = path.segments.iter();
        if let (Some(root), Some(module)) = (segments.next(), segments.next())
            && root.ident.unraw() == "std"
        {
            self.inspect_std_module(&module.ident, root.ident.span());
        }
        visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.inspect_use(&item.tree, false);
        visit::visit_item_use(self, item);
    }
}
