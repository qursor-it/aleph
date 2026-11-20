//! Lexicon constants mirroring `spec/lexicon.toml` (ADR-0005).
//!
//! The lexicon is data: keyword, type and builtin spellings must live here and
//! be kept in sync with the spec file (a test checks they match).

/// Lexicon version, from the root `version = "..."` in `spec/lexicon.toml`.
pub const VERSION: &str = "0";

/// Keyword spellings: `(concept, spelling)`.
pub const KEYWORDS: &[(&str, &str)] = &[
    ("function", "fn"),
    ("record", "type"),
    ("capability", "use"),
    ("bind", "let"),
    ("effect", "do"),
    ("example", "ex"),
    ("intent", "doc"),
    ("cond", "if"),
    ("propagate", "try"),
    ("optsome", "some"),
    ("optnone", "none"),
    ("resok", "ok"),
    ("reserr", "err"),
    ("default", "else"),
    ("isome", "has"),
    ("isokay", "isok"),
    ("emptylist", "empty"),
    ("unitval", "nil"),
    ("true", "true"),
    ("false", "false"),
    ("land", "and"),
    ("lor", "or"),
    ("lnot", "not"),
    ("negate", "neg"),
    ("match", "match"),
    ("lambda", "lam"),
];

/// Type spellings: `(concept, spelling)`.
pub const TYPES: &[(&str, &str)] = &[
    ("int", "i64"),
    ("float", "f64"),
    ("boolean", "bool"),
    ("string", "str"),
    ("unit", "unit"),
    ("list", "list"),
    ("option", "opt"),
    ("result", "res"),
    ("dict", "dict"),
    ("json", "json"),
];

/// Operator and punctuation spellings: `(concept, spelling)`. Mirrors the
/// `[operators]` section of `spec/lexicon.toml`.
pub const OPERATORS: &[(&str, &str)] = &[
    ("add", "+"),
    ("sub", "-"),
    ("mul", "*"),
    ("div", "/"),
    ("mod", "%"),
    ("eq", "=="),
    ("ne", "!="),
    ("lt", "<"),
    ("le", "<="),
    ("gt", ">"),
    ("ge", ">="),
    ("arrow", "->"),
    ("fnref", "@"),
    ("field", "."),
    ("bar", "|"),
];

/// Binary operators (arity 2, SPEC §5).
pub const BINARY_OPERATORS: &[(&str, &str)] = &[
    ("add", "+"),
    ("sub", "-"),
    ("mul", "*"),
    ("div", "/"),
    ("mod", "%"),
    ("eq", "=="),
    ("ne", "!="),
    ("lt", "<"),
    ("le", "<="),
    ("gt", ">"),
    ("ge", ">="),
];

/// Punctuation spellings that are not operators.
pub const PUNCTUATION: &[(&str, &str)] = &[
    ("arrow", "->"),
    ("fnref", "@"),
    ("field", "."),
    ("bar", "|"),
];

/// Capability names accepted after `use` (SPEC §3.1, grammar `capname`).
pub const CAPABILITIES: &[&str] = &["fs.read", "fs.write", "net.host", "env", "time"];

/// Builtin function names.
pub const BUILTINS: &[&str] = &[
    "len", "at", "push", "slice", "rev", "repeat", "sort", "sortby", "range", "sum", "map",
    "filter", "fold", "cat", "count", "split", "join", "trim", "upper", "lower", "contains",
    "show", "int", "float", "args", "print", "env", "eprint", "fetch", "read", "write", "put",
    "get", "index", "del", "keys", "vals", "size", "json", "jparse", "jbool", "jint", "jnum",
    "jstr", "jlist", "jdict", "abs", "min", "max", "now", "starts", "ends", "uniq", "map2", "mapr",
    "ord", "chr",
];

/// Spelling of a keyword given its concept name, e.g. `keyword("function") == Some("fn")`.
pub fn keyword(concept: &str) -> Option<&'static str> {
    KEYWORDS
        .iter()
        .find(|(name, _)| *name == concept)
        .map(|(_, spelling)| *spelling)
}

