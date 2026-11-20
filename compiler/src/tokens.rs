//! Token counters for `aleph tokens` (ADR-0031).
//!
//! The built-in counters are local, deterministic approximations. A real BPE
//! counter loads a vocabulary file given by path: a tiktoken vocabulary
//! (`base64token rank`, as in `o200k_base.tiktoken`) or a Hugging Face
//! `tokenizer.json` with a byte-level BPE model. Nothing is downloaded.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::{lexer, llms};

/// A named token counter. `count` returns `None` when the text cannot be
/// tokenized by this counter (the `aleph` counter on a lexical error).
pub struct Tokenizer {
    pub name: String,
    pub method: &'static str,
    pub kind: Kind,
}

pub enum Kind {
    Builtin(fn(&str) -> Option<usize>),
    Bpe(Bpe),
}

impl Tokenizer {
    pub fn count(&self, text: &str) -> Option<usize> {
        match &self.kind {
            Kind::Builtin(count) => (*count)(text),
            Kind::Bpe(bpe) => Some(bpe.count(text)),
        }
    }
}

/// The built-in counters; the first is the default.
pub fn builtins() -> Vec<Tokenizer> {
    vec![
        Tokenizer {
            name: "chars4".to_string(),
            method: "ceil(unicode chars / 4), the usual LLM estimate",
            kind: Kind::Builtin(count_chars4),
        },
        Tokenizer {
            name: "bytes".to_string(),
            method: "utf-8 bytes",
            kind: Kind::Builtin(count_bytes),
        },
        Tokenizer {
            name: "chars".to_string(),
            method: "unicode scalar values",
            kind: Kind::Builtin(count_chars),
        },
        Tokenizer {
            name: "words".to_string(),
            method: "whitespace-separated words",
            kind: Kind::Builtin(count_words),
        },
        Tokenizer {
            name: "aleph".to_string(),
            method: "aleph lexer tokens (SPEC §2)",
            kind: Kind::Builtin(aleph_tokens),
        },
    ]
}

fn count_chars4(text: &str) -> Option<usize> {
    Some(llms::estimated_tokens(text))
}

fn count_bytes(text: &str) -> Option<usize> {
    Some(text.len())
}

fn count_chars(text: &str) -> Option<usize> {
    Some(text.chars().count())
}

fn count_words(text: &str) -> Option<usize> {
    Some(text.split_whitespace().count())
}

fn aleph_tokens(text: &str) -> Option<usize> {
    let (lines, diagnostics) = lexer::lex("", text);
    diagnostics
        .is_empty()
        .then(|| lines.iter().map(|line| line.tokens.len()).sum())
}

/// Load one BPE counter per vocabulary path, named `bpe:<file stem>`.
pub fn load(paths: &[String]) -> Result<Vec<Tokenizer>, String> {
    paths.iter().map(|path| load_one(path)).collect()
}

fn load_one(path: &str) -> Result<Tokenizer, String> {
    let bytes = fs::read(path).map_err(|err| format!("cannot read vocabulary {path}: {err}"))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| format!("vocabulary {path} is not UTF-8 text"))?;
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("vocab");
    let bpe = if text.trim_start().starts_with('{') {
        Bpe::from_hf_json(text).map_err(|err| {
            // Keep the documented message and put the file name where a
            // person reading the error expects it.
            if let Some(pattern) = err.strip_prefix("unsupported pre-tokenizer pattern: ") {
                format!("unsupported pre-tokenizer pattern in {path}: {pattern}")
            } else {
                format!("cannot load vocabulary {path}: {err}")
            }
        })?
    } else {
        Bpe::from_tiktoken(text, stem)
            .map_err(|err| format!("cannot load vocabulary {path}: {err}"))?
    };
    Ok(Tokenizer {
        name: format!("bpe:{stem}"),
        method: bpe.method,
        kind: Kind::Bpe(bpe),
    })
}

/// The counters selected by `--tokenizer` (`None` = default, `all` = every
/// counter), or `None` for an unknown name.
pub fn select<'a>(name: Option<&str>, counters: &'a [Tokenizer]) -> Option<Vec<&'a Tokenizer>> {
    match name {
        None => counters.first().map(|tokenizer| vec![tokenizer]),
        Some("all") => Some(counters.iter().collect()),
        Some(name) => counters
            .iter()
            .find(|tokenizer| tokenizer.name == name)
            .map(|tokenizer| vec![tokenizer]),
    }
}

