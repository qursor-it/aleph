//! JSON values for the interpreter (ADR-0033).
//!
//! A JSON value is `Value::Json(payload)` where the payload is `Unit` (null),
//! `Bool`, `I64`, `F64`, `Str`, a `List` of JSON values or a `Dict` from
//! `Str` to JSON values. Tlön mirrors this module byte for byte.

use std::rc::Rc;

use crate::interp::{compare_values, Value};

/// Maximum nesting of arrays and objects accepted by [`parse`].
pub const MAX_DEPTH: usize = 256;

/// `json x`: wrap a jsonable value; a NaN or infinite `f64` becomes null.
pub fn wrap(value: Value) -> Value {
    let payload = match value {
        Value::F64(number) if !number.is_finite() => Value::Unit,
        other => other,
    };
    Value::Json(Rc::new(payload))
}

/// Compact JSON text (`show` on a `json` value).
pub fn print(payload: &Value) -> String {
    let mut out = String::new();
    print_into(&mut out, payload);
    out
}

fn print_into(out: &mut String, payload: &Value) {
    match payload {
        Value::Unit => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::I64(number) => out.push_str(&number.to_string()),
        Value::F64(number) => out.push_str(&format!("{number:?}")),
        Value::Str(text) => print_str(out, text),
        Value::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                print_into(out, inner(item));
            }
            out.push(']');
        }
        Value::Dict(pairs) => {
            out.push('{');
            for (index, (key, val)) in pairs.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                print_into(out, key);
                out.push(':');
                print_into(out, inner(val));
            }
            out.push('}');
        }
        _ => panic!("type checker guarantees a jsonable payload"),
    }
}

