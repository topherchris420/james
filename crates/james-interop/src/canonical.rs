//! `agent-interop/v1` canonical JSON and SHA-256.
//!
//! The text is exactly what the R.A.I.N. Lab's `canonicalJson` (JavaScript
//! `JSON.stringify` with sorted keys) produces, so a digest computed in one
//! system can be recomputed in the other: keys in UTF-16 code-unit order,
//! no whitespace, `JSON.stringify` string escapes, and ECMAScript
//! `Number::toString` number formatting. Checked against the golden fixtures
//! in `docs/interop/agent-interop-v1/fixtures/canonical.json`, which the Lab's
//! own code wrote.
//!
//! A digest detects change. It is not a signature.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

/// The largest integer JavaScript represents exactly (2^53 - 1).
const MAX_SAFE: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalError(pub &'static str);

impl std::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for CanonicalError {}

/// The canonical text of a JSON value.
pub fn canonical(value: &Value) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_value(&mut out, value)?;
    Ok(out)
}

fn write_value(out: &mut String, value: &Value) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(u) = n.as_u64().filter(|u| *u <= MAX_SAFE) {
                let _ = write!(out, "{u}");
            } else if let Some(i) = n.as_i64().filter(|i| i.unsigned_abs() <= MAX_SAFE) {
                let _ = write!(out, "{i}");
            } else {
                let f = n
                    .as_f64()
                    .ok_or(CanonicalError("number is not representable"))?;
                out.push_str(&js_number(f)?);
            }
        }
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_value(out, &map[*key])?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// `JSON.stringify`'s string quoting (ECMA-262 QuoteJSONString).
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// ECMAScript `Number::toString(10)` for a finite double.
pub fn js_number(f: f64) -> Result<String, CanonicalError> {
    if !f.is_finite() {
        return Err(CanonicalError(
            "NaN and infinities are refused in hashed JSON",
        ));
    }
    if f == 0.0 {
        return Ok("0".into()); // also -0
    }
    // Rust's LowerExp prints the shortest digits that round-trip.
    let exp_form = format!("{:e}", f.abs());
    let (mantissa, exponent) = exp_form
        .split_once('e')
        .ok_or(CanonicalError("unexpected float format"))?;
    let exponent: i32 = exponent
        .parse()
        .map_err(|_| CanonicalError("unexpected float exponent"))?;
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent + 1; // value = 0.d1d2…dk × 10^n
    let mut out = String::new();
    if f < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let _ = write!(out, "e{}{}", if e >= 0 { '+' } else { '-' }, e.abs());
    }
    Ok(out)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// SHA-256 of a value's canonical text.
pub fn digest(value: &Value) -> Result<String, CanonicalError> {
    canonical(value).map(|text| sha256_hex(text.as_bytes()))
}

/// The digest a record carries: canonical JSON of the record without `sha256`.
pub fn record_digest(record: &Value) -> Result<String, CanonicalError> {
    let Value::Object(map) = record else {
        return Err(CanonicalError("a record is a JSON object"));
    };
    let mut body = map.clone();
    body.remove("sha256");
    digest(&Value::Object(body))
}

/// The record with its `sha256` field set from its canonical body.
pub fn seal(mut record: Value) -> Result<Value, CanonicalError> {
    let sha = record_digest(&record)?;
    match &mut record {
        Value::Object(map) => {
            map.insert("sha256".into(), Value::String(sha));
            Ok(record)
        }
        _ => Err(CanonicalError("a record is a JSON object")),
    }
}

/// Whether a record's `sha256` matches its canonical body.
pub fn verify_sealed(record: &Value) -> bool {
    match (
        record.get("sha256").and_then(Value::as_str),
        record_digest(record),
    ) {
        (Some(claimed), Ok(actual)) => claimed == actual,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_follow_ecmascript() {
        let cases: [(f64, &str); 16] = [
            (1.0, "1"),
            (-0.0, "0"),
            (0.1, "0.1"),
            (100.0, "100"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (1e-7, "1e-7"),
            (1e-6, "0.000001"),
            (1.5e-7, "1.5e-7"),
            (123e-20, "1.23e-18"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (-2.25, "-2.25"),
            (2.5e-5, "0.000025"),
            (1.2345678901234568e20, "123456789012345680000"),
            (1e16, "10000000000000000"),
        ];
        for (f, expected) in cases {
            assert_eq!(js_number(f).unwrap(), expected, "{f:e}");
        }
        assert!(js_number(f64::NAN).is_err());
        assert!(js_number(f64::INFINITY).is_err());
    }

    #[test]
    fn keys_sort_by_utf16_not_bytes() {
        // U+E000 is EE 80 80 in UTF-8 but 0xE000 in UTF-16; the emoji is
        // F0 … in UTF-8 but 0xD83D … in UTF-16, so the two orders disagree.
        let text = canonical(&json!({"\u{e000}": 1, "😀": 2, "a": 3})).unwrap();
        assert_eq!(text, "{\"a\":3,\"😀\":2,\"\u{e000}\":1}");
    }

    #[test]
    fn strings_escape_like_json_stringify() {
        let text = canonical(&json!("a\"\\\n\u{1f}\u{7f}\u{2028}é")).unwrap();
        assert_eq!(text, "\"a\\\"\\\\\\n\\u001f\u{7f}\u{2028}é\"");
    }

    #[test]
    fn seal_and_verify() {
        let sealed = seal(json!({"schema": "x", "n": 1.5})).unwrap();
        assert!(verify_sealed(&sealed));
        let mut tampered = sealed.clone();
        tampered["n"] = json!(2.5);
        assert!(!verify_sealed(&tampered));
    }
}
