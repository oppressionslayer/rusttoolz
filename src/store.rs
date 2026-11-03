//! **The prime store.** Everything the examples prove, written down once so it is never proved
//! twice: a Mersenne exponent with its Lucas–Lehmer verdict, a rung of the ladder with its bits
//! and witness, a Proth prime or twin from the forge. One JSON object per line, appended, so a
//! run that stops leaves every line before it good and the next run picks up at the top.
//!
//! The file is `primes.jsonl` in the working directory, or wherever `RUSTTOOLZ_STORE` points.
//! No dependency: the lines are flat objects of strings, integers and booleans, written and
//! read by the two small functions at the bottom.
//!
//! What the store buys: a Mersenne search skips every exponent it has a verdict for, true or
//! false; `rollball` starts a ladder from the highest rung it already holds, so a 4,423-bit
//! climb that is stored costs a read; the pages and the games can look a proven prime up by its
//! bits instead of proving it in the browser.

use crate::prove::Rung;
use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One line of the store. `kind` is `mersenne`, `rung`, `proth` or `twin`; the rest is what that
/// kind needs. Every record has `bits`; a proven one has `n` (the prime in decimal) unless it is
/// a Mersenne, whose `n` is `2^p − 1` and is not written out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub kind: String,
    pub fields: BTreeMap<String, Field>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    Str(String),
    Int(u64),
    Bool(bool),
}

impl Record {
    pub fn new(kind: &str) -> Record {
        Record { kind: kind.to_string(), fields: BTreeMap::new() }
    }
    pub fn str(mut self, k: &str, v: impl ToString) -> Record {
        self.fields.insert(k.to_string(), Field::Str(v.to_string()));
        self
    }
    pub fn int(mut self, k: &str, v: u64) -> Record {
        self.fields.insert(k.to_string(), Field::Int(v));
        self
    }
    pub fn flag(mut self, k: &str, v: bool) -> Record {
        self.fields.insert(k.to_string(), Field::Bool(v));
        self
    }
    pub fn get_str(&self, k: &str) -> Option<&str> {
        match self.fields.get(k) {
            Some(Field::Str(s)) => Some(s),
            _ => None,
        }
    }
    pub fn get_int(&self, k: &str) -> Option<u64> {
        match self.fields.get(k) {
            Some(Field::Int(v)) => Some(*v),
            Some(Field::Str(s)) => s.parse().ok(),
            _ => None,
        }
    }
    pub fn get_bool(&self, k: &str) -> Option<bool> {
        match self.fields.get(k) {
            Some(Field::Bool(b)) => Some(*b),
            _ => None,
        }
    }
    pub fn big(&self, k: &str) -> Option<BigUint> {
        self.get_str(k).and_then(|s| s.parse().ok())
    }

    /// **What makes two records the same fact**, so a rerun appends nothing: the kind and its
    /// identifying fields.
    pub fn key(&self) -> String {
        let f = |k: &str| self.fields.get(k).map(|v| v.to_json()).unwrap_or_default();
        match self.kind.as_str() {
            "mersenne" => format!("mersenne:{}", f("p")),
            // a ladder has one rung per bit length, whether the line spells the number out
            // or names a known Mersenne by its exponent; keying on the number kept both
            "rung" => format!("rung:{}:{}", f("base"), f("bits")),
            "recipe" => format!("recipe:{}:{}", f("base"), f("bits")),
            "proth" | "twin" => format!("{}:{}:{}", self.kind, f("k"), f("n")),
            _ => format!("{}:{}", self.kind, self.fields.values().map(|v| v.to_json()).collect::<Vec<_>>().join(":")),
        }
    }

    /// A Mersenne verdict: `2^p − 1` prime or not by Lucas–Lehmer, and how long it took.
    pub fn mersenne(p: u32, prime: bool, secs: f64) -> Record {
        Record::new("mersenne").int("p", p as u64).int("bits", p as u64).flag("prime", prime).str("proof", "lucas-lehmer").str("secs", format!("{secs:.3}"))
    }

    /// A rung of a ladder under `base` (`M127`, `net-1000003`, …): its bits, the witness, and the
    /// prime and its `q` in decimal.
    pub fn rung(base: &str, r: &Rung) -> Record {
        let rec = Record::new("rung").str("base", base).int("bits", r.n.bits()).int("witness", r.a as u64);
        if r.is_seed() {
            // **a known Mersenne seed is its exponent.** `2^p − 1` at p = 136,279,841 is forty-one
            // million digits, and a line that long says nothing `p` does not
            let p = r.n.bits();
            let mersenne = r.n == (BigUint::from(1u32) << p) - BigUint::from(1u32) && crate::prove::known_mersenne(p as u32);
            if mersenne {
                rec.str("proof", "seed").int("p", p)
            } else {
                rec.str("proof", "seed").str("n", &r.n)
            }
        } else {
            rec.str("proof", "pocklington").str("n", &r.n).str("q", &r.q).int("q_bits", r.q.bits())
        }
    }

