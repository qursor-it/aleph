//! Compact Aleph specification for LLM context (T-022, ADR-0026).
//!
//! `aleph llms` prints [`render`]. The repository root `llms.txt` is the
//! checked-in output and must stay byte-identical; regenerate it with
//! `cargo run --quiet --bin aleph -- llms > llms.txt`.
//!
//! The text is built from the spec, the diagnostics table and the tests/aph
//! sources (via `include_str!`), plus the lexicon and builtin tables in Rust.
//! The hand-written digest is pinned to the sources it summarizes: see
//! [`SOURCE_DIGESTS`]. It is deterministic and has no dependency on the
//! working directory.

use std::fmt::Write as _;

use crate::ast::Capability;
use crate::builtins::{self, Bound, Effect, Ty, TyVar};
use crate::lexicon;

pub const SPEC: &str = include_str!("../../spec/SPEC.md");
pub const GRAMMAR: &str = include_str!("../../spec/grammar.ebnf");
pub const DIAGNOSTICS: &str = include_str!("../../spec/diagnostics.md");
const ADR_0018: &str = include_str!("../../adr/0018-shadowing-callables.md");
const ADR_0020: &str = include_str!("../../adr/0020-capability-path-rules.md");
const ADR_0021: &str = include_str!("../../adr/0021-runtime-semantics-v01.md");
const ADR_0022: &str = include_str!("../../adr/0022-examples-semantics.md");

/// A hand-written digest of a source that `render()` summarizes.
#[derive(Debug, Clone, Copy)]
pub struct SourceDigest {
    pub path: &'static str,
    pub contents: &'static str,
    /// FNV-1a 64 of `contents`.
    pub hash: u64,
}

/// Sources whose meaning is curated into this digest. Update a hash only after
/// reviewing the corresponding prose in `render`.
pub const SOURCE_DIGESTS: &[SourceDigest] = &[
    SourceDigest {
        path: "spec/SPEC.md",
        contents: SPEC,
        hash: 0xcfd294b4eaaaa7be,
    },
    SourceDigest {
        path: "spec/grammar.ebnf",
        contents: GRAMMAR,
        hash: 0x4e14ec3b092fe8d7,
    },
    SourceDigest {
        path: "spec/diagnostics.md",
        contents: DIAGNOSTICS,
        hash: 0x65ea0aaf5af44d02,
    },
    SourceDigest {
        path: "adr/0018-shadowing-callables.md",
        contents: ADR_0018,
        hash: 0xb5cb72055f126135,
    },
    SourceDigest {
        path: "adr/0020-capability-path-rules.md",
        contents: ADR_0020,
        hash: 0x82df01549aa5d310,
    },
    SourceDigest {
        path: "adr/0021-runtime-semantics-v01.md",
        contents: ADR_0021,
        hash: 0x162c77c23f987b50,
    },
    SourceDigest {
        path: "adr/0022-examples-semantics.md",
        contents: ADR_0022,
        hash: 0xb4bc43e4a1cf1d16,
    },
];

/// FNV-1a 64 (hand-written; no dependency).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Canonical examples, verbatim from `tests/aph/`.
const EXAMPLES: &[&str] = &[
    include_str!("../../tests/aph/llms_ops.aph"),
    include_str!("../../tests/aph/run_strings.aph"),
    include_str!("../../tests/aph/bench_upper.aph"),
    include_str!("../../tests/aph/run_opt_res.aph"),
    include_str!("../../tests/aph/run_higher_order.aph"),
    include_str!("../../tests/aph/run_records.aph"),
    include_str!("../../tests/aph/bench_fib.aph"),
    include_str!("../../tests/aph/run_eprint.aph"),
    include_str!("../../tests/aph/run_args.aph"),
    include_str!("../../tests/aph/run_show.aph"),
];

/// The ten canonical examples, in `render()` order.
pub fn examples() -> &'static [&'static str] {
    EXAMPLES
}