/// Whether `spelling` is a reserved keyword.
pub fn is_keyword(spelling: &str) -> bool {
    KEYWORDS.iter().any(|(_, value)| *value == spelling)
}

/// All keyword spellings, in lexicon order.
pub fn keyword_names() -> impl Iterator<Item = &'static str> {
    KEYWORDS.iter().map(|(_, spelling)| *spelling)
}

/// Spelling of a type given its concept name, e.g. `type_name("int") == Some("i64")`.
pub fn type_name(concept: &str) -> Option<&'static str> {
    TYPES
        .iter()
        .find(|(name, _)| *name == concept)
        .map(|(_, spelling)| *spelling)
}

/// Whether `spelling` is a built-in type name.
pub fn is_type_name(spelling: &str) -> bool {
    TYPES.iter().any(|(_, value)| *value == spelling)
}

/// All type spellings, in lexicon order.
pub fn type_names() -> impl Iterator<Item = &'static str> {
    TYPES.iter().map(|(_, spelling)| *spelling)
}

/// Spelling of an operator or punctuation given its concept name, e.g.
/// `operator("add") == Some("+")` and `operator("arrow") == Some("->")`.
pub fn operator(concept: &str) -> Option<&'static str> {
    OPERATORS
        .iter()
        .find(|(name, _)| *name == concept)
        .map(|(_, spelling)| *spelling)
}

/// Whether `spelling` is a binary operator (arity 2, SPEC §5).
///
/// Punctuation such as `->`, `@` and `.` is *not* an operator; use
/// [`is_punctuation`].
pub fn is_operator(spelling: &str) -> bool {
    is_binary_operator(spelling)
}

/// Whether `spelling` is a binary operator (arity 2, SPEC §5).
pub fn is_binary_operator(spelling: &str) -> bool {
    BINARY_OPERATORS.iter().any(|(_, value)| *value == spelling)
}

/// Whether `spelling` is a binary operator in grammar terms: the symbolic
/// operators from [`BINARY_OPERATORS`] plus the keyword spellings `and`/`or`.
pub fn is_binop(spelling: &str) -> bool {
    is_binary_operator(spelling)
        || Some(spelling) == keyword("land")
        || Some(spelling) == keyword("lor")
}

/// Whether `spelling` is a unary operator: the keyword spellings `not`/`neg`.
pub fn is_unop(spelling: &str) -> bool {
    Some(spelling) == keyword("lnot") || Some(spelling) == keyword("negate")
}

/// Whether `name` is a capability name from [`CAPABILITIES`].
pub fn is_capability(name: &str) -> bool {
    CAPABILITIES.contains(&name)
}

/// Whether `spelling` is punctuation (`->`, `@`, `.`, `|`).
pub fn is_punctuation(spelling: &str) -> bool {
    PUNCTUATION.iter().any(|(_, value)| *value == spelling)
}

/// All operator and punctuation spellings, in lexicon order.
pub fn operator_names() -> impl Iterator<Item = &'static str> {
    OPERATORS.iter().map(|(_, spelling)| *spelling)
}

/// All binary operator spellings, in lexicon order.
pub fn binary_operator_names() -> impl Iterator<Item = &'static str> {
    BINARY_OPERATORS.iter().map(|(_, spelling)| *spelling)
}

/// All punctuation spellings, in lexicon order.
pub fn punctuation_names() -> impl Iterator<Item = &'static str> {
    PUNCTUATION.iter().map(|(_, spelling)| *spelling)
}

/// All builtin names, in lexicon order.
pub fn builtin_names() -> &'static [&'static str] {
    BUILTINS
}

