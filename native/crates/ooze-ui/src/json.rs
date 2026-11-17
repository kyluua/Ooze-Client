//! A small, bounded JSON reader and writer for Ooze's own data documents (profiles, presets).
//!
//! Standard library only, so the crate adds no third-party packages to the native workspace.
//! Imported documents are untrusted: input size and nesting depth are bounded, and every
//! error is a value, never a panic.

use std::fmt::Write as _;

/// Largest accepted document (bytes). Profiles are a few kilobytes.
pub const MAX_BYTES: usize = 256 * 1024;
/// Deepest accepted nesting.
pub const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    /// Insertion order is kept; duplicate keys are rejected while parsing.
    Object(Vec<(String, Json)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub reason: &'static str,
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Self::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// Serialise with two-space indentation and a trailing newline.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(n) => {
                if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
                    let _ = write!(out, "{}", *n as i64);
                } else if n.is_finite() {
                    let _ = write!(out, "{n}");
                } else {
                    out.push_str("null");
                }
            }
            Self::String(s) => write_string(out, s),
            Self::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    pad(out, indent + 1);
                    item.write(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push(']');
            }
            Self::Object(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    out.push_str(if i == 0 { "\n" } else { ",\n" });
                    pad(out, indent + 1);
                    write_string(out, k);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push('}');
            }
        }
    }
}

fn pad(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse a complete JSON document.
pub fn parse(text: &str) -> Result<Json, JsonError> {
    if text.len() > MAX_BYTES {
        return Err(JsonError {
            offset: MAX_BYTES,
            reason: "document too large",
        });
    }
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(p.err("trailing characters"));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, reason: &'static str) -> JsonError {
        JsonError {
            offset: self.i,
            reason,
        }
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\n' | b'\r' | b'\t') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &[u8]) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.err("nesting too deep"));
        }
        match self.s.get(self.i) {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') if self.eat(b"true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat(b"false") => Ok(Json::Bool(false)),
            Some(b'n') if self.eat(b"null") => Ok(Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.err("expected a value")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.i += 1;
        let mut fields: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.eat(b"}") {
            return Ok(Json::Object(fields));
        }
        loop {
            self.ws();
            if self.s.get(self.i) != Some(&b'"') {
                return Err(self.err("expected a key"));
            }
            let key = self.string()?;
            if fields.iter().any(|(k, _)| *k == key) {
                return Err(self.err("duplicate key"));
            }
            self.ws();
            if !self.eat(b":") {
                return Err(self.err("expected ':'"));
            }
            self.ws();
            let v = self.value(depth + 1)?;
            fields.push((key, v));
            self.ws();
            if self.eat(b",") {
                continue;
            }
            if self.eat(b"}") {
                return Ok(Json::Object(fields));
            }
            return Err(self.err("expected ',' or '}'"));
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.eat(b"]") {
            return Ok(Json::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value(depth + 1)?);
            self.ws();
            if self.eat(b",") {
                continue;
            }
            if self.eat(b"]") {
                return Ok(Json::Array(items));
            }
            return Err(self.err("expected ',' or ']'"));
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&b) = self.s.get(self.i) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(
                std::str::from_utf8(&self.s[start..self.i])
                    .map_err(|_| self.err("invalid UTF-8"))?,
            );
            match self.s.get(self.i) {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = match self.s.get(self.i) {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            self.i += 1;
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                if !self.eat(b"\\u") {
                                    return Err(self.err("unpaired surrogate"));
                                }
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(self.err("unpaired surrogate"));
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(
                                char::from_u32(code).ok_or_else(|| self.err("invalid escape"))?,
                            );
                            continue;
                        }
                        _ => return Err(self.err("invalid escape")),
                    };
                    out.push(c);
                    self.i += 1;
                }
                _ => return Err(self.err("unterminated string")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.err("short escape"))?;
        let text = std::str::from_utf8(digits).map_err(|_| self.err("invalid escape"))?;
        let v = u32::from_str_radix(text, 16).map_err(|_| self.err("invalid escape"))?;
        self.i += 4;
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.i;
        self.eat(b"-");
        let digits = |p: &mut Self| {
            let s = p.i;
            while let Some(b'0'..=b'9') = p.s.get(p.i) {
                p.i += 1;
            }
            p.i > s
        };
        if !digits(self) {
            return Err(self.err("expected digits"));
        }
        if self.eat(b".") && !digits(self) {
            return Err(self.err("expected digits"));
        }
        if let Some(b'e' | b'E') = self.s.get(self.i) {
            self.i += 1;
            if let Some(b'+' | b'-') = self.s.get(self.i) {
                self.i += 1;
            }
            if !digits(self) {
                return Err(self.err("expected digits"));
            }
        }
        let text =
            std::str::from_utf8(&self.s[start..self.i]).map_err(|_| self.err("invalid number"))?;
        let n: f64 = text.parse().map_err(|_| self.err("invalid number"))?;
        if !n.is_finite() {
            return Err(self.err("number out of range"));
        }
        Ok(Json::Number(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_documents() {
        let text = r#"{"a": [1, 2.5, -3e2, true, false, null], "b": {"c": "x\"y\\z\u00e9\ud83d\ude00"}, "e": {}}"#;
        let v = parse(text).unwrap();
        assert_eq!(
            v.get("b").and_then(|b| b.get("c")).and_then(Json::as_str),
            Some("x\"y\\zé😀")
        );
        let again = parse(&v.to_pretty()).unwrap();
        assert_eq!(v, again);
    }

    #[test]
    fn rejects_hostile_or_malformed_input() {
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "{\"a\":1,\"a\":2}",
            "01x",
            "\"\\ud800\"",
            "nul",
            "1 2",
            "\"\u{1}\"",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        let deep = "[".repeat(MAX_DEPTH + 2) + &"]".repeat(MAX_DEPTH + 2);
        assert_eq!(parse(&deep).unwrap_err().reason, "nesting too deep");
        let big = format!("\"{}\"", "a".repeat(MAX_BYTES));
        assert_eq!(parse(&big).unwrap_err().reason, "document too large");
    }
}