/// Comma-separated counter names, for usage errors.
pub fn names(counters: &[Tokenizer]) -> String {
    counters
        .iter()
        .map(|tokenizer| tokenizer.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A byte-level BPE vocabulary loaded from a local file.
pub struct Bpe {
    method: &'static str,
    kind: BpeKind,
}

enum BpeKind {
    Tiktoken {
        ranks: HashMap<Vec<u8>, u32>,
        pretokenizer: Pretokenizer,
    },
    HuggingFace {
        vocab: HashMap<String, u32>,
        /// `(left id, right id) -> (merge rank, merged id)`.
        merges: HashMap<(u32, u32), (usize, u32)>,
        /// Byte-level symbols mapped to their vocabulary ids once, at load.
        byte_ids: Box<[u32; 256]>,
        alphabet: Box<[char; 256]>,
        pretokenizer: Pretokenizer,
        add_prefix_space: bool,
        ignore_merges: bool,
    },
}

impl Bpe {
    fn count(&self, text: &str) -> usize {
        match &self.kind {
            BpeKind::Tiktoken {
                ranks,
                pretokenizer,
            } => split(text, *pretokenizer)
                .iter()
                .map(|piece| byte_pair_count(piece.as_bytes(), ranks))
                .sum(),
            BpeKind::HuggingFace {
                vocab,
                merges,
                byte_ids,
                alphabet,
                pretokenizer,
                add_prefix_space,
                ignore_merges,
            } => {
                if text.is_empty() {
                    return 0;
                }
                let prefixed;
                let text = if *add_prefix_space && !text.starts_with(' ') {
                    prefixed = format!(" {text}");
                    prefixed.as_str()
                } else {
                    text
                };
                split(text, *pretokenizer)
                    .iter()
                    .map(|piece| {
                        hf_pair_count(piece, vocab, merges, *ignore_merges, alphabet, byte_ids)
                    })
                    .sum()
            }
        }
    }

    /// Parse a tiktoken vocabulary: one `base64token rank` line per token.
    fn from_tiktoken(text: &str, stem: &str) -> Result<Self, String> {
        let mut ranks = HashMap::new();
        for (number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split_ascii_whitespace();
            let token = fields
                .next()
                .ok_or_else(|| format!("line {}: expected `base64token rank`", number + 1))?;
            let rank = fields
                .next()
                .ok_or_else(|| format!("line {}: expected `base64token rank`", number + 1))?;
            if fields.next().is_some() {
                return Err(format!("line {}: expected `base64token rank`", number + 1));
            }
            let token = base64_decode(token)
                .ok_or_else(|| format!("line {}: invalid base64 token", number + 1))?;
            let rank = rank
                .parse::<u32>()
                .map_err(|_| format!("line {}: invalid rank", number + 1))?;
            ranks.insert(token, rank);
        }
        if ranks.is_empty() {
            return Err("no tokens".to_string());
        }
        // The file format does not say which pre-tokenizer trained the
        // vocabulary: identify it by entry count, not by the file name. Small
        // fixtures (tests) have no meaningful count and must be named after
        // the family they stand in for.
        let pretokenizer = tiktoken_pretokenizer(stem, ranks.len())?;
        Ok(Self {
            method: tiktoken_method(pretokenizer),
            kind: BpeKind::Tiktoken {
                ranks,
                pretokenizer,
            },
        })
    }

    /// Parse a Hugging Face `tokenizer.json` byte-level BPE model.
    fn from_hf_json(text: &str) -> Result<Self, String> {
        let value: Value =
            serde_json::from_str(text).map_err(|err| format!("invalid JSON: {err}"))?;
        check_normalizer(value.get("normalizer"))?;
        let model = value
            .get("model")
            .and_then(Value::as_object)
            .ok_or("missing model")?;
        let model_type = model
            .get("type")
            .and_then(Value::as_str)
            .ok_or("missing model.type")?;
        if model_type != "BPE" {
            return Err(format!("unsupported model type: {model_type}"));
        }
        if model.get("byte_fallback").and_then(Value::as_bool) == Some(true) {
            return Err("byte_fallback is unsupported".to_string());
        }
        let (pretokenizer, add_prefix_space) = pre_tokenizer(value.get("pre_tokenizer"))?;
        let ignore_merges = model
            .get("ignore_merges")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let vocab_object = model
            .get("vocab")
            .and_then(Value::as_object)
            .ok_or("missing model.vocab")?;
        if vocab_object.is_empty() {
            return Err("model.vocab is empty".to_string());
        }
        let mut vocab = HashMap::new();
        for (symbol, id) in vocab_object {
            let id = id
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
                .ok_or_else(|| format!("model.vocab[{symbol:?}] is not a token id"))?;
            vocab.insert(symbol.clone(), id);
        }

        let alphabet = byte_to_unicode();
        let mut byte_ids = [0u32; 256];
        for (byte, character) in alphabet.iter().enumerate() {
            let symbol = character.to_string();
            let id = *vocab.get(&symbol).ok_or_else(|| {
                format!(
                    "model.vocab lacks byte-level symbol U+{:04X}",
                    *character as u32
                )
            })?;
            byte_ids[byte] = id;
        }

        let merge_list = model
            .get("merges")
            .and_then(Value::as_array)
            .ok_or("missing model.merges")?;
        let mut merges = HashMap::new();
        for (index, merge) in merge_list.iter().enumerate() {
            let (left, right) = parse_merge(merge)
                .ok_or_else(|| format!("merge {index}: expected \"a b\" or [a, b]"))?;
            let left_id = *vocab
                .get(&left)
                .ok_or_else(|| format!("merge {index}: symbol {left:?} is not in model.vocab"))?;
            let right_id = *vocab
                .get(&right)
                .ok_or_else(|| format!("merge {index}: symbol {right:?} is not in model.vocab"))?;
            let merged = format!("{left}{right}");
            let merged_id = *vocab.get(&merged).ok_or_else(|| {
                format!("merge {index}: merged token {merged:?} is not in model.vocab")
            })?;
            // The first occurrence wins, as in the reference implementation.
            merges
                .entry((left_id, right_id))
                .or_insert((index, merged_id));
        }
        Ok(Self {
            method: hf_method(pretokenizer),
            kind: BpeKind::HuggingFace {
                vocab,
                merges,
                byte_ids: Box::new(byte_ids),
                alphabet: Box::new(alphabet),
                pretokenizer,
                add_prefix_space,
                ignore_merges,
            },
        })
    }
}

fn tiktoken_pretokenizer(stem: &str, entries: usize) -> Result<Pretokenizer, String> {
    let named = family_from_stem(stem);
    if entries < 1000 {
        return named.ok_or_else(|| {
            format!(
                "cannot identify tokenizer vocabulary from {entries} entries: name the file o200k_base, cl100k_base, r50k_base or p50k_base"
            )
        });
    }
    let sized = family_from_size(entries)
        .ok_or_else(|| format!("cannot identify tokenizer vocabulary from {entries} entries"))?;
    if let Some(named) = named {
        if named != sized {
            return Err(format!(
                "vocabulary has {entries} entries (looks like {}) but its file name says {}",
                sized.label(),
                named.label()
            ));
        }
    }
    Ok(sized)
}

fn family_from_stem(stem: &str) -> Option<Pretokenizer> {
    let lower = stem.to_ascii_lowercase();
    if lower.contains("o200k") {
        Some(Pretokenizer::O200k)
    } else if lower.contains("cl100k") {
        Some(Pretokenizer::Cl100k)
    } else if lower.contains("r50k") || lower.contains("p50k") {
        Some(Pretokenizer::Gpt2)
    } else {
        None
    }
}

fn family_from_size(entries: usize) -> Option<Pretokenizer> {
    match entries {
        150_000..=250_000 => Some(Pretokenizer::O200k),
        75_000..=149_999 => Some(Pretokenizer::Cl100k),
        40_000..=74_999 => Some(Pretokenizer::Gpt2),
        _ => None,
    }
}

fn tiktoken_method(pretokenizer: Pretokenizer) -> &'static str {
    match pretokenizer {
        Pretokenizer::O200k => "tiktoken BPE, o200k pre-tokenizer (approximate)",
        Pretokenizer::Cl100k => "tiktoken BPE, cl100k pre-tokenizer (approximate)",
        Pretokenizer::Gpt2 => "tiktoken BPE, GPT-2 (r50k/p50k) pre-tokenizer (approximate)",
        Pretokenizer::Qwen2 => "tiktoken BPE, Qwen2 pre-tokenizer (approximate)",
    }
}

fn hf_method(pretokenizer: Pretokenizer) -> &'static str {
    match pretokenizer {
        Pretokenizer::O200k => "hugging face byte-level BPE, o200k pre-tokenizer (approximate)",
        Pretokenizer::Cl100k => "hugging face byte-level BPE, cl100k pre-tokenizer (approximate)",
        Pretokenizer::Gpt2 => "hugging face byte-level BPE, GPT-2 pre-tokenizer (approximate)",
        Pretokenizer::Qwen2 => "hugging face byte-level BPE, Qwen2 pre-tokenizer (approximate)",
    }
}

fn parse_merge(value: &Value) -> Option<(String, String)> {
    match value {
        Value::String(text) => {
            let mut parts = text.split(' ');
            let left = parts.next()?;
            let right = parts.next()?;
            if parts.next().is_some() {
                return None;
            }
            Some((left.to_string(), right.to_string()))
        }
        Value::Array(items) => {
            if items.len() != 2 {
                return None;
            }
            Some((
                items[0].as_str()?.to_string(),
                items[1].as_str()?.to_string(),
            ))
        }
        _ => None,
    }
}

/// The `normalizer` is either absent, NFC, or a sequence of NFC. NFC is not
/// applied: the vocabularies we accept do not need it for ASCII code and the
/// difference is documented in ADR-0031.
fn check_normalizer(value: Option<&Value>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let object = value.as_object().ok_or("invalid normalizer")?;
    match object.get("type").and_then(Value::as_str) {
        Some("NFC") => Ok(()),
        Some("Sequence") => {
            let items = object
                .get("normalizers")
                .and_then(Value::as_array)
                .ok_or("normalizer Sequence without normalizers")?;
            for item in items {
                check_normalizer(Some(item))?;
            }
            Ok(())
        }
        Some(other) => Err(format!("unsupported normalizer: {other}")),
        None => Err("normalizer without type".to_string()),
    }
}