fn print_str(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

/// The payload of a `Value::Json`.
pub fn inner(value: &Value) -> &Value {
    match value {
        Value::Json(payload) => payload,
        _ => panic!("type checker guarantees a json value"),
    }
}

/// `jparse`: the value, or `json: <reason> at byte <n>`.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let value = parser.value(0)?;
    parser.skip_ws();
    if parser.pos < parser.bytes.len() {
        return Err(parser.fail("unexpected byte", parser.pos));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn fail(&self, reason: &str, at: usize) -> String {
        format!("json: {reason} at byte {at}")
    }

    /// `unexpected end` at the end, else `unexpected byte` here.
    fn unexpected(&self) -> String {
        if self.pos >= self.bytes.len() {
            self.fail("unexpected end", self.bytes.len())
        } else {
            self.fail("unexpected byte", self.pos)
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    /// One value after optional whitespace; `depth` counts open brackets.
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        self.skip_ws();
        let payload = match self.peek() {
            None => return Err(self.unexpected()),
            Some(b'[') => self.list(depth + 1)?,
            Some(b'{') => self.object(depth + 1)?,
            Some(b'"') => Value::Str(Rc::from(self.string()?.as_str())),
            Some(b't') => self.literal("true", Value::Bool(true))?,
            Some(b'f') => self.literal("false", Value::Bool(false))?,
            Some(b'n') => self.literal("null", Value::Unit)?,
            Some(b'-' | b'0'..=b'9') => self.number()?,
            Some(_) => return Err(self.unexpected()),
        };
        Ok(Value::Json(Rc::new(payload)))
    }

    fn literal(&mut self, word: &str, payload: Value) -> Result<Value, String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(payload)
        } else {
            Err(self.fail("unexpected byte", self.pos))
        }
    }

    fn open(&mut self, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err(self.fail("too deep", self.pos));
        }
        self.pos += 1;
        self.skip_ws();
        Ok(())
    }

    /// After an element: `true` on `,`, `false` on `close`.
    fn separator(&mut self, close: u8) -> Result<bool, String> {
        self.skip_ws();
        match self.peek() {
            Some(b',') => {
                self.pos += 1;
                Ok(true)
            }
            Some(byte) if byte == close => {
                self.pos += 1;
                Ok(false)
            }
            _ => Err(self.unexpected()),
        }
    }

    fn list(&mut self, depth: usize) -> Result<Value, String> {
        self.open(depth)?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
        } else {
            loop {
                items.push(self.value(depth)?);
                if !self.separator(b']')? {
                    break;
                }
            }
        }
        Ok(Value::List(Rc::new(items)))
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.open(depth)?;
        let mut pairs: Vec<(Value, Value)> = Vec::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Dict(Rc::new(pairs)));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.unexpected());
            }
            let key_at = self.pos;
            let key = Value::Str(Rc::from(self.string()?.as_str()));
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.unexpected());
            }
            self.pos += 1;
            let val = self.value(depth)?;
            match pairs.binary_search_by(|(probe, _)| compare_values(probe, &key)) {
                Ok(_) => return Err(self.fail("duplicate key", key_at)),
                Err(index) => pairs.insert(index, (key, val)),
            }
            if !self.separator(b'}')? {
                break;
            }
        }
        Ok(Value::Dict(Rc::new(pairs)))
    }

    /// A string starting at the opening quote.
    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.unexpected());
            };
            match byte {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    let ch = self.escape()?;
                    let mut buffer = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
                }
                0..=0x1f => return Err(self.unexpected()),
                _ => {
                    out.push(byte);
                    self.pos += 1;
                }
            }
        }
        // Raw bytes come whole from a valid `str`; escapes add valid chars.
        Ok(String::from_utf8(out).expect("json string is utf-8"))
    }

    /// One escape starting at its backslash.
    fn escape(&mut self) -> Result<char, String> {
        let at = self.pos;
        self.pos += 1;
        let Some(byte) = self.peek() else {
            return Err(self.unexpected());
        };
        self.pos += 1;
        let ch = match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode(at),
            _ => return Err(self.fail("bad escape", at)),
        };
        Ok(ch)
    }

    /// The rest of `\uXXXX` (and a following low surrogate escape).
    fn unicode(&mut self, at: usize) -> Result<char, String> {
        let high = self.hex4().ok_or_else(|| self.fail("bad escape", at))?;
        let code = match high {
            0xd800..=0xdbff => {
                if !self.bytes[self.pos..].starts_with(b"\\u") {
                    return Err(self.fail("bad escape", at));
                }
                self.pos += 2;
                match self.hex4() {
                    Some(low @ 0xdc00..=0xdfff) => {
                        0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                    }
                    _ => return Err(self.fail("bad escape", at)),
                }
            }
            0xdc00..=0xdfff => return Err(self.fail("bad escape", at)),
            code => code,
        };
        Ok(char::from_u32(code).expect("valid scalar value"))
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.pos..self.pos + 4)?;
        let mut code = 0;
        for &digit in digits {
            code = code * 16 + (digit as char).to_digit(16)?;
        }
        self.pos += 4;
        Some(code)
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        let bad = |parser: &Self| parser.fail("bad number", start);
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(bad(self)),
        }
        let mut integral = true;
        if self.peek() == Some(b'.') {
            integral = false;
            self.pos += 1;
            if !self.digits_required() {
                return Err(bad(self));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integral = false;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !self.digits_required() {
                return Err(bad(self));
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).expect("ascii number");
        if integral {
            if let Ok(number) = text.parse::<i64>() {
                return Ok(Value::I64(number));
            }
        }
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(Value::F64(number)),
            _ => Err(bad(self)),
        }
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }

    fn digits_required(&mut self) -> bool {
        let start = self.pos;
        self.digits();
        self.pos > start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(text: &str) -> String {
        match parse(text) {
            Ok(value) => print(inner(&value)),
            Err(message) => message,
        }
    }

    #[test]
    fn prints_canonical_text() {
        assert_eq!(
            round(" {\"b\" : [1, 2.5e0, -0, true, null], \"a\":\"x\\u00e9\\n\"} "),
            "{\"a\":\"xé\\n\",\"b\":[1,2.5,0,true,null]}"
        );
        assert_eq!(round("1e400"), "json: bad number at byte 0");
        assert_eq!(round("12345678901234567890"), "1.2345678901234567e19");
        assert_eq!(round("-9223372036854775808"), "-9223372036854775808");
        assert_eq!(round("\"\\ud83d\\ude00\\u0001\""), "\"😀\\u0001\"");
        assert_eq!(round("[1e2,1E-2,0.5]"), "[100.0,0.01,0.5]");
    }

    #[test]
    fn reports_errors_with_offsets() {
        assert_eq!(round(""), "json: unexpected end at byte 0");
        assert_eq!(round("[1,]"), "json: unexpected byte at byte 3");
        assert_eq!(round("[1 2]"), "json: unexpected byte at byte 3");
        assert_eq!(round("{\"a\":1"), "json: unexpected end at byte 6");
        assert_eq!(round("tru"), "json: unexpected byte at byte 0");
        assert_eq!(round("01"), "json: unexpected byte at byte 1");
        assert_eq!(round("-"), "json: bad number at byte 0");
        assert_eq!(round("1."), "json: bad number at byte 0");
        assert_eq!(round("[\"a\\x\"]"), "json: bad escape at byte 3");
        assert_eq!(round("\"\\ud800\""), "json: bad escape at byte 1");
        assert_eq!(round("\"\\udc00\""), "json: bad escape at byte 1");
        assert_eq!(round("\"\\u12\""), "json: bad escape at byte 1");
        assert_eq!(round("\"a\nb\""), "json: unexpected byte at byte 2");
        assert_eq!(round("{\"a\":1,\"a\":2}"), "json: duplicate key at byte 7");
        assert_eq!(round("{1:2}"), "json: unexpected byte at byte 1");
        assert_eq!(round("1 x"), "json: unexpected byte at byte 2");
        let deep = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert_eq!(round(&deep), deep);
        let deeper = "[".repeat(MAX_DEPTH + 1);
        assert_eq!(round(&deeper), "json: too deep at byte 256");
    }

    #[test]
    fn wrap_maps_non_finite_to_null() {
        assert_eq!(print(inner(&wrap(Value::F64(f64::NAN)))), "null");
        assert_eq!(print(inner(&wrap(Value::F64(1.0)))), "1.0");
    }
}