    /// **A recipe**: a climb under `base` to `bits` as its offsets (see [`crate::prove::recipe`])
    /// and witnesses, a few bytes that rebuild every rung exactly.
    pub fn recipe(base: &str, bits: u64, offsets: &[i64], witnesses: &[u32]) -> Record {
        let join = |v: Vec<String>| v.join(",");
        Record::new("recipe")
            .str("base", base)
            .int("bits", bits)
            .int("rungs", offsets.len() as u64)
            .str("offsets", join(offsets.iter().map(|o| o.to_string()).collect()))
            .str("witnesses", join(witnesses.iter().map(|w| w.to_string()).collect()))
    }

    /// What the forge proved about `k·2^n ± 1`: a `proth` when only the `+1` side is proven, a
    /// `twin` when both are.
    pub fn forge(t: &crate::twin::Twin) -> Option<Record> {
        if !t.upper_proven {
            return None;
        }
        let kind = if t.lower_proven { "twin" } else { "proth" };
        let mut rec = Record::new(kind).int("k", t.k).int("n", t.n as u64).int("bits", t.upper.bits()).int("digits", t.digits as u64).str("upper", &t.upper).str("proof", "pocklington");
        if t.lower_proven {
            rec = rec.str("lower", &t.lower).str("lower_proof", "lucas");
        }
        Some(rec)
    }

    pub fn to_json(&self) -> String {
        let mut s = format!("{{\"kind\": \"{}\"", self.kind);
        for (k, v) in &self.fields {
            s.push_str(&format!(", \"{k}\": {}", v.to_json()));
        }
        s.push('}');
        s
    }

    pub fn from_json(line: &str) -> Result<Record, String> {
        let fields = parse_flat(line)?;
        let mut rec = Record::new("");
        for (k, v) in fields {
            if k == "kind" {
                rec.kind = match v {
                    Field::Str(s) => s,
                    _ => return Err("kind must be a string".into()),
                };
            } else {
                rec.fields.insert(k, v);
            }
        }
        if rec.kind.is_empty() {
            return Err("a record needs a kind".into());
        }
        Ok(rec)
    }
}

