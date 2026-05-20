//! Build script: parse `crates/common/src/knobs.rs` with `syn`, find every
//! `pub static <NAME>: LazyLock<...> = LazyLock::new(|| ...env_config("ENV",
//! default)...);` declaration, capture the env-var name and the
//! doc-comment that precedes it, and emit a `KNOWN_KNOBS` slice into
//! `$OUT_DIR/known_knobs.rs` for `include!` from the registry module.

use std::{env, fs, path::PathBuf};
use syn::{visit::Visit, Attribute, Expr, ItemStatic, Lit, Meta};

const KNOBS_RS: &str = "../common/src/knobs.rs";

fn main() {
    println!("cargo:rerun-if-changed={KNOBS_RS}");
    let src = fs::read_to_string(KNOBS_RS).expect("read knobs.rs");
    let file = syn::parse_file(&src).expect("parse knobs.rs");
    let mut v = KnobVisitor { knobs: vec![] };
    v.visit_file(&file);

    let mut out = String::new();
    out.push_str("&[\n");
    for k in &v.knobs {
        let desc = k.doc.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!(
            "    KnobMeta {{ env_var: \"{}\", description: \"{}\", category: \"{}\" }},\n",
            k.env_var,
            desc,
            category_from(&k.env_var),
        ));
    }
    out.push_str("]\n");

    let dest = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("known_knobs.rs");
    fs::write(dest, out).unwrap();
}

#[derive(Debug)]
struct ExtractedKnob {
    env_var: String,
    doc: String,
}

struct KnobVisitor {
    knobs: Vec<ExtractedKnob>,
}

impl<'a> Visit<'a> for KnobVisitor {
    fn visit_item_static(&mut self, item: &'a ItemStatic) {
        // Only public statics named like SCREAMING_SNAKE_CASE.
        if !matches!(item.vis, syn::Visibility::Public(_)) {
            return;
        }
        let doc = collect_doc_comment(&item.attrs);
        // Walk the initializer for any `env_config("...", ...)` string-literal first arg.
        let env_var = extract_env_var(&item.expr);
        if let Some(env_var) = env_var {
            self.knobs.push(ExtractedKnob { env_var, doc });
        }
    }
}

fn collect_doc_comment(attrs: &[Attribute]) -> String {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(nv) = &attr.meta {
            if let Expr::Lit(lit) = &nv.value {
                if let Lit::Str(s) = &lit.lit {
                    out.push(s.value().trim().to_string());
                }
            }
        }
    }
    out.join(" ")
}

fn extract_env_var(expr: &Expr) -> Option<String> {
    // We only need the first `env_config("ENV_VAR_NAME", ...)` call we find
    // anywhere in the initializer (the only way knobs get their env vars).
    let mut found: Option<String> = None;
    syn::visit::visit_expr(
        &mut FindEnvConfig { out: &mut found },
        expr,
    );
    found
}

struct FindEnvConfig<'a> {
    out: &'a mut Option<String>,
}

impl<'ast> Visit<'ast> for FindEnvConfig<'_> {
    // We only override `visit_expr_call`. The default `Visit` impl
    // recurses into every other expression form (method calls, blocks,
    // try-expressions, etc.) and eventually reaches any nested
    // `ExprCall`. That's what makes `env_config(...).max(1)` and
    // `Duration::from_secs(env_config(...))` work — we don't need
    // explicit `visit_expr_method_call` overrides as long as we let the
    // default traversal proceed.
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.out.is_some() {
            return;
        }
        let is_env_config = if let Expr::Path(p) = &*call.func {
            p.path.segments.last().is_some_and(|s| s.ident == "env_config")
        } else {
            false
        };
        if is_env_config {
            if let Some(first_arg) = call.args.first() {
                if let Expr::Lit(lit_expr) = first_arg {
                    if let Lit::Str(s) = &lit_expr.lit {
                        *self.out = Some(s.value());
                        return;
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn category_from(env_var: &str) -> &'static str {
    // First underscore-separated word, with a few groupings collapsed.
    let first = env_var.split('_').next().unwrap_or("");
    match first {
        "FUNRUN" => "FUNRUN",
        "UDF" => "UDF",
        "TRANSACTION" => "TRANSACTION",
        "SCHEDULER" | "SCHEDULED" => "SCHEDULER",
        "ACTION" | "ACTIONS" => "ACTION",
        "DOCUMENT" | "DOCUMENTS" => "DOCUMENT",
        "HTTP" => "HTTP",
        "RUNTIME" => "RUNTIME",
        "AUDIT" => "AUDIT",
        "INDEX" => "INDEX",
        "SNAPSHOT" => "SNAPSHOT",
        "LOG" => "LOG",
        "FUNCTION" => "FUNCTION",
        "MAX" => {
            if env_var.contains("FUNRUN") {
                "FUNRUN"
            } else {
                "OTHER"
            }
        },
        _ => "OTHER",
    }
}