/// Estimate tokens as `ceil(chars / 4)` (no tokenizer exists yet).
pub fn estimated_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Render the compact specification. Deterministic: same sources, same bytes.
pub fn render() -> String {
    let mut out = String::new();
    out.push_str("# Aleph v0.1\n\n");
    out.push_str(purpose());
    out.push('\n');

    section(
        &mut out,
        spec_heading("1."),
        "`.aph`; UTF-8; outside strings only printable ASCII and LF. LF only; no tabs, trailing spaces or empty lines; exactly one final LF. One file = one module (imports v0.2).",
    );
    section(
        &mut out,
        spec_heading("2."),
        &format!(
            "One space between tokens. Column 0 = item; one leading space = `fn` body line. ident `[a-z][a-z0-9]*`; int `0|-?[1-9][0-9]*`; float `-?[0-9]+\\.[0-9]+`; string `\"...\"` escapes `\\n \\t \\\" \\\\`; `@f` fnref; `.f` field; `x:` param; `[1 2]`; ops `+ - * / % == != < <= > >=`. Keywords: `{}`.",
            keyword_list()
        ),
    );
    section(
        &mut out,
        spec_heading("3."),
        "Order: `use` by full line text, `type` by name, `fn` by name (`main` last). Body: <=1 `doc`, then `ex E = E`+ (E001; unless `main`/effectful), then `let`/`do`*, then one result `E`. Items: `use CAP [\"PREFIX\"]`; `type NAME FIELD*` (field `f:T`; ctor `NAME a b`; access `.f e`); sum `type NAME | V FIELD* | ..` (ctor `V a`); `fn NAME PARAM* -> TYPE` (param `x:T`; `main` no params -> `unit` or `res unit str`). `fn`/`type`/variant names unique, not builtin/type names (P001).",
    );
    let primitives = lexicon::TYPES
        .iter()
        .filter(|(_, name)| !matches!(*name, "list" | "opt" | "res" | "dict" | "json"))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(" ");
    section(
        &mut out,
        spec_heading("4."),
        &format!(
            "`{primitives}`; `list T`; `opt T`; `res T E`; `dict K V` (K i64/bool/str; expr `dict K V` = empty; immutable, keys ascending, not showable/comparable); record; sum. Prefix, fixed arity. Static bidirectional checking; signatures explicit, `let` inferred; no conversions/subtyping/overloading; builtins generic, user fns not. `json`: `json x` (x jsonable: unit bool i64 f64 str json, list json, dict str json; NaN -> null; depth >256 -> R005)."
        ),
    );
    section(
        &mut out,
        spec_heading("5."),
        "No parentheses; arity from signatures/ctors/builtins. `+ - * / %` 2 same numeric (`%` i64); `== !=` 2 same primitive -> bool; `< <= > >=` 2 numeric -> bool; `and or` 2 bool short-circuit; `not` 1 bool; `neg` 1 numeric; `if c a b` 3 bool/same; `[e..]` n>=1 list T; `empty T`/`none T` 1 type; `.f e` 1 field; `some`/`ok`/`err` 1; `try e` 1 res T E -> T or early err (fn returns `res U E`); `else e d` 2 opt/res default T; `has e`/`isok e` 1 -> bool; `match e V b.. E ..`: an arm per variant of e's sum, in order, a binder per field, same type. `@f`: fn value, only as higher-order arg.",
    );
    section(&mut out, spec_heading("5.1"), &builtins_table());
    section(
        &mut out,
        spec_heading("8."),
        "Deny by default. `use fs.read \"PREFIX\"` / `use fs.write \"PREFIX\"`. `net.host`/`env`/`time`: no builtins yet. Relative/absolute POSIX prefixes, normalized (empty/`.` dropped), `..` never covered. Empty prefix, `..`, or a filesystem capability without a path -> C003. Literals checked statically (C001/C002); computed paths at run time (R003). stdout/stderr allowed. Traps (JSON stderr, exit 70): R001 overflow, R002 division by zero, R003 capability, R004 stack, R005 allocation.",
    );
    section(
        &mut out,
        "## Semantics",
        "T005: params/lets may not reuse a param/local/fn/ctor/builtin name. `ex` sides see globals only; examples run hermetically (fs denied -> R003, empty args, discarded output). `print`/`eprint` add one LF (stderr flushes stdout). `show`: i64 decimal; f64 Rust `{:?}` (`1.0`, `0.1`, `1e20`); list `[1 2]` (str elements quoted, `[]` empty); top-level str raw; json compact. `jparse` RFC 8259, err `json: REASON at byte N`; `jint` integers, `jnum` any number. `upper`/`lower` ASCII only. `at` negative/oob -> `none`; `slice` clamps to `[0 len]`; `range a b` half-open (`a>=b` empty, >10,000,000 elements -> R005); `sort` ascending; `count` bytes; `split s \"\"` -> Unicode chars (`split \"\" \"\"` -> `[]`); `int` `-?[0-9]+` in i64 (leading zeros) else `err \"invalid int\"`; `float` `-?[0-9]+\\.[0-9]+` else `err \"invalid float\"`. `ok`/`err` need an expected `res` type else T010. recursion >10,000 calls or deep evaluation -> R004. `main` `err e` -> stderr, exit 1.",
    );
    section(&mut out, spec_heading("9."), &diagnostics_section());
    section(
        &mut out,
        spec_heading("10."),
        "AST is truth; text is its canonical printing. `aleph fmt` prints it; `aleph check` rejects different text (F001) with `canonical`. Invariant: `print(parse(t)) == t`; `parse(print(ast)) == ast`.",
    );
    section(
        &mut out,
        spec_heading("11."),
        "`llms` prints this text; `check FILE [--json]`; `fmt FILE [-w]`; `test FILE [--json]`; `run FILE [ARGS..]`; `build FILE -o OUT`; `explain FILE [--json]`; `ast FILE`; `tokens FILE` (v0.2). Exit 0 ok, 1 error, 2 usage, 70 trap.",
    );

    section(
        &mut out,
        "## Examples",
        "Canonical, check-clean programs (verbatim from `tests/aph/`):",
    );
    for example in EXAMPLES {
        out.push_str("\n```\n");
        out.push_str(example);
        out.push_str("```\n");
    }
    out
}