const CL100K_PATTERN: &str = r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";
const O200K_PATTERN: &str = r"[^\r\n\p{L}\p{N}]?[\p{Lu}\p{Lt}\p{Lm}\p{Lo}\p{M}]*[\p{Ll}\p{Lm}\p{Lo}\p{M}]+(?i:'s|'t|'re|'ve|'m|'ll|'d)?|[^\r\n\p{L}\p{N}]?[\p{Lu}\p{Lt}\p{Lm}\p{Lo}\p{M}]+[\p{Ll}\p{Lm}\p{Lo}\p{M}]*(?i:'s|'t|'re|'ve|'m|'ll|'d)?|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n/]*|\s*[\r\n]+|\s+(?!\S)|\s+";
const GPT2_PATTERN: &str =
    r"'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+";
const QWEN2_PATTERN: &str = r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";

fn pretokenizer_from_pattern(pattern: &str) -> Option<Pretokenizer> {
    match pattern {
        CL100K_PATTERN => Some(Pretokenizer::Cl100k),
        O200K_PATTERN => Some(Pretokenizer::O200k),
        GPT2_PATTERN => Some(Pretokenizer::Gpt2),
        QWEN2_PATTERN => Some(Pretokenizer::Qwen2),
        _ => None,
    }
}

/// Parse the pre-tokenizer config into the regex to split with and whether to
/// prepend a space. Any configuration we cannot reproduce exactly is an
/// error: treating a whole file as one piece would give silently wrong counts.
fn pre_tokenizer(value: Option<&Value>) -> Result<(Pretokenizer, bool), String> {
    let object = value
        .and_then(Value::as_object)
        .ok_or("missing ByteLevel pre-tokenizer")?;
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or("pre-tokenizer without type")?;
    match kind {
        "ByteLevel" => {
            let use_regex = object
                .get("use_regex")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let add_prefix_space = object
                .get("add_prefix_space")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !use_regex {
                return Err(
                    "byte-level pre-tokenizer without a Split regex would treat the whole file as one piece"
                        .to_string(),
                );
            }
            Ok((Pretokenizer::Gpt2, add_prefix_space))
        }
        "Sequence" => {
            let items = object
                .get("pretokenizers")
                .and_then(Value::as_array)
                .ok_or("Sequence pre-tokenizer without pretokenizers")?;
            let splits = items
                .iter()
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("Split"))
                .count();
            if splits > 1 {
                return Err(
                    "chained Split pre-tokenizers (e.g. DeepSeek) are not supported".to_string(),
                );
            }
            let mut pattern = None;
            let mut byte_level = None;
            for item in items {
                let item = item
                    .as_object()
                    .ok_or("pre-tokenizer component is not an object")?;
                let component = item
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or("pre-tokenizer component without type")?;
                match component {
                    "Split" => {
                        if byte_level.is_some() {
                            return Err("Split after ByteLevel is unsupported".to_string());
                        }
                        if pattern.is_some() {
                            return Err(
                                "chained Split pre-tokenizers (e.g. DeepSeek) are not supported"
                                    .to_string(),
                            );
                        }
                        pattern = Some(split_pattern(item)?);
                    }
                    "ByteLevel" => {
                        if byte_level.is_some() {
                            return Err("multiple ByteLevel pre-tokenizers".to_string());
                        }
                        let use_regex = item
                            .get("use_regex")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        let add_prefix_space = item
                            .get("add_prefix_space")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        byte_level = Some((use_regex, add_prefix_space));
                    }
                    other => {
                        return Err(format!("unsupported pre-tokenizer component: {other}"));
                    }
                }
            }
            let pattern = pattern.ok_or(
                "byte-level pre-tokenizer without a Split regex would treat the whole file as one piece",
            )?;
            let (use_regex, add_prefix_space) =
                byte_level.ok_or("missing ByteLevel pre-tokenizer")?;
            if add_prefix_space {
                return Err(
                    "add_prefix_space is not supported inside a pre-tokenizer Sequence".to_string(),
                );
            }
            if use_regex {
                return Err("byte-level pre-tokenizer uses its own regex after a Split".to_string());
            }
            Ok((pattern, add_prefix_space))
        }
        other => Err(format!("unsupported pre-tokenizer: {other}")),
    }
}

fn split_pattern(object: &serde_json::Map<String, Value>) -> Result<Pretokenizer, String> {
    let behavior = object.get("behavior").and_then(Value::as_str);
    if behavior != Some("Isolated") {
        return Err(format!(
            "unsupported Split behavior: {}",
            behavior.unwrap_or("<missing>")
        ));
    }
    if object.get("invert").and_then(Value::as_bool) == Some(true) {
        return Err("unsupported Split invert: true".to_string());
    }
    let pattern_value = object.get("pattern").ok_or("Split without pattern")?;
    let pattern = match pattern_value {
        Value::Object(map) => map
            .get("Regex")
            .and_then(Value::as_str)
            .ok_or("Split pattern is not a Regex")?,
        _ => return Err("Split pattern is not a Regex".to_string()),
    };
    pretokenizer_from_pattern(pattern).ok_or_else(|| {
        let first: String = pattern.chars().take(80).collect();
        format!("unsupported pre-tokenizer pattern: {first}")
    })
}

/// Standard base64 with optional padding; rejects truncated tokens.
fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u32;
    let mut padding = false;
    for byte in text.bytes() {
        if byte == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return None;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    // A base64 group cannot leave one dangling character, and an empty token
    // is not a vocabulary entry.
    if out.is_empty() || text.len() % 4 == 1 || bits >= 6 {
        None
    } else {
        Some(out)
    }
}

/// Standard tiktoken merge: repeatedly merge the adjacent pair whose merged
/// token has the lowest rank. Missing single-byte tokens simply stay apart.
///
/// Parts are byte offsets with a cached rank per adjacent pair, as in
/// tiktoken's `_byte_pair_merge`: only the merged part and its predecessor
/// change after a merge, so only their ranks are recomputed. A binary heap
/// keeps finding the minimum cheap even for long pieces.
fn byte_pair_count(piece: &[u8], ranks: &HashMap<Vec<u8>, u32>) -> usize {
    const NO_RANK: u32 = u32::MAX;
    const NO_PART: usize = usize::MAX;
    if piece.is_empty() {
        return 0;
    }
    if ranks.contains_key(piece) {
        return 1;
    }
    let len = piece.len();
    // Part `i` starts at byte `i` and never moves; `next` skips merged parts.
    let mut next: Vec<usize> = (1..=len).collect();
    next[len - 1] = NO_PART;
    let mut prev: Vec<usize> = (0..len).map(|i| i.wrapping_sub(1)).collect();
    let mut rank = vec![NO_RANK; len];
    let mut heap = BinaryHeap::new();
    for i in 0..len - 1 {
        let end = if next[i + 1] == NO_PART {
            len
        } else {
            next[i + 1]
        };
        rank[i] = ranks.get(&piece[i..end]).copied().unwrap_or(NO_RANK);
        if rank[i] != NO_RANK {
            heap.push(Reverse((rank[i], i)));
        }
    }
    let mut parts = len;
    while let Some(Reverse((r, i))) = heap.pop() {
        // Entries left behind by earlier merges no longer match `rank`.
        if next[i] == NO_PART || rank[i] != r {
            continue;
        }
        let merged = next[i];
        let after = next[merged];
        next[merged] = NO_PART;
        next[i] = after;
        if after != NO_PART {
            prev[after] = i;
        }
        rank[i] = if after == NO_PART {
            NO_RANK
        } else {
            let end = if next[after] == NO_PART {
                len
            } else {
                next[after]
            };
            ranks.get(&piece[i..end]).copied().unwrap_or(NO_RANK)
        };
        if rank[i] != NO_RANK {
            heap.push(Reverse((rank[i], i)));
        }
        let before = prev[i];
        if before != NO_PART {
            let end = if next[i] == NO_PART { len } else { next[i] };
            rank[before] = ranks.get(&piece[before..end]).copied().unwrap_or(NO_RANK);
            if rank[before] != NO_RANK {
                heap.push(Reverse((rank[before], before)));
            }
        }
        parts -= 1;
    }
    parts
}

