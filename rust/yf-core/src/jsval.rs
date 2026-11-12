//! Reading loosely-typed JSON the way JS code reads it: truthiness, `+x` coercion, `for...in` key order.

use serde_json::{Map, Value};

/// A JSON value as JS sees it, including `undefined` for a missing key
#[derive(Clone, Debug)]
pub enum Jv<'a> {
    Undef,
    Null,
    Bool(bool),
    Num(f64),
    Str(&'a str),
    Arr(&'a Vec<Value>),
    Obj(&'a Map<String, Value>),
}

pub fn jv(v: Option<&Value>) -> Jv<'_> {
    match v {
        None => Jv::Undef,
        Some(Value::Null) => Jv::Null,
        Some(Value::Bool(b)) => Jv::Bool(*b),
        Some(Value::Number(n)) => Jv::Num(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => Jv::Str(s),
        Some(Value::Array(a)) => Jv::Arr(a),
        Some(Value::Object(o)) => Jv::Obj(o),
    }
}

pub fn get<'a>(obj: &'a Value, key: &str) -> Jv<'a> {
    jv(obj.as_object().and_then(|o| o.get(key)))
}

impl<'a> Jv<'a> {
    pub fn truthy(&self) -> bool {
        match self {
            Jv::Undef | Jv::Null => false,
            Jv::Bool(b) => *b,
            Jv::Num(n) => *n != 0.0 && !n.is_nan(),
            Jv::Str(s) => !s.is_empty(),
            Jv::Arr(_) | Jv::Obj(_) => true,
        }
    }

    /// Unary `+x` (ToNumber)
    pub fn to_number(&self) -> f64 {
        match self {
            Jv::Undef => f64::NAN,
            Jv::Null => 0.0,
            Jv::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Jv::Num(n) => *n,
            Jv::Str(s) => string_to_number(s),
            Jv::Arr(a) => match a.len() {
                0 => 0.0,
                1 => jv(a.first()).to_js_string().map(|s| string_to_number(&s)).unwrap_or(f64::NAN),
                _ => f64::NAN,
            },
            Jv::Obj(_) => f64::NAN,
        }
    }

    fn to_js_string(&self) -> Option<String> {
        match self {
            Jv::Str(s) => Some(s.to_string()),
            Jv::Num(n) => Some(crate::js::num_to_string(*n)),
            Jv::Bool(b) => Some(b.to_string()),
            Jv::Null => Some(String::new()),
            _ => None,
        }
    }

    /// A value assigned to a `number | undefined` field. Strings, nulls etc. would make the TS
    /// arithmetic go sideways (string concatenation), so we refuse them rather than guess.
    pub fn opt_num(&self, field: &str) -> Result<Option<f64>, String> {
        match self {
            Jv::Undef => Ok(None),
            Jv::Num(n) => Ok(Some(*n)),
            other => Err(format!("unsupported non-numeric value in numeric field {field}: {other:?}")),
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Jv::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_undef(&self) -> bool {
        matches!(self, Jv::Undef)
    }
}

/// ToNumber applied to a string
pub fn string_to_number(s: &str) -> f64 {
    let t = crate::js::trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(h, 16).map(|v| v as f64).unwrap_or(f64::NAN);
    }
    if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(b, 2).map(|v| v as f64).unwrap_or(f64::NAN);
    }
    if let Some(o) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(o, 8).map(|v| v as f64).unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // StrDecimalLiteral: digits, optional fraction, optional exponent. Rust's parser also accepts
    // "inf"/"nan", which JS doesn't, so screen those out.
    let ok = t.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'));
    if !ok {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// Keys of an object in `for...in` order: integer-like keys ascending, then the rest in insertion order
pub fn js_keys(o: &Map<String, Value>) -> Vec<&String> {
    let mut ints: Vec<(u32, &String)> = vec![];
    let mut rest: Vec<&String> = vec![];
    for k in o.keys() {
        match canonical_index(k) {
            Some(i) => ints.push((i, k)),
            None => rest.push(k),
        }
    }
    ints.sort_by_key(|x| x.0);
    ints.into_iter().map(|x| x.1).chain(rest).collect()
}

fn canonical_index(k: &str) -> Option<u32> {
    let n: u32 = k.parse().ok()?;
    if n == u32::MAX || n.to_string() != k {
        return None;
    }
    Some(n)
}