fn section(out: &mut String, heading: &str, body: &str) {
    write!(out, "\n{heading}\n{body}\n").expect("writing to a String never fails");
}

/// The purpose paragraph from the spec, verbatim.
fn purpose() -> &'static str {
    SPEC.lines()
        .find(|line| line.starts_with("Aleph is a programming language"))
        .expect("SPEC purpose paragraph")
}

/// A section heading from the spec, by exact number prefix (e.g. `"5.1"`).
fn spec_heading(prefix: &str) -> &'static str {
    SPEC.lines()
        .find(|line| {
            line.starts_with('#')
                && line
                    .trim_start_matches('#')
                    .trim_start()
                    .starts_with(prefix)
        })
        .expect("SPEC heading")
}

fn keyword_list() -> String {
    lexicon::keyword_names().collect::<Vec<_>>().join(" ")
}

/// Markdown table of `builtins::BUILTINS` (name, signature, effect, capability).
fn builtins_table() -> String {
    let mut out = String::from("`name signature`; effectful: `!` then capability, if any.\n");
    for spec in builtins::BUILTINS {
        let capability = match spec.capability {
            Some(Capability::FsRead) => " fs.read",
            Some(Capability::FsWrite) => " fs.write",
            Some(Capability::NetHost) => " net.host",
            Some(Capability::Env) => " env",
            Some(Capability::Time) => " time",
            None => "",
        };
        let effect = match spec.effect {
            Effect::Pure => "",
            Effect::Effectful => " !",
        };
        writeln!(out, "{} {}{effect}{capability}", spec.name, signature(spec))
            .expect("writing to a String never fails");
    }
    out.pop();
    out
}

fn signature(spec: &builtins::BuiltinSpec) -> String {
    let params = spec
        .params
        .iter()
        .map(|param| ty(param))
        .collect::<Vec<_>>()
        .join(",");
    let mut text = if params.is_empty() {
        format!("()->{}", ty(spec.ret))
    } else {
        format!("{params}->{}", ty(spec.ret))
    };
    if spec.bounds.iter().any(|(_, bound)| *bound != Bound::Key) {
        let bounds = spec
            .bounds
            .iter()
            .filter(|(_, bound)| *bound != Bound::Key)
            .map(|(var, bound)| bound_text(*var, *bound))
            .collect::<Vec<_>>()
            .join("; ");
        write!(text, " ({bounds})").expect("writing to a String never fails");
    }
    text
}

fn bound_text(var: TyVar, bound: Bound) -> String {
    let restriction = match bound {
        Bound::Primitive => "primitive",
        Bound::Showable => "primitive, list of primitive or json",
        // Defined once in section 4.
        Bound::Jsonable => "jsonable",
        // Stated once in section 4 instead of on every dict builtin.
        Bound::Key => "i64/bool/str",
    };
    format!("{var:?} {restriction}")
}