impl Field {
    fn to_json(&self) -> String {
        match self {
            Field::Str(s) => {
                let mut out = String::from("\"");
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        c => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            Field::Int(v) => v.to_string(),
            Field::Bool(b) => b.to_string(),
        }
    }
}

/// The store: its file, and every record in it keyed by [`Record::key`].
pub struct Store {
    path: PathBuf,
    records: Vec<Record>,
    keys: std::collections::HashSet<String>,
}

impl Store {
    /// `RUSTTOOLZ_STORE`, or `primes.jsonl` in the working directory.
    pub fn default_path() -> PathBuf {
        std::env::var_os("RUSTTOOLZ_STORE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("primes.jsonl"))
    }

    /// Open (or create) the store at `path` and read every line. A line that does not parse
    /// is an error with its number, not a silent skip: a store is only worth what it holds.
    pub fn open(path: impl AsRef<Path>) -> Result<Store, String> {
        let path = path.as_ref().to_path_buf();
        let mut st = Store { path: path.clone(), records: Vec::new(), keys: Default::default() };
        if path.exists() {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            for (i, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let rec = Record::from_json(line).map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?;
                st.keys.insert(rec.key());
                st.records.push(rec);
            }
        }
        Ok(st)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Record> + 'a {
        self.records.iter().filter(move |r| r.kind == kind)
    }

    /// **Append a record unless the store already holds that fact.** Returns whether it was new.
    pub fn record(&mut self, rec: Record) -> Result<bool, String> {
        let key = rec.key();
        if self.keys.contains(&key) {
            return Ok(false);
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        writeln!(f, "{}", rec.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.keys.insert(key);
        self.records.push(rec);
        Ok(true)
    }

    /// **The 52 Mersenne primes everyone knows are facts, not work.** Every exponent in
    /// [`crate::numtoolz::MERSENNE_EXPONENTS`] goes in as `prime: true, proof: known`, so a search
    /// never spends a Lucas–Lehmer on one, and a ladder can seed on any of them. How many were new.
    pub fn seed_known_mersennes(&mut self) -> Result<usize, String> {
        let mut new = 0;
        for &p in crate::numtoolz::MERSENNE_EXPONENTS.iter() {
            let rec = Record::new("mersenne").int("p", p).int("bits", p).flag("prime", true).str("proof", "known");
            if self.record(rec)? {
                new += 1;
            }
        }
        Ok(new)
    }

    /// The Lucas–Lehmer verdict the store holds for `2^p − 1`, if any.
    pub fn mersenne(&self, p: u32) -> Option<bool> {
        self.of_kind("mersenne").find(|r| r.get_int("p") == Some(p as u64)).and_then(|r| r.get_bool("prime"))
    }

    /// Every exponent with a verdict, with the verdict.
    pub fn mersennes(&self) -> Vec<(u32, bool)> {
        self.of_kind("mersenne").filter_map(|r| Some((r.get_int("p")? as u32, r.get_bool("prime")?))).collect()
    }

    /// **The ladder under `base`, lowest rung first**, rebuilt from the stored rungs. Rungs are
    /// chained by `q`, so a stored climb replays through `verify_certificate` as if just built.
    pub fn ladder(&self, base: &str) -> Vec<Rung> {
        let mut rungs: Vec<Rung> = self
            .of_kind("rung")
            .filter(|r| r.get_str("base") == Some(base))
            .filter_map(|r| {
                let n = match r.big("n") {
                    Some(n) => n,
                    None => (BigUint::from(1u32) << r.get_int("p")?) - BigUint::from(1u32),
                };
                let q = if r.get_str("proof") == Some("seed") { BigUint::zero() } else { r.big("q")? };
                Some(Rung { n, q, a: r.get_int("witness")? as u32 })
            })
            .collect();
        rungs.sort_by(|a, b| a.n.cmp(&b.n));
        // keep one chain: from the seed, each rung's q is the previous rung's n
        let mut chain: Vec<Rung> = Vec::new();
        for r in rungs {
            match chain.last() {
                None if r.is_seed() => chain.push(r),
                Some(top) if r.q == top.n => chain.push(r),
                _ => {}
            }
        }
        chain
    }

    /// The stored recipe for a climb under `base` to `bits`: its offsets and witnesses.
    pub fn recipe_for(&self, base: &str, bits: u64) -> Option<(Vec<i64>, Vec<u32>)> {
        let r = self.of_kind("recipe").find(|r| r.get_str("base") == Some(base) && r.get_int("bits") == Some(bits))?;
        let nums = |k: &str| -> Option<Vec<i64>> { r.get_str(k)?.split(',').filter(|s| !s.is_empty()).map(|s| s.parse().ok()).collect() };
        let offsets = nums("offsets")?;
        let witnesses = nums("witnesses")?.into_iter().map(|w| w as u32).collect();
        Some((offsets, witnesses))
    }

    /// Record every rung of a chain under `base`; how many were new.
    pub fn record_ladder(&mut self, base: &str, chain: &[Rung]) -> Result<usize, String> {
        let mut new = 0;
        for r in chain {
            if self.record(Record::rung(base, r))? {
                new += 1;
            }
        }
        Ok(new)
    }

    /// **Whether the store holds `n` as a proven prime**, under any kind: a rung, a Proth or
    /// twin's upper, a twin's lower, or a known Mersenne. A number the store holds can seed a
    /// climb from another language without being proven again.
    pub fn holds(&self, n: &BigUint) -> bool {
        let bits = n.bits();
        self.records.iter().any(|r| {
            if r.get_int("bits") != Some(bits) {
                return false;
            }
            match r.kind.as_str() {
                "mersenne" => r.get_bool("prime") == Some(true) && *n == (BigUint::from(1u32) << bits) - BigUint::from(1u32),
                "rung" => r.big("n").as_ref() == Some(n) || (r.get_str("proof") == Some("seed") && r.get_int("p").map(|p| (BigUint::from(1u32) << p) - BigUint::from(1u32)) == Some(n.clone())),
                "proth" | "twin" => r.big("upper").as_ref() == Some(n) || r.big("lower").as_ref() == Some(n),
                _ => false,
            }
        })
    }

    /// **Verify a chain against the store**: its seed must be a known or tested Mersenne, or a
    /// prime the store already holds; every rung above must chain and pass its witness. This is
    /// how a ladder from Python or the browser, seeded on a rung of ours, is checked before it
    /// is written down.
    pub fn verify(&self, chain: &[Rung]) -> bool {
        let Some(seed) = chain.first() else { return false };
        if !seed.is_seed() {
            return false;
        }
        let seed_ok = crate::prove::verify_certificate(&chain[..1], None) || self.holds(&seed.n);
        if !seed_ok {
            return false;
        }
        let mut prev = &seed.n;
        for r in &chain[1..] {
            if r.is_seed() || r.q != *prev || !crate::prove::gcd_certify(&r.n, &r.q, r.a) {
                return false;
            }
            prev = &r.n;
        }
        true
    }

    /// The tallest proven prime the store holds with at least `bits` bits, any kind.
    pub fn tallest(&self, bits: u64) -> Option<(&Record, BigUint)> {
        self.records
            .iter()
            .filter(|r| r.get_int("bits").unwrap_or(0) >= bits)
            .filter_map(|r| r.big("n").or_else(|| r.big("upper")).map(|n| (r, n)))
            .max_by(|a, b| a.1.cmp(&b.1))
    }
}

/// Parse one flat JSON object: string, integer and boolean values only, which is all a record
/// holds. Order is kept.
fn parse_flat(line: &str) -> Result<Vec<(String, Field)>, String> {
    let b = line.as_bytes();
    let mut i = 0;
    let skip = |i: &mut usize| while *i < b.len() && b[*i].is_ascii_whitespace() { *i += 1 };
    let string = |i: &mut usize| -> Result<String, String> {
        if b.get(*i) != Some(&b'"') {
            return Err(format!("expected a string at byte {i}"));
        }
        *i += 1;
        let mut out = String::new();
        while *i < b.len() {
            match b[*i] {
                b'"' => {
                    *i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    *i += 1;
                    match b.get(*i) {
                        Some(b'n') => out.push('\n'),
                        Some(b't') => out.push('\t'),
                        Some(&c) => out.push(c as char),
                        None => return Err("unterminated escape".into()),
                    }
                    *i += 1;
                }
                _ => {
                    let start = *i;
                    while *i < b.len() && b[*i] != b'"' && b[*i] != b'\\' {
                        *i += 1;
                    }
                    out.push_str(std::str::from_utf8(&b[start..*i]).map_err(|e| e.to_string())?);
                }
            }
        }
        Err("unterminated string".into())
    };
    skip(&mut i);
    if b.get(i) != Some(&b'{') {
        return Err("a record is a JSON object".into());
    }
    i += 1;
    let mut out = Vec::new();
    loop {
        skip(&mut i);
        match b.get(i) {
            Some(b'}') => return Ok(out),
            Some(b',') => {
                i += 1;
                continue;
            }
            Some(b'"') => {}
            _ => return Err(format!("expected a key at byte {i}")),
        }
        let k = string(&mut i)?;
        skip(&mut i);
        if b.get(i) != Some(&b':') {
            return Err(format!("expected ':' after {k}"));
        }
        i += 1;
        skip(&mut i);
        let v = match b.get(i) {
            Some(b'"') => Field::Str(string(&mut i)?),
            Some(b't') if b[i..].starts_with(b"true") => {
                i += 4;
                Field::Bool(true)
            }
            Some(b'f') if b[i..].starts_with(b"false") => {
                i += 5;
                Field::Bool(false)
            }
            Some(c) if c.is_ascii_digit() => {
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                let s = std::str::from_utf8(&b[start..i]).unwrap();
                match s.parse::<u64>() {
                    Ok(v) => Field::Int(v),
                    Err(_) => Field::Str(s.to_string()),
                }
            }
            _ => return Err(format!("unsupported value for {k} at byte {i}")),
        };
        out.push((k, v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_round_trips_and_keys_the_same_fact_once() {
        let r = Record::mersenne(127, true, 0.001);
        let back = Record::from_json(&r.to_json()).unwrap();
        assert_eq!(r, back);
        assert_eq!(r.key(), Record::mersenne(127, true, 9.0).key());
        let q = Record::new("rung").str("base", "M127").int("bits", 254).str("n", "123").str("q", "77").int("witness", 2).str("proof", "pocklington");
        assert_eq!(Record::from_json(&q.to_json()).unwrap(), q);
        assert!(Record::from_json("{\"bits\": 1}").is_err());
        assert!(Record::from_json("[1]").is_err());
    }

    #[test]
    fn a_decimal_too_big_for_u64_is_kept_as_text() {
        let f = parse_flat("{\"n\": 123456789012345678901234567890}").unwrap();
        assert_eq!(f[0].1, Field::Str("123456789012345678901234567890".into()));
    }
}