/// Hugging Face byte-level BPE over the UTF-8 bytes of a pre-tokenized piece.
/// Symbols are vocabulary ids, so merging is a `Vec<u32>` of ids with no
/// per-pair strings. It uses the same offset/cached-rank merge as tiktoken.
fn hf_pair_count(
    piece: &str,
    vocab: &HashMap<String, u32>,
    merges: &HashMap<(u32, u32), (usize, u32)>,
    ignore_merges: bool,
    alphabet: &[char; 256],
    byte_ids: &[u32; 256],
) -> usize {
    const NO_RANK: usize = usize::MAX;
    const NO_PART: usize = usize::MAX;
    let len = piece.len();
    if len == 0 {
        return 0;
    }
    let mut symbols: Vec<u32> = piece
        .as_bytes()
        .iter()
        .map(|byte| byte_ids[*byte as usize])
        .collect();
    if ignore_merges {
        let mapped: String = piece
            .as_bytes()
            .iter()
            .map(|byte| alphabet[*byte as usize])
            .collect();
        if vocab.contains_key(&mapped) {
            return 1;
        }
    }
    let mut next: Vec<usize> = (1..=len).collect();
    next[len - 1] = NO_PART;
    let mut prev: Vec<usize> = (0..len).map(|i| i.wrapping_sub(1)).collect();
    let mut rank = vec![NO_RANK; len];
    let mut heap = BinaryHeap::new();
    for i in 0..len - 1 {
        if let Some(&(r, _)) = merges.get(&(symbols[i], symbols[i + 1])) {
            rank[i] = r;
            heap.push(Reverse((r, i)));
        }
    }
    let mut parts = len;
    while let Some(Reverse((r, i))) = heap.pop() {
        if next[i] == NO_PART || rank[i] != r {
            continue;
        }
        let merged = next[i];
        let after = next[merged];
        symbols[i] = merges[&(symbols[i], symbols[merged])].1;
        next[merged] = NO_PART;
        next[i] = after;
        if after != NO_PART {
            prev[after] = i;
        }
        rank[i] = if after == NO_PART {
            NO_RANK
        } else {
            merges
                .get(&(symbols[i], symbols[after]))
                .map_or(NO_RANK, |&(rank, _)| rank)
        };
        if rank[i] != NO_RANK {
            heap.push(Reverse((rank[i], i)));
        }
        let before = prev[i];
        if before != NO_PART {
            rank[before] = merges
                .get(&(symbols[before], symbols[i]))
                .map_or(NO_RANK, |&(rank, _)| rank);
            if rank[before] != NO_RANK {
                heap.push(Reverse((rank[before], before)));
            }
        }
        parts -= 1;
    }
    parts
}

/// GPT-2's byte-to-unicode table: printable Latin-1 maps to itself, the rest
/// to the code points from U+0100 up, in byte order.
fn byte_to_unicode() -> [char; 256] {
    let mut alphabet = ['\0'; 256];
    let mut next = 256u32;
    for byte in 0..=255u16 {
        let mapped = (0x21..=0x7e).contains(&byte)
            || (0xa1..=0xac).contains(&byte)
            || (0xae..=0xff).contains(&byte);
        let code = if mapped {
            u32::from(byte)
        } else {
            let code = next;
            next += 1;
            code
        };
        alphabet[byte as usize] = char::from_u32(code).expect("valid scalar value");
    }
    alphabet
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pretokenizer {
    Cl100k,
    O200k,
    Gpt2,
    Qwen2,
}

impl Pretokenizer {
    fn label(self) -> &'static str {
        match self {
            Pretokenizer::Cl100k => "cl100k",
            Pretokenizer::O200k => "o200k",
            Pretokenizer::Gpt2 => "GPT-2 (r50k/p50k)",
            Pretokenizer::Qwen2 => "Qwen2",
        }
    }
}

/// The cl100k/o200k/GPT-2/Qwen2 pre-tokenizer regexes, by hand. Unicode
/// categories are approximated with `char::is_alphabetic`/`is_numeric`/
/// `is_whitespace`: this reproduces their behaviour for ordinary text but not
/// for every combining mark or unusual numeric character.
fn split(text: &str, pretokenizer: Pretokenizer) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut pieces = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let len = match_at(&chars, start, pretokenizer);
        let end = start + len;
        let end_byte = chars.get(end).map_or(text.len(), |(offset, _)| *offset);
        pieces.push(&text[chars[start].0..end_byte]);
        start = end;
    }
    pieces
}

fn match_at(chars: &[(usize, char)], start: usize, pretokenizer: Pretokenizer) -> usize {
    let matched = match pretokenizer {
        Pretokenizer::Cl100k => match_cl100k(chars, start),
        Pretokenizer::O200k => match_o200k(chars, start),
        Pretokenizer::Gpt2 => match_gpt2(chars, start),
        Pretokenizer::Qwen2 => match_qwen2(chars, start),
    };
    // Every character belongs to one alternative below.
    matched.unwrap_or(1)
}

fn match_cl100k(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match_contraction(chars, start, true)
        .or_else(|| match_word_cl100k(chars, start))
        .or_else(|| match_digits(chars, start))
        .or_else(|| match_punctuation(chars, start, false))
        .or_else(|| match_whitespace(chars, start))
}

fn match_qwen2(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match_contraction(chars, start, true)
        .or_else(|| match_word_cl100k(chars, start))
        .or_else(|| match_single_number(chars, start))
        .or_else(|| match_punctuation(chars, start, false))
        .or_else(|| match_whitespace(chars, start))
}

fn match_o200k(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match_word_o200k(chars, start)
        .or_else(|| match_digits(chars, start))
        .or_else(|| match_punctuation(chars, start, true))
        .or_else(|| match_whitespace(chars, start))
}

fn match_gpt2(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match_contraction(chars, start, false)
        .or_else(|| match_optional_letters(chars, start))
        .or_else(|| match_optional_numbers(chars, start))
        .or_else(|| match_punctuation_run(chars, start))
        .or_else(|| match_whitespace_gpt2(chars, start))
}

fn char_at(chars: &[(usize, char)], index: usize) -> Option<char> {
    chars.get(index).map(|(_, c)| *c)
}

fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

fn is_number(c: char) -> bool {
    c.is_numeric()
}

fn is_newline(c: char) -> bool {
    c == '\r' || c == '\n'
}

fn is_prefix(c: char) -> bool {
    !is_newline(c) && !is_letter(c) && !is_number(c)
}

fn is_punctuation(c: char) -> bool {
    !c.is_whitespace() && !is_letter(c) && !is_number(c)
}

/// `Lm`/`Lo` (and marks that Rust reports as alphabetic) belong to both
/// o200k character classes. Approximating "other letter" this way is what
/// ADR-0031 documents.
fn is_other_letter(c: char) -> bool {
    c.is_alphabetic() && !c.is_uppercase() && !c.is_lowercase()
}

fn is_upper_like(c: char) -> bool {
    c.is_uppercase() || is_other_letter(c)
}

fn is_lower_like(c: char) -> bool {
    c.is_lowercase() || is_other_letter(c)
}

