//! T-022: root `llms.txt` is the deterministic output of `aleph llms`, its
//! ten examples are canonical and check-clean, and it stays small (ADR-0026).

use std::path::{Path, PathBuf};
use std::process::Command;

use alephc::{caps, check, examples, llms, parser, printer};

const TOKEN_BUDGET: usize = 2050;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

fn regenerate_command() -> &'static str {
    "cargo run --quiet --bin aleph -- llms > llms.txt"
}

/// The contents of every fenced block, in order.
fn fenced_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line == "```" {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    assert!(current.is_none(), "unterminated fenced block");
    blocks
}

#[test]
fn root_llms_txt_matches_render() {
    let rendered = llms::render();
    let path = repo_root().join("llms.txt");
    let checked_in = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}; regenerate with `{}`",
            path.display(),
            regenerate_command()
        )
    });
    assert_eq!(
        checked_in,
        rendered,
        "llms.txt is out of date; regenerate with `{}`",
        regenerate_command()
    );
}

#[test]
fn source_digests_match_the_summarized_sources() {
    for source in llms::SOURCE_DIGESTS {
        let actual = llms::fnv1a64(source.contents.as_bytes());
        assert_eq!(
            actual, source.hash,
            "source {} changed: review llms.rs digest, then update hash",
            source.path
        );
    }

    let mut paths: Vec<&str> = llms::SOURCE_DIGESTS
        .iter()
        .map(|source| source.path)
        .collect();
    paths.sort_unstable();
    assert_eq!(
        paths,
        [
            "adr/0018-shadowing-callables.md",
            "adr/0020-capability-path-rules.md",
            "adr/0021-runtime-semantics-v01.md",
            "adr/0022-examples-semantics.md",
            "spec/SPEC.md",
            "spec/diagnostics.md",
            "spec/grammar.ebnf",
        ]
    );
}

#[test]
fn cli_llms_rejects_a_file_and_flags() {
    for args in [["llms", "foo.aph"], ["llms", "--json"], ["llms", "-w"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
            .args(args)
            .output()
            .expect("failed to run aleph");
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        assert!(output.stdout.is_empty(), "args: {args:?}");
        assert!(!output.stderr.is_empty(), "args: {args:?}");
    }
}

#[test]
fn render_is_deterministic() {
    assert_eq!(llms::render(), llms::render());
}

#[test]
fn cli_llms_needs_no_file_and_prints_render() {
    let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .arg("llms")
        .output()
        .expect("failed to run aleph");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, llms::render().as_bytes());
}

#[test]
fn llms_txt_stays_within_the_token_budget() {
    let text = llms::render();
    let chars = text.chars().count();
    let tokens = llms::estimated_tokens(&text);
    assert!(
        tokens <= TOKEN_BUDGET,
        "llms.txt is {chars} chars (~{tokens} tokens), over the {TOKEN_BUDGET}-token budget"
    );
}

#[test]
fn exactly_ten_fenced_examples_match_the_tests_aph_sources() {
    let text = llms::render();
    assert_eq!(
        text.matches("```").count(),
        20,
        "expected exactly 10 fenced examples"
    );

    let blocks = fenced_blocks(&text);
    assert_eq!(blocks.len(), 10, "expected exactly 10 fenced examples");
    assert_eq!(blocks.len(), llms::examples().len());
    for (index, (block, expected)) in blocks.iter().zip(llms::examples()).enumerate() {
        assert_eq!(block, expected, "example {index} is not verbatim");
    }
}

#[test]
fn every_example_parses_is_canonical_and_checks_clean() {
    for (index, source) in llms::examples().iter().enumerate() {
        let name = format!("tests/aph/llms-example-{index}.aph");

        let parsed = parser::parse(&name, source);
        assert!(
            parsed.diagnostics.is_empty(),
            "example {index} does not parse: {:?}",
            parsed.diagnostics
        );
        let file = parsed.file.expect("diagnostic-free parse yields a file");

        assert_eq!(
            printer::print_file(&file),
            *source,
            "example {index} is not canonical"
        );

        let checked = check::check(&name, &file);
        assert!(
            checked.diagnostics.is_empty(),
            "example {index} does not type check: {:?}",
            checked.diagnostics
        );

        let capabilities = caps::check_caps(&name, &file);
        assert!(
            capabilities.is_empty(),
            "example {index} fails the capability check: {capabilities:?}"
        );

        let example_diagnostics = examples::check(&name, &file);
        assert!(
            example_diagnostics.is_empty(),
            "example {index} fails the example check: {example_diagnostics:?}"
        );
    }
}
