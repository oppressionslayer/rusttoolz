//! **One certificate, three languages.** A climb is a list of rungs. The Python package writes
//! them as `[[seed, null, null], [N, q, a], ...]` with `q` the previous proven rung (the
//! `numtools` convention), the arcade's JavaScript verifier also reads the *slip* convention
//! `[N, k, a]` with `N = 2·k·q + 1` off the previous rung, and this module reads and writes both,
//! big integers as decimal strings, so a chain rolled in Rust verifies in Python and in the
//! browser, and theirs verify here.

use crate::prove::Rung;
use num_bigint::BigUint;
use num_traits::{One, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Convention {
    /// `[N, q, a]`: the middle entry is the previous proven prime.
    Numtools,
    /// `[N, k, a]`: the middle entry is the multiplier, `N = 2·k·q + 1`.
    Slip,
}

fn quote(n: &BigUint) -> String {
    format!("\"{n}\"")
}

/// The chain as JSON text in the given convention.
pub fn to_json(chain: &[Rung], conv: Convention) -> String {
    let mut parts = Vec::with_capacity(chain.len());
    for (i, r) in chain.iter().enumerate() {
        if i == 0 || r.is_seed() {
            parts.push(format!("[{}, null, null]", quote(&r.n)));
        } else {
            let middle = match conv {
                Convention::Numtools => r.q.clone(),
                Convention::Slip => (&r.n - BigUint::one()) / (BigUint::from(2u32) * &r.q),
            };
            parts.push(format!("[{}, {}, {}]", quote(&r.n), quote(&middle), r.a));
        }
    }
    format!("[{}]", parts.join(", "))
}

/// A chain back from JSON text, in either convention, with integers as strings or bare numbers.
/// Python's `repr` of such a list (`'...'`, `None`) is accepted too.
pub fn from_json(text: &str, conv: Convention) -> Result<Vec<Rung>, String> {
    let text = text.replace('\'', "\"").replace("None", "null");
    let v: serde_json_lite::Value = serde_json_lite::parse(&text)?;
    let rows = v.as_array().ok_or("a certificate is a list of rungs")?;
    let mut chain: Vec<Rung> = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let cells = row.as_array().ok_or(format!("rung {i} is not a list"))?;
        if cells.len() != 3 {
            return Err(format!("rung {i} is not a triple"));
        }
        let n = cells[0].as_big().ok_or(format!("rung {i}: N"))?;
        if cells[1].is_null() {
            chain.push(Rung { n, q: BigUint::zero(), a: 0 });
            continue;
        }
        let middle = cells[1].as_big().ok_or(format!("rung {i}: middle"))?;
        let a = cells[2].as_big().and_then(|b| u32::try_from(b).ok()).ok_or(format!("rung {i}: witness"))?;
        let q = match conv {
            Convention::Numtools => middle,
            Convention::Slip => {
                let prev = chain.last().ok_or("a slip rung before the seed")?;
                prev.n.clone()
            }
        };
        if conv == Convention::Slip {
            let k = cells[1].as_big().unwrap();
            if BigUint::from(2u32) * &k * &q + BigUint::one() != n {
                return Err(format!("rung {i}: N is not 2kq + 1 off the previous rung"));
            }
        }
        chain.push(Rung { n, q, a });
    }
    Ok(chain)
}

/// The convention a text is in: slip when the middle of rung 1 times `2·seed` plus one is `N`.
pub fn sniff(text: &str) -> Option<Convention> {
    let chain = from_json(text, Convention::Numtools).ok()?;
    if chain.len() < 2 {
        return Some(Convention::Numtools);
    }
    let (seed, r1) = (&chain[0].n, &chain[1]);
    if BigUint::from(2u32) * &r1.q * seed + BigUint::one() == r1.n {
        Some(Convention::Slip)
    } else {
        Some(Convention::Numtools)
    }
}

/// A tiny JSON reader for certificate lists: arrays, strings, integers and null are all a
/// certificate ever holds, so the crate carries no JSON dependency outside its tests.
mod serde_json_lite {
    use num_bigint::BigUint;

    #[derive(Debug)]
    pub enum Value {
        Null,
        Str(String),
        Num(String),
        Arr(Vec<Value>),
    }

    impl Value {
        pub fn as_array(&self) -> Option<&Vec<Value>> {
            match self {
                Value::Arr(v) => Some(v),
                _ => None,
            }
        }
        pub fn is_null(&self) -> bool {
            matches!(self, Value::Null)
        }
        pub fn as_big(&self) -> Option<BigUint> {
            match self {
                Value::Str(s) | Value::Num(s) => s.trim().parse().ok(),
                _ => None,
            }
        }
    }

    pub fn parse(text: &str) -> Result<Value, String> {
        let b = text.as_bytes();
        let mut i = 0;
        let v = value(b, &mut i)?;
        skip(b, &mut i);
        if i != b.len() {
            return Err("trailing text after the certificate".into());
        }
        Ok(v)
    }

    fn skip(b: &[u8], i: &mut usize) {
        while *i < b.len() && b[*i].is_ascii_whitespace() {
            *i += 1;
        }
    }

    fn value(b: &[u8], i: &mut usize) -> Result<Value, String> {
        skip(b, i);
        match b.get(*i) {
            Some(b'[') => {
                *i += 1;
                let mut out = Vec::new();
                loop {
                    skip(b, i);
                    if b.get(*i) == Some(&b']') {
                        *i += 1;
                        return Ok(Value::Arr(out));
                    }
                    out.push(value(b, i)?);
                    skip(b, i);
                    match b.get(*i) {
                        Some(b',') => *i += 1,
                        Some(b']') => {}
                        _ => return Err("expected , or ]".into()),
                    }
                }
            }
            Some(b'"') => {
                *i += 1;
                let start = *i;
                while *i < b.len() && b[*i] != b'"' {
                    *i += 1;
                }
                let s = std::str::from_utf8(&b[start..*i]).map_err(|_| "bad string")?.to_string();
                *i += 1;
                Ok(Value::Str(s))
            }
            Some(b'n') if b[*i..].starts_with(b"null") => {
                *i += 4;
                Ok(Value::Null)
            }
            Some(c) if c.is_ascii_digit() => {
                let start = *i;
                while *i < b.len() && b[*i].is_ascii_digit() {
                    *i += 1;
                }
                Ok(Value::Num(std::str::from_utf8(&b[start..*i]).unwrap().to_string()))
            }
            _ => Err(format!("unexpected character at {i}")),
        }
    }
}