fn contraction_length(text: &str) -> usize {
    text.chars().count()
}

fn match_contraction(chars: &[(usize, char)], start: usize, ignore_case: bool) -> Option<usize> {
    for contraction in ["'s", "'t", "'re", "'ve", "'m", "'ll", "'d"] {
        let matches = contraction.chars().enumerate().all(|(offset, expected)| {
            char_at(chars, start + offset).is_some_and(|actual| {
                if ignore_case {
                    actual.eq_ignore_ascii_case(&expected)
                } else {
                    actual == expected
                }
            })
        });
        if matches {
            return Some(contraction_length(contraction));
        }
    }
    None
}

fn match_word_cl100k(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut end = start;
    match char_at(chars, start) {
        Some(c) if is_letter(c) => {}
        Some(c) if is_prefix(c) => end = start + 1,
        _ => return None,
    }
    if !char_at(chars, end).is_some_and(is_letter) {
        return None;
    }
    while char_at(chars, end).is_some_and(is_letter) {
        end += 1;
    }
    Some(end - start)
}

fn match_word_o200k(chars: &[(usize, char)], start: usize) -> Option<usize> {
    match_word_o200k_lower_first(chars, start)
        .or_else(|| match_word_o200k_upper_first(chars, start))
}

/// First o200k word alternative: prefix? upper* lower+ contraction?
fn match_word_o200k_lower_first(chars: &[(usize, char)], start: usize) -> Option<usize> {
    for prefix in [1usize, 0] {
        if prefix == 1 && !char_at(chars, start).is_some_and(is_prefix) {
            continue;
        }
        let upper_start = start + prefix;
        let mut upper_end = upper_start;
        while char_at(chars, upper_end).is_some_and(is_upper_like) {
            upper_end += 1;
        }
        // Greedy `upper*` backtracks one character at a time until `lower+`
        // can match; `Lm`/`Lo` characters are in both classes.
        let mut u_end = upper_end;
        loop {
            let mut end = u_end;
            let lower_start = end;
            while char_at(chars, end).is_some_and(is_lower_like) {
                end += 1;
            }
            if end > lower_start {
                if let Some(length) = match_contraction(chars, end, true) {
                    end += length;
                }
                return Some(end - start);
            }
            if u_end == upper_start {
                break;
            }
            u_end -= 1;
        }
    }
    None
}

/// Second o200k word alternative: prefix? upper+ lower* contraction?
fn match_word_o200k_upper_first(chars: &[(usize, char)], start: usize) -> Option<usize> {
    for prefix in [1usize, 0] {
        if prefix == 1 && !char_at(chars, start).is_some_and(is_prefix) {
            continue;
        }
        let mut end = start + prefix;
        let upper_start = end;
        while char_at(chars, end).is_some_and(is_upper_like) {
            end += 1;
        }
        if end == upper_start {
            continue;
        }
        while char_at(chars, end).is_some_and(is_lower_like) {
            end += 1;
        }
        if let Some(length) = match_contraction(chars, end, true) {
            end += length;
        }
        return Some(end - start);
    }
    None
}

fn match_single_number(chars: &[(usize, char)], start: usize) -> Option<usize> {
    char_at(chars, start).is_some_and(is_number).then_some(1)
}

fn match_digits(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut end = start;
    while end - start < 3 && char_at(chars, end).is_some_and(is_number) {
        end += 1;
    }
    (end > start).then_some(end - start)
}

fn match_optional_letters(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut end = start;
    if char_at(chars, start) == Some(' ') && char_at(chars, start + 1).is_some_and(is_letter) {
        end = start + 1;
    }
    if !char_at(chars, end).is_some_and(is_letter) {
        return None;
    }
    while char_at(chars, end).is_some_and(is_letter) {
        end += 1;
    }
    Some(end - start)
}

fn match_optional_numbers(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut end = start;
    if char_at(chars, start) == Some(' ') && char_at(chars, start + 1).is_some_and(is_number) {
        end = start + 1;
    }
    if !char_at(chars, end).is_some_and(is_number) {
        return None;
    }
    while char_at(chars, end).is_some_and(is_number) {
        end += 1;
    }
    Some(end - start)
}

fn match_punctuation_run(chars: &[(usize, char)], start: usize) -> Option<usize> {
    let mut end = start;
    if char_at(chars, start) == Some(' ') && char_at(chars, start + 1).is_some_and(is_punctuation) {
        end = start + 1;
    }
    if !char_at(chars, end).is_some_and(is_punctuation) {
        return None;
    }
    while char_at(chars, end).is_some_and(is_punctuation) {
        end += 1;
    }
    Some(end - start)
}

fn match_punctuation(chars: &[(usize, char)], start: usize, trailing_slash: bool) -> Option<usize> {
    let mut end = start + match_punctuation_run(chars, start)?;
    while let Some(c) = char_at(chars, end) {
        if is_newline(c) || (trailing_slash && c == '/') {
            end += 1;
        } else {
            break;
        }
    }
    Some(end - start)
}

fn match_whitespace(chars: &[(usize, char)], start: usize) -> Option<usize> {
    if !char_at(chars, start).is_some_and(char::is_whitespace) {
        return None;
    }
    let mut end = start;
    let mut last_newline = None;
    while let Some(c) = char_at(chars, end) {
        if !c.is_whitespace() {
            break;
        }
        if is_newline(c) {
            last_newline = Some(end);
        }
        end += 1;
    }
    if let Some(newline) = last_newline {
        return Some(newline - start + 1);
    }
    whitespace_run_end(chars, start, end)
}

fn match_whitespace_gpt2(chars: &[(usize, char)], start: usize) -> Option<usize> {
    if !char_at(chars, start).is_some_and(char::is_whitespace) {
        return None;
    }
    let mut end = start;
    while char_at(chars, end).is_some_and(char::is_whitespace) {
        end += 1;
    }
    whitespace_run_end(chars, start, end)
}