fn ty(value: &Ty) -> String {
    match value {
        Ty::I64 => "i64".to_string(),
        Ty::F64 => "f64".to_string(),
        Ty::Bool => "bool".to_string(),
        Ty::Str => "str".to_string(),
        Ty::Unit => "unit".to_string(),
        Ty::Json => "json".to_string(),
        Ty::List(inner) => format!("list {}", ty(inner)),
        Ty::Opt(inner) => format!("opt {}", ty(inner)),
        Ty::Res(ok, err) => format!("res {} {}", ty(ok), ty(err)),
        Ty::Dict(key, val) => format!("dict {} {}", ty(key), ty(val)),
        Ty::Var(TyVar::T) => "T".to_string(),
        Ty::Var(TyVar::A) => "A".to_string(),
        Ty::Var(TyVar::B) => "B".to_string(),
        Ty::Fn(params, ret) => {
            let params = params.iter().map(|param| ty(param)).collect::<Vec<_>>();
            format!("({}->{})", params.join(","), ty(ret))
        }
    }
}

/// Compact list of the stable codes from `spec/diagnostics.md`.
fn diagnostics_section() -> String {
    let mut codes = Vec::new();
    for line in DIAGNOSTICS.lines() {
        let line = line.trim();
        let Some(row) = line.strip_prefix('|') else {
            continue;
        };
        let cells: Vec<&str> = row
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 2
            || cells[0] == "Code"
            || cells[0].chars().all(|ch| ch == '-' || ch == ':')
        {
            continue;
        }
        codes.push(cells[0]);
    }
    format!(
        "JSON lines: required `code severity file line col msg`; optional `node expected found fix canonical`; lowercase msg; `fix` (replace/insert) only when certain. Prefixes: L lex, P parse, F format, T type, C cap, E example, R runtime, X tool. Codes: {}.",
        collapse_codes(&codes)
    )
}

/// `L001 L002 P001` -> `L001-L002 P001`: runs of consecutive codes with the
/// same prefix are written as ranges to save tokens.
fn collapse_codes(codes: &[&str]) -> String {
    let split = |code: &str| -> (String, Option<u32>) {
        let (prefix, digits) = code.split_at(code.len().min(1));
        (prefix.to_string(), digits.parse().ok())
    };
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (index, code) in codes.iter().enumerate() {
        let extends = runs.last().is_some_and(|&(_, end)| {
            let (prev_prefix, prev) = split(codes[end]);
            let (prefix, number) = split(code);
            prefix == prev_prefix && prev.zip(number).is_some_and(|(a, b)| b == a + 1)
        });
        match runs.last_mut() {
            Some(run) if extends => run.1 = index,
            _ => runs.push((index, index)),
        }
    }
    runs.iter()
        .map(|&(start, end)| {
            if start == end {
                codes[start].to_string()
            } else {
                format!("{}-{}", codes[start], codes[end])
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn collapse_codes_writes_runs_as_ranges() {
        assert_eq!(
            super::collapse_codes(&["L001", "L002", "P001", "T007", "T008", "T010", "X001"]),
            "L001-L002 P001 T007-T008 T010 X001"
        );
        assert_eq!(super::collapse_codes(&[]), "");
    }

    use super::*;

    #[test]
    fn token_estimate_rounds_up() {
        assert_eq!(estimated_tokens(""), 0);
        assert_eq!(estimated_tokens("a"), 1);
        assert_eq!(estimated_tokens("abcd"), 1);
        assert_eq!(estimated_tokens("abcde"), 2);
    }

    #[test]
    fn keywords_match_the_lexicon_exactly() {
        let expected = lexicon::keyword_names().collect::<Vec<_>>().join(" ");
        assert_eq!(keyword_list(), expected);
        let text = render();
        let marker = "Keywords: `";
        let start = text.find(marker).expect("keywords marker") + marker.len();
        let end = start + text[start..].find('`').expect("closing backtick");
        assert_eq!(&text[start..end], expected);
    }

    #[test]
    fn every_builtin_bound_is_rendered() {
        for spec in builtins::BUILTINS {
            let rendered = signature(spec);
            for (var, bound) in spec.bounds.iter().filter(|(_, b)| *b != Bound::Key) {
                let needle = bound_text(*var, *bound);
                assert!(
                    rendered.contains(&needle),
                    "bound `{needle}` of `{}` missing from `{rendered}`",
                    spec.name
                );
            }
        }
        assert_eq!(
            signature(builtins::lookup("sort").unwrap()),
            "list T->list T (T primitive)"
        );
        assert!(render().contains("`dict K V` (K i64/bool/str"));
        assert_eq!(
            signature(builtins::lookup("show").unwrap()),
            "T->str (T primitive, list of primitive or json)"
        );
    }
}
