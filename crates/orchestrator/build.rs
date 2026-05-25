//! Build script: parse `crates/common/src/knobs.rs` with `syn`, find every
//! `pub static <NAME>: LazyLock<...> = LazyLock::new(|| ...env_config("ENV",
//! default)...);` declaration, capture the env-var name and the
//! doc-comment that precedes it, and emit a `KNOWN_KNOBS` slice into
//! `$OUT_DIR/known_knobs.rs` for `include!` from the registry module.

use std::{env, fs, path::PathBuf};
use syn::{
    visit::Visit,
    Attribute,
    BinOp,
    Expr,
    ItemStatic,
    Lit,
    Meta,
    UnOp,
};

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
        let desc = escape_rust_str(&k.doc);
        let default_value = k
            .default_value
            .as_ref()
            .map(|value| format!("Some(\"{}\")", escape_rust_str(value)))
            .unwrap_or_else(|| "None".to_string());
        out.push_str(&format!(
            "    KnobMeta {{ env_var: \"{}\", description: \"{}\", category: \"{}\", default_value: {} }},\n",
            k.env_var,
            desc,
            category_from(&k.env_var),
            default_value,
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
    default_value: Option<String>,
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
        // Walk the initializer for any `env_config("...", default)` call.
        let env_config = extract_env_config(&item.expr);
        if let Some(env_config) = env_config {
            self.knobs.push(ExtractedKnob {
                env_var: env_config.env_var,
                doc,
                default_value: env_config.default_value,
            });
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

#[derive(Debug)]
struct ExtractedEnvConfig {
    env_var: String,
    default_value: Option<String>,
}

fn extract_env_config(expr: &Expr) -> Option<ExtractedEnvConfig> {
    // We only need the first `env_config("ENV_VAR_NAME", ...)` call we find
    // anywhere in the initializer (the only way knobs get their env vars).
    let mut found: Option<ExtractedEnvConfig> = None;
    syn::visit::visit_expr(
        &mut FindEnvConfig { out: &mut found },
        expr,
    );
    found
}

struct FindEnvConfig<'a> {
    out: &'a mut Option<ExtractedEnvConfig>,
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
                        let default_value = call.args.iter().nth(1).and_then(default_value);
                        *self.out = Some(ExtractedEnvConfig {
                            env_var: s.value(),
                            default_value,
                        });
                        return;
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn default_value(expr: &Expr) -> Option<String> {
    if let Some(value) = eval_int(expr) {
        return Some(value.to_string());
    }
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Bool(value) => Some(value.value.to_string()),
            Lit::Float(value) => Some(value.base10_digits().to_string()),
            Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        Expr::Call(call) if is_single_arg_constructor(call) => {
            call.args.first().and_then(default_value)
        },
        Expr::MethodCall(method) if method.method == "unwrap" => default_value(&method.receiver),
        Expr::Paren(paren) => default_value(&paren.expr),
        Expr::Group(group) => default_value(&group.expr),
        _ => None,
    }
}

fn eval_int(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(value) => value.base10_parse::<i128>().ok(),
            _ => None,
        },
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            eval_int(&unary.expr).map(|value| -value)
        },
        Expr::Binary(binary) => {
            let left = eval_int(&binary.left)?;
            let right = eval_int(&binary.right)?;
            match binary.op {
                BinOp::Add(_) => Some(left + right),
                BinOp::Sub(_) => Some(left - right),
                BinOp::Mul(_) => Some(left * right),
                BinOp::Div(_) => left.checked_div(right),
                BinOp::Shl(_) => u32::try_from(right)
                    .ok()
                    .and_then(|shift| left.checked_shl(shift)),
                BinOp::Shr(_) => u32::try_from(right)
                    .ok()
                    .and_then(|shift| left.checked_shr(shift)),
                _ => None,
            }
        },
        Expr::Call(call) if is_single_arg_constructor(call) => call.args.first().and_then(eval_int),
        Expr::MethodCall(method) if method.method == "unwrap" => eval_int(&method.receiver),
        Expr::Paren(paren) => eval_int(&paren.expr),
        Expr::Group(group) => eval_int(&group.expr),
        _ => None,
    }
}

fn is_single_arg_constructor(call: &syn::ExprCall) -> bool {
    if call.args.len() != 1 {
        return false;
    }
    let Expr::Path(path) = &*call.func else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "new")
}

fn escape_rust_str(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