fn whitespace_run_end(chars: &[(usize, char)], start: usize, end: usize) -> Option<usize> {
    if end == chars.len() {
        Some(end - start)
    } else if end - start >= 2 {
        // `\s+(?!\S)` matches all but the last whitespace before non-space.
        Some(end - start - 1)
    } else {
        Some(end - start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(name: &str, text: &str) -> Option<usize> {
        let counters = builtins();
        select(Some(name), &counters).expect("known")[0].count(text)
    }

    #[test]
    fn counters_measure_what_they_say() {
        let text = "fn main -> unit\n print \"hé\"\n";
        assert_eq!(count("bytes", text), Some(text.len()));
        assert_eq!(count("chars", text), Some(text.len() - 1));
        assert_eq!(count("chars4", text), Some((text.len() - 1).div_ceil(4)));
        assert_eq!(count("words", text), Some(6));
        // fn main -> unit / print "hé"
        assert_eq!(count("aleph", text), Some(6));
        assert_eq!(count("aleph", "fn main -> unit\n print $\n"), None);
    }

    #[test]
    fn selection() {
        let counters = builtins();
        assert_eq!(select(None, &counters).unwrap()[0].name, "chars4");
        assert_eq!(
            select(Some("all"), &counters).unwrap().len(),
            counters.len()
        );
        assert!(select(Some("gpt4"), &counters).is_none());
        assert_eq!(names(&counters), "chars4, bytes, chars, words, aleph");
    }

    fn base64_encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let a = u32::from(chunk[0]) << 16
                | u32::from(chunk.get(1).copied().unwrap_or(0)) << 8
                | u32::from(chunk.get(2).copied().unwrap_or(0));
            out.push(ALPHABET[((a >> 18) & 63) as usize] as char);
            out.push(ALPHABET[((a >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                out.push(ALPHABET[((a >> 6) & 63) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(a & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    fn tiktoken_vocab(merges: &[(&[u8], u32)]) -> String {
        let mut entries: Vec<(Vec<u8>, u32)> = (0..=255u16)
            .map(|byte| (vec![byte as u8], 1000 + u32::from(byte)))
            .collect();
        entries.extend(merges.iter().map(|(token, rank)| (token.to_vec(), *rank)));
        entries.sort_by_key(|(_, rank)| *rank);
        entries
            .iter()
            .map(|(token, rank)| format!("{} {rank}", base64_encode(token)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A vocabulary with `n` distinct entries, for size-based identification.
    fn tiktoken_vocab_of_size(n: usize) -> String {
        (0..n)
            .map(|i| {
                let token = base64_encode(&(i as u32).to_le_bytes());
                format!("{token} {i}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn tiktoken_counts_bytes_merges_and_pre_tokens() {
        let vocab = tiktoken_vocab(&[
            (b" b".as_slice(), 1),
            (b"12".as_slice(), 2),
            (b"123".as_slice(), 3),
            ("é".as_bytes(), 4),
        ]);
        let bpe = Bpe::from_tiktoken(&vocab, "o200k_base").expect("valid vocabulary");
        assert!(bpe.method.contains("o200k"));
        assert_eq!(bpe.count("a"), 1);
        assert_eq!(bpe.count("a b"), 2);
        // o200k splits number runs as 1-3 digits: "123" then "4".
        assert_eq!(bpe.count("1234"), 2);
        // a / " b" / newline / "123" / 4 / "!é" / tab
        assert_eq!(bpe.count("a b\n1234!é\t"), 8);
        assert_eq!(bpe.count(""), 0);

        let cl = Bpe::from_tiktoken(&tiktoken_vocab(&[]), "cl100k_base").unwrap();
        assert!(cl.method.contains("cl100k"));
    }

    #[test]
    fn tiktoken_family_comes_from_entry_count() {
        let cl_text = tiktoken_vocab_of_size(100_100);
        let cl = Bpe::from_tiktoken(&cl_text, "cl100k_base").expect("matching name");
        assert!(cl.method.contains("cl100k"));
        // The content decides even when the name says nothing.
        let unnamed = Bpe::from_tiktoken(&cl_text, "some_vocab").expect("content decides");
        assert!(unnamed.method.contains("cl100k"));
        // A real-sized file whose name disagrees with its size is an error.
        let error = Bpe::from_tiktoken(&cl_text, "o200k_base")
            .err()
            .expect("mismatch");
        assert!(error.contains("file name says"), "{error}");

        let r_text = tiktoken_vocab_of_size(50_000);
        let r = Bpe::from_tiktoken(&r_text, "r50k_base").expect("matching name");
        assert!(r.method.contains("GPT-2"));
        let error = Bpe::from_tiktoken(&r_text, "cl100k_base")
            .err()
            .expect("mismatch");
        assert!(error.contains("file name says"), "{error}");

        // Neither fixture-sized nor a known real vocabulary.
        let error = Bpe::from_tiktoken(&tiktoken_vocab_of_size(1234), "o200k_base")
            .err()
            .expect("unknown size");
        assert!(error.contains("cannot identify"), "{error}");
        // Small fixtures must be named after a family.
        let error = Bpe::from_tiktoken(&tiktoken_vocab(&[]), "tiny")
            .err()
            .expect("unidentified");
        assert!(error.contains("name the file"), "{error}");
    }

    #[test]
    fn pre_tokenizers_approximate_the_regexes() {
        assert_eq!(
            split("a b\n1234!é\t", Pretokenizer::O200k),
            ["a", " b", "\n", "123", "4", "!é", "\t"]
        );
        assert_eq!(split("it's", Pretokenizer::Cl100k), ["it", "'s"]);
        assert_eq!(split("it's", Pretokenizer::O200k), ["it's"]);
        assert_eq!(split("abc world", Pretokenizer::Gpt2), ["abc", " world"]);
        assert_eq!(split("   x", Pretokenizer::O200k), ["  ", " x"]);
        assert_eq!(
            split("\n  \n  x", Pretokenizer::O200k),
            ["\n  \n", " ", " x"]
        );
    }

    #[test]
    fn o200k_words_follow_case_alternatives() {
        assert_eq!(
            split("getUserName", Pretokenizer::O200k),
            ["get", "User", "Name"]
        );
        assert_eq!(split(" getUser", Pretokenizer::O200k), [" get", "User"]);
        assert_eq!(
            split("XMLHttpRequest", Pretokenizer::O200k),
            ["XMLHttp", "Request"]
        );
        assert_eq!(split("getUSER", Pretokenizer::O200k), ["get", "USER"]);
        assert_eq!(split("HTTPServer", Pretokenizer::O200k), ["HTTPServer"]);
        assert_eq!(split("snake_case", Pretokenizer::O200k), ["snake", "_case"]);
        // cl100k does not split camelCase and keeps contractions apart.
        assert_eq!(split("getUserName", Pretokenizer::Cl100k), ["getUserName"]);
        assert_eq!(split("don't", Pretokenizer::Cl100k), ["don", "'t"]);
        assert_eq!(split("DON'T", Pretokenizer::Cl100k), ["DON", "'T"]);
    }

    #[test]
    fn contractions_attach_in_o200k_and_gpt2_is_lowercase_only() {
        assert_eq!(split("don't", Pretokenizer::O200k), ["don't"]);
        assert_eq!(split("DON'T", Pretokenizer::O200k), ["DON'T"]);
        assert_eq!(split("don't", Pretokenizer::Gpt2), ["don", "'t"]);
        assert_eq!(split("'S", Pretokenizer::Gpt2), ["'", "S"]);
    }

    #[test]
    fn qwen2_counts_one_digit_at_a_time() {
        assert_eq!(split("1234", Pretokenizer::Qwen2), ["1", "2", "3", "4"]);
        assert_eq!(split("1234", Pretokenizer::Cl100k), ["123", "4"]);
    }

    fn mapped(bytes: &[u8]) -> String {
        let alphabet = byte_to_unicode();
        bytes.iter().map(|byte| alphabet[*byte as usize]).collect()
    }

    fn hf_document(merges: &[(String, String)], with_added: bool) -> String {
        hf_document_with(
            merges,
            with_added,
            &serde_json::json!({}),
            serde_json::json!({"type": "ByteLevel"}),
            None,
            &[],
        )
    }

    fn hf_document_with(
        merges: &[(String, String)],
        with_added: bool,
        model_extra: &Value,
        pre_tokenizer: Value,
        normalizer: Option<Value>,
        extra_vocab: &[String],
    ) -> String {
        let alphabet = byte_to_unicode();
        let mut vocab = serde_json::Map::new();
        for (byte, character) in alphabet.iter().enumerate() {
            vocab.insert(character.to_string(), Value::from(byte));
        }
        let mut encoded = Vec::new();
        for (next_id, (index, (left, right))) in (256u64..).zip(merges.iter().enumerate()) {
            let merged = format!("{left}{right}");
            vocab.entry(merged).or_insert(Value::from(next_id));
            if index % 2 == 0 {
                encoded.push(Value::Array(vec![
                    Value::String(left.clone()),
                    Value::String(right.clone()),
                ]));
            } else {
                encoded.push(Value::String(format!("{left} {right}")));
            }
        }
        for (next_id, symbol) in (256 + merges.len() as u64..).zip(extra_vocab.iter()) {
            vocab.entry(symbol.clone()).or_insert(Value::from(next_id));
        }
        let mut model = serde_json::json!({
            "type": "BPE",
            "vocab": vocab,
            "merges": encoded,
        });
        if let (Some(target), Some(extra)) = (model.as_object_mut(), model_extra.as_object()) {
            for (key, value) in extra {
                target.insert(key.clone(), value.clone());
            }
        }
        let mut document = serde_json::json!({
            "model": model,
            "pre_tokenizer": pre_tokenizer,
        });
        if let Some(normalizer) = normalizer {
            document["normalizer"] = normalizer;
        }
        if with_added {
            document["added_tokens"] =
                serde_json::json!([{"id": 0, "content": "<|end|>", "special": true}]);
        }
        document.to_string()
    }

    fn hf_merges() -> Vec<(String, String)> {
        vec![
            ("a".to_string(), "b".to_string()),
            ("ab".to_string(), "c".to_string()),
            (mapped(b" "), "w".to_string()),
            (mapped(b" w"), "o".to_string()),
            (mapped(b" wo"), "r".to_string()),
            (mapped(b" wor"), "l".to_string()),
            (mapped(b" worl"), "d".to_string()),
        ]
    }

    /// The old loop, kept as a reference for small inputs.
    fn byte_pair_count_naive(piece: &[u8], ranks: &HashMap<Vec<u8>, u32>) -> usize {
        if ranks.contains_key(piece) {
            return 1;
        }
        let mut parts: Vec<Vec<u8>> = piece.iter().map(|byte| vec![*byte]).collect();
        while parts.len() > 1 {
            let mut best: Option<(u32, usize)> = None;
            for i in 0..parts.len() - 1 {
                let mut merged = Vec::with_capacity(parts[i].len() + parts[i + 1].len());
                merged.extend_from_slice(&parts[i]);
                merged.extend_from_slice(&parts[i + 1]);
                if let Some(&rank) = ranks.get(&merged) {
                    if best.is_none_or(|(lowest, _)| rank < lowest) {
                        best = Some((rank, i));
                    }
                }
            }
            let Some((_, i)) = best else { break };
            let right = parts.remove(i + 1);
            parts[i].extend_from_slice(&right);
        }
        parts.len()
    }

    #[test]
    fn byte_pair_merge_matches_the_naive_loop_on_small_inputs() {
        let mut ranks = HashMap::new();
        for (token, rank) in [
            (b"ab".as_slice(), 0u32),
            (b"abc".as_slice(), 1),
            (b"bc".as_slice(), 2),
            (b"aa".as_slice(), 3),
            (b"  ".as_slice(), 4),
            (b" \n".as_slice(), 5),
            (b"\n\n".as_slice(), 6),
            (b"aba".as_slice(), 7),
        ] {
            ranks.insert(token.to_vec(), rank);
        }
        let alphabet = *b"abc \n";
        for len in 0..=5 {
            let total = alphabet.len().pow(len as u32);
            for mut code in 0..total {
                let mut piece = Vec::with_capacity(len);
                for _ in 0..len {
                    piece.push(alphabet[code % alphabet.len()]);
                    code /= alphabet.len();
                }
                assert_eq!(
                    byte_pair_count(&piece, &ranks),
                    byte_pair_count_naive(&piece, &ranks),
                    "piece {piece:?}"
                );
            }
        }
    }

    #[test]
    fn byte_pair_merge_scales_to_a_long_piece() {
        let mut ranks = HashMap::new();
        for (index, run) in [2usize, 4, 8, 16, 32, 64, 128, 256].into_iter().enumerate() {
            ranks.insert(vec![b' '; run], index as u32);
        }
        let piece = vec![b' '; 20_000];
        let started = std::time::Instant::now();
        let tokens = byte_pair_count(&piece, &ranks);
        let elapsed = started.elapsed();
        // 20000 spaces collapse to 78 runs of 256 plus a 32-space remainder.
        assert_eq!(tokens, 79);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "counting {tokens} tokens took {elapsed:?}"
        );
    }

    #[test]
    fn hf_counts_byte_level_merges_and_ignores_added_tokens() {
        let bpe = Bpe::from_hf_json(&hf_document(&hf_merges(), false)).expect("valid vocabulary");
        assert_eq!(bpe.count("abc world"), 2);
        assert_eq!(bpe.count("abc"), 1);
        assert_eq!(bpe.count(" world"), 1);
        // GPT-2 keeps a whole digit run in one piece; without digit merges it
        // still costs one token per byte.
        assert_eq!(bpe.count("1234"), 4);
        // Non-ASCII is UTF-8 bytes through the byte-to-unicode table.
        assert_eq!(bpe.count("é"), 2);
        // a / " b" / newline / 1234 / ! / é / tab
        assert_eq!(bpe.count("a b\n1234!é\t"), 12);
        assert_eq!(bpe.count(""), 0);

        let with_added =
            Bpe::from_hf_json(&hf_document(&hf_merges(), true)).expect("valid vocabulary");
        // Special tokens are not part of the model: they count as plain text.
        assert_eq!(with_added.count("<|end|>"), 7);
    }

    #[test]
    fn hf_honours_ignore_merges_and_add_prefix_space() {
        let extra = vec!["ab".to_string()];
        let plain = hf_document_with(
            &[],
            false,
            &serde_json::json!({"ignore_merges": false}),
            serde_json::json!({"type": "ByteLevel"}),
            None,
            &extra,
        );
        let plain = Bpe::from_hf_json(&plain).expect("valid vocabulary");
        // No merge for a+b: two byte symbols.
        assert_eq!(plain.count("ab"), 2);

        let ignoring = hf_document_with(
            &[],
            false,
            &serde_json::json!({"ignore_merges": true}),
            serde_json::json!({"type": "ByteLevel"}),
            None,
            &extra,
        );
        let ignoring = Bpe::from_hf_json(&ignoring).expect("valid vocabulary");
        // The whole piece is a vocabulary token, so it is one token.
        assert_eq!(ignoring.count("ab"), 1);

        let prefixed = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({"type": "ByteLevel", "add_prefix_space": true}),
            None,
            &[],
        );
        let prefixed = Bpe::from_hf_json(&prefixed).expect("valid vocabulary");
        // One leading space is prepended once: Ġ a b.
        assert_eq!(prefixed.count("ab"), 3);
        // Text that already starts with a space is not prefixed again.
        assert_eq!(prefixed.count(" ab"), 3);
        assert_eq!(prefixed.count(""), 0);

        let plain = Bpe::from_hf_json(&hf_document(&[], false)).expect("valid vocabulary");
        assert_eq!(plain.count("ab"), 2);
    }

    #[test]
    fn hf_split_patterns_and_configuration_errors() {
        let split = |pattern: &str, behavior: &str| {
            serde_json::json!({
                "type": "Sequence",
                "pretokenizers": [
                    {
                        "type": "Split",
                        "pattern": {"Regex": pattern},
                        "behavior": behavior,
                        "invert": false
                    },
                    {
                        "type": "ByteLevel",
                        "add_prefix_space": false,
                        "trim_offsets": true,
                        "use_regex": false
                    }
                ]
            })
        };
        // Qwen2: digits are separate pieces, so the BPE merges cannot join
        // them; cl100k keeps "123" together and merges it.
        let digits = vec![
            ("1".to_string(), "2".to_string()),
            ("12".to_string(), "3".to_string()),
        ];
        let qwen = hf_document_with(
            &digits,
            false,
            &serde_json::json!({}),
            split(QWEN2_PATTERN, "Isolated"),
            None,
            &[],
        );
        let qwen = Bpe::from_hf_json(&qwen).expect("Qwen2 pattern is supported");
        assert!(qwen.method.contains("Qwen2"));
        assert_eq!(qwen.count("1234"), 4);

        // The same Qwen2 pattern written out literally, so the fixture does
        // not depend on the constant.
        let literal_qwen2 = r"(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";
        let literal = hf_document_with(
            &digits,
            false,
            &serde_json::json!({}),
            split(literal_qwen2, "Isolated"),
            None,
            &[],
        );
        let literal = Bpe::from_hf_json(&literal).expect("literal Qwen2 pattern is supported");
        assert!(literal.method.contains("Qwen2"));
        assert_eq!(literal.count("1234"), 4);

        let cl = hf_document_with(
            &digits,
            false,
            &serde_json::json!({}),
            split(CL100K_PATTERN, "Isolated"),
            None,
            &[],
        );
        let cl = Bpe::from_hf_json(&cl).expect("cl100k pattern is supported");
        assert_eq!(cl.count("1234"), 2);

        let unknown = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            split(r"[\r\n]", "Isolated"),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&unknown).err().expect("unknown pattern");
        assert!(
            error.starts_with("unsupported pre-tokenizer pattern: "),
            "{error}"
        );
        assert!(error.contains(r"[\r\n]"), "{error}");

        let not_isolated = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            split(CL100K_PATTERN, "Removed"),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&not_isolated).err().expect("behaviour");
        assert!(error.contains("unsupported Split behavior"), "{error}");
    }

    #[test]
    fn hf_rejects_chained_splits_sequence_prefix_space_and_bare_patterns() {
        let byte_level = serde_json::json!({
            "type": "ByteLevel",
            "add_prefix_space": false,
            "trim_offsets": true,
            "use_regex": false
        });
        let split = |pattern: Value| {
            serde_json::json!({
                "type": "Split",
                "pattern": pattern,
                "behavior": "Isolated",
                "invert": false
            })
        };

        // DeepSeek chains several Split pre-tokenizers around a ByteLevel.
        let chained = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({
                "type": "Sequence",
                "pretokenizers": [
                    split(serde_json::json!({"Regex": CL100K_PATTERN})),
                    byte_level.clone(),
                    split(serde_json::json!({"Regex": QWEN2_PATTERN}))
                ]
            }),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&chained)
            .err()
            .expect("chained Split pre-tokenizers");
        assert!(
            error.contains("chained Split pre-tokenizers (e.g. DeepSeek) are not supported"),
            "{error}"
        );

        let prefixed_sequence = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({
                "type": "Sequence",
                "pretokenizers": [
                    split(serde_json::json!({"Regex": CL100K_PATTERN})),
                    {
                        "type": "ByteLevel",
                        "add_prefix_space": true,
                        "trim_offsets": true,
                        "use_regex": false
                    }
                ]
            }),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&prefixed_sequence)
            .err()
            .expect("add_prefix_space inside a Sequence");
        assert!(error.contains("add_prefix_space"), "{error}");

        let bare_pattern = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({
                "type": "Sequence",
                "pretokenizers": [split(Value::String(QWEN2_PATTERN.to_string())), byte_level]
            }),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&bare_pattern)
            .err()
            .expect("bare-string Split pattern");
        assert!(error.contains("Split pattern is not a Regex"), "{error}");
    }

    #[test]
    fn hf_normalizer_and_model_options_are_checked() {
        let nfc = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({"type": "ByteLevel"}),
            Some(serde_json::json!({"type": "NFC"})),
            &[],
        );
        assert!(Bpe::from_hf_json(&nfc).is_ok());

        let other = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({"type": "ByteLevel"}),
            Some(serde_json::json!({"type": "NFKC"})),
            &[],
        );
        let error = Bpe::from_hf_json(&other)
            .err()
            .expect("unsupported normalizer");
        assert!(error.contains("unsupported normalizer"), "{error}");

        let byte_fallback = hf_document_with(
            &[],
            false,
            &serde_json::json!({"byte_fallback": true}),
            serde_json::json!({"type": "ByteLevel"}),
            None,
            &[],
        );
        let error = Bpe::from_hf_json(&byte_fallback)
            .err()
            .expect("byte fallback");
        assert!(error.contains("byte_fallback"), "{error}");

        let no_regex = hf_document_with(
            &[],
            false,
            &serde_json::json!({}),
            serde_json::json!({"type": "ByteLevel", "use_regex": false}),
            None,
            &[],
        );
        assert!(Bpe::from_hf_json(&no_regex).is_err());
    }

    #[test]
    fn hf_counts_a_large_python_like_file_quickly() {
        let mut text = String::new();
        for i in 0..3000 {
            text.push_str(&format!("x{i} = compute(value_{i})\n"));
        }
        assert!(text.len() > 60_000, "{} bytes", text.len());
        let bpe = Bpe::from_hf_json(&hf_document(&hf_merges(), false)).expect("valid vocabulary");
        let started = std::time::Instant::now();
        let tokens = bpe.count(&text);
        let elapsed = started.elapsed();
        assert!(tokens > 0);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "counting {tokens} tokens took {elapsed:?}"
        );
    }

    #[test]
    fn byte_to_unicode_matches_gpt2() {
        let alphabet = byte_to_unicode();
        assert_eq!(alphabet[b'A' as usize], 'A');
        assert_eq!(alphabet[b' ' as usize], '\u{0120}');
        assert_eq!(alphabet[b'\n' as usize], '\u{010a}');
        assert_eq!(alphabet[0], '\u{0100}');
        assert_eq!(alphabet[0xff], '\u{00ff}');
    }

    #[test]
    fn malformed_vocabularies_are_errors() {
        assert!(Bpe::from_tiktoken("nonsense", "tiny").is_err());
        assert!(Bpe::from_tiktoken("AAAA", "tiny").is_err());
        assert!(Bpe::from_tiktoken("!!! 1", "tiny").is_err());
        assert!(Bpe::from_tiktoken("", "tiny").is_err());
        assert!(base64_decode("A").is_none());
        assert!(base64_decode("YQ=A").is_none());
        assert!(Bpe::from_hf_json("{}").is_err());
        assert!(
            Bpe::from_hf_json(r#"{"model":{"type":"WordPiece","vocab":{},"merges":[]}}"#).is_err()
        );
        let no_byte_level =
            serde_json::json!({"model": {"type": "BPE", "vocab": {"a": 0}, "merges": []}})
                .to_string();
        assert!(Bpe::from_hf_json(&no_byte_level).is_err());
    }

    #[test]
    fn load_names_counters_by_file_stem_and_reports_bad_files() {
        let dir = std::env::temp_dir().join(format!("aleph-tokens-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let path = dir.join("r50k_base.tiktoken");
        fs::write(&path, tiktoken_vocab(&[])).unwrap();
        let loaded = load(&[path.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(loaded[0].name, "bpe:r50k_base");

        let path = dir.join("bad.tiktoken");
        fs::write(&path, "not a vocabulary\n").unwrap();
        let error = load(&[path.to_string_lossy().into_owned()])
            .err()
            .expect("a malformed vocabulary is an error");
        assert!(error.contains("bad.tiktoken"), "{error}");

        fs::remove_dir_all(&dir).unwrap();
    }
}