/// Whether `name` is a builtin function name.
pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Entry {
        Str(String),
        List(Vec<String>),
    }

    fn quoted_strings(text: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut chars = text.chars();
        while let Some(ch) = chars.next() {
            if ch != '"' {
                continue;
            }
            let mut value = String::new();
            for ch in chars.by_ref() {
                if ch == '"' {
                    break;
                }
                value.push(ch);
            }
            values.push(value);
        }
        values
    }

    fn insert_entry(
        sections: &mut BTreeMap<String, BTreeMap<String, Entry>>,
        section: &str,
        key: String,
        entry: Entry,
    ) {
        let entries = sections.entry(section.to_string()).or_default();
        assert!(
            entries.insert(key.clone(), entry).is_none(),
            "duplicate key `{key}` in section `{section}`"
        );
    }

    /// Tiny parser for the `[section]` / `key = "value"` / `key = ["a","b"]`
    /// subset used by `spec/lexicon.toml`. Root keys live under `""`.
    fn parse_lexicon(text: &str) -> BTreeMap<String, BTreeMap<String, Entry>> {
        let mut sections: BTreeMap<String, BTreeMap<String, Entry>> = BTreeMap::new();
        let mut section = String::new();
        let mut pending_key: Option<String> = None;
        let mut pending_list: Vec<String> = Vec::new();

        for raw in text.lines() {
            let line = raw.trim();
            if pending_key.is_some() {
                pending_list.extend(quoted_strings(line));
                if line.contains(']') {
                    let key = pending_key.take().unwrap();
                    let list = std::mem::take(&mut pending_list);
                    insert_entry(&mut sections, &section, key, Entry::List(list));
                }
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                section = line.trim_matches(|c| c == '[' || c == ']').to_string();
                continue;
            }
            let (key, value) = line.split_once('=').expect("key = value line");
            let key = key.trim().to_string();
            let value = value.trim();
            if let Some(rest) = value.strip_prefix('[') {
                let list = quoted_strings(rest);
                if rest.contains(']') {
                    insert_entry(&mut sections, &section, key, Entry::List(list));
                } else {
                    pending_key = Some(key);
                    pending_list = list;
                }
            } else {
                let parsed = quoted_strings(value);
                assert_eq!(parsed.len(), 1, "expected one quoted value in {line}");
                insert_entry(&mut sections, &section, key, Entry::Str(parsed[0].clone()));
            }
        }

        assert!(pending_key.is_none(), "unterminated list in lexicon");
        sections
    }

    fn keys_of(map: &BTreeMap<String, Entry>) -> Vec<String> {
        map.keys().cloned().collect()
    }

    fn pair_keys(pairs: &[(&str, &str)]) -> Vec<String> {
        let mut keys: Vec<String> = pairs.iter().map(|(key, _)| key.to_string()).collect();
        keys.sort();
        keys
    }

    fn strings(pairs: &[(&str, &str)]) -> BTreeMap<String, Entry> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), Entry::Str(value.to_string())))
            .collect()
    }

    #[test]
    fn rust_module_matches_spec_lexicon() {
        let sections = parse_lexicon(include_str!("../../spec/lexicon.toml"));

        let root = sections.get("").expect("root section");
        assert_eq!(keys_of(root), ["version"]);
        assert_eq!(
            root.get("version"),
            Some(&Entry::Str(VERSION.to_string())),
            "version mismatch"
        );

        let keywords = sections.get("keywords").expect("keywords section");
        assert_eq!(keys_of(keywords), pair_keys(KEYWORDS));
        assert_eq!(keywords, &strings(KEYWORDS));

        let types = sections.get("types").expect("types section");
        assert_eq!(keys_of(types), pair_keys(TYPES));
        assert_eq!(types, &strings(TYPES));

        let operators = sections.get("operators").expect("operators section");
        assert_eq!(keys_of(operators), pair_keys(OPERATORS));
        assert_eq!(operators, &strings(OPERATORS));

        let builtins = sections.get("builtins").expect("builtins section");
        assert_eq!(keys_of(builtins), ["names"]);
        assert_eq!(
            builtins.get("names"),
            Some(&Entry::List(
                BUILTINS.iter().map(|name| name.to_string()).collect()
            ))
        );

        let mut expected_sections = vec!["", "keywords", "types", "operators", "builtins"];
        expected_sections.sort_unstable();
        let actual_sections: Vec<&str> = sections.keys().map(String::as_str).collect();
        assert_eq!(actual_sections, expected_sections);
    }

    #[test]
    fn spec_keyword_list_is_reserved() {
        let spec_keywords =
            "fn type use let do ex doc if try some none ok err else has isok empty nil true false and or not neg";
        for spelling in spec_keywords.split_whitespace() {
            assert!(
                is_keyword(spelling),
                "SPEC §2 keyword not reserved: {spelling}"
            );
        }
    }

    #[test]
    fn lookups_work_both_ways() {
        assert_eq!(keyword("function"), Some("fn"));
        assert_eq!(keyword("missing"), None);
        assert!(is_keyword("fn"));
        assert!(!is_keyword("function"));

        assert_eq!(type_name("int"), Some("i64"));
        assert!(is_type_name("i64"));
        assert!(!is_type_name("int"));

        assert_eq!(operator("add"), Some("+"));
        assert_eq!(operator("arrow"), Some("->"));
        assert!(is_operator("+"));
        assert!(!is_operator("->"));

        assert!(is_builtin("map"));
        assert!(!is_builtin("nope"));

        assert_eq!(keyword_names().count(), KEYWORDS.len());
        assert_eq!(type_names().count(), TYPES.len());
        assert_eq!(operator_names().count(), OPERATORS.len());
        assert_eq!(builtin_names().len(), BUILTINS.len());
    }

    #[test]
    fn operators_exclude_punctuation() {
        for spelling in ["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">="] {
            assert!(is_operator(spelling), "not an operator: {spelling}");
            assert!(is_binary_operator(spelling), "not binary: {spelling}");
            assert!(!is_punctuation(spelling), "punctuation: {spelling}");
        }
        for spelling in ["->", "@", "."] {
            assert!(
                !is_operator(spelling),
                "punctuation as operator: {spelling}"
            );
            assert!(
                !is_binary_operator(spelling),
                "binary punctuation: {spelling}"
            );
            assert!(is_punctuation(spelling), "not punctuation: {spelling}");
        }
        for (_, spelling) in OPERATORS {
            let classified =
                u8::from(is_binary_operator(spelling)) + u8::from(is_punctuation(spelling));
            assert_eq!(classified, 1, "unclassified spelling: {spelling}");
        }
        assert_eq!(binary_operator_names().count(), BINARY_OPERATORS.len());
        assert_eq!(punctuation_names().count(), PUNCTUATION.len());
    }

    #[test]
    fn operators_are_binary_operators_plus_punctuation() {
        let binary_and_punctuation: BTreeSet<&str> = BINARY_OPERATORS
            .iter()
            .chain(PUNCTUATION.iter())
            .map(|(_, spelling)| *spelling)
            .collect();
        let operators: BTreeSet<&str> = OPERATORS.iter().map(|(_, spelling)| *spelling).collect();

        assert!(binary_and_punctuation.is_subset(&operators));
        assert!(operators.is_subset(&binary_and_punctuation));
        assert_eq!(binary_and_punctuation, operators);
    }

    #[test]
    fn binops_and_unops_cover_grammar() {
        // `binop` / `unop` from spec/grammar.ebnf.
        let binops = [
            "+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "and", "or",
        ];
        let unops = ["not", "neg"];

        for spelling in binops {
            assert!(is_binop(spelling), "not a binop: {spelling}");
            assert!(!is_unop(spelling), "binop as unop: {spelling}");
        }
        for spelling in unops {
            assert!(is_unop(spelling), "not a unop: {spelling}");
            assert!(!is_binop(spelling), "unop as binop: {spelling}");
        }
        for (_, spelling) in BINARY_OPERATORS {
            assert!(is_binop(spelling), "not a binop: {spelling}");
        }
    }

    #[test]
    fn capabilities_match_grammar() {
        for name in ["fs.read", "fs.write", "net.host", "env", "time"] {
            assert!(is_capability(name), "not a capability: {name}");
        }
        assert!(!is_capability("fs.exec"));
        assert_eq!(CAPABILITIES.len(), 5);
    }
}
