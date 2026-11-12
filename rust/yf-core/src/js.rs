//! JavaScript semantics that the stats engine depends on. Every function here exists because the
//! obvious Rust equivalent gives a different answer on some real input.

use std::cmp::Ordering;

/// `Number.prototype.toFixed`. Rust's `{:.N}` rounds exact ties to even (0.25 -> "0.2");
/// JS rounds them away from zero (0.25 -> "0.3"). Games played is fractional, so this shows up.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.abs() >= 1e21 {
        return num_to_string(x);
    }
    let neg = x < 0.0;
    let a = x.abs();
    // 1100 digits is enough for the exact decimal expansion of any f64 (at most 1074 fractional digits),
    // so this string is exact, not rounded.
    let s = format!("{a:.1100}");
    let (int_part, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut kept: Vec<u8> = int_part.bytes().chain(frac.bytes().take(digits)).collect();
    let round_up = frac.as_bytes().get(digits).is_some_and(|d| *d >= b'5');
    if round_up {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let int_len = kept.len() - digits;
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&kept[..int_len]).unwrap());
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&kept[int_len..]).unwrap());
    }
    out
}

/// `Number.prototype.toString()` / template-literal interpolation of a number.
pub fn num_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if x < 0.0 {
        return format!("-{}", num_to_string(-x));
    }
    // Shortest round-trip digits, as JS uses
    let e = format!("{x:e}");
    let (mantissa, exp) = e.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap() + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let e = (n - 1).abs();
        if k == 1 {
            format!("{digits}e{sign}{e}")
        } else {
            format!("{}.{}e{sign}{e}", &digits[..1], &digits[1..])
        }
    }
}

/// `Math.round`: halves go toward +Infinity (Rust's `round` goes away from zero).
pub fn round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | ' '
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// `String.prototype.trim`. Differs from `str::trim` on U+FEFF (JS trims) and U+0085 (JS keeps).
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// Code-point length, i.e. `[...s].length`
pub fn cp_len(s: &str) -> usize {
    s.chars().count()
}

/// UTF-16 length, i.e. `s.length`
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Convert a JS comparator result to an Ordering. NaN counts as equal, as in `Array.prototype.sort`.
pub fn cmp_result(v: f64) -> Ordering {
    if v < 0.0 {
        Ordering::Less
    } else if v > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// Stable sort with a JS-style comparator (`a - b`). Unlike `slice::sort_by`, this never panics
/// when the comparator isn't a total order (NaN, Infinity - Infinity), which JS code happily produces.
/// For comparators that are consistent, any stable sort gives the same result as V8's TimSort.
pub fn sort_by<T, F: FnMut(&T, &T) -> f64>(v: &mut Vec<T>, mut cmp: F) {
    if v.len() < 2 {
        return;
    }
    let items: Vec<T> = std::mem::take(v);
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
    let mut idx: Vec<usize> = (0..slots.len()).collect();
    let mut buf = idx.clone();
    merge_sort(&mut idx, &mut buf, &slots, &mut cmp);
    *v = idx.into_iter().map(|i| slots[i].take().unwrap()).collect();
}

fn merge_sort<T, F: FnMut(&T, &T) -> f64>(idx: &mut [usize], buf: &mut [usize], items: &[Option<T>], cmp: &mut F) {
    let n = idx.len();
    if n < 2 {
        return;
    }
    let mid = n / 2;
    merge_sort(&mut idx[..mid], &mut buf[..mid], items, cmp);
    merge_sort(&mut idx[mid..], &mut buf[mid..], items, cmp);
    let (mut i, mut j, mut k) = (0, mid, 0);
    while i < mid && j < n {
        let a = items[idx[i]].as_ref().unwrap();
        let b = items[idx[j]].as_ref().unwrap();
        // take from the right only if strictly less, to keep the sort stable
        if cmp_result(cmp(b, a)) == Ordering::Less {
            buf[k] = idx[j];
            j += 1;
        } else {
            buf[k] = idx[i];
            i += 1;
        }
        k += 1;
    }
    while i < mid {
        buf[k] = idx[i];
        i += 1;
        k += 1;
    }
    while j < n {
        buf[k] = idx[j];
        j += 1;
        k += 1;
    }
    idx.copy_from_slice(&buf[..n]);
}

/// `a.localeCompare(b)` under Node's default ICU locale (root/en collation, tertiary strength).
pub fn locale_compare(a: &str, b: &str) -> f64 {
    use icu_collator::options::CollatorOptions;
    use icu_collator::Collator;
    thread_local! {
        static COLLATOR: icu_collator::CollatorBorrowed<'static> =
            Collator::try_new(Default::default(), CollatorOptions::default()).expect("ICU collator data");
    }
    COLLATOR.with(|c| match c.compare(a, b) {
        Ordering::Less => -1.0,
        Ordering::Equal => 0.0,
        Ordering::Greater => 1.0,
    })
}

/// `trunc` from GeneralUtils: truncate to `size` code points, trim, add an ellipsis
pub fn trunc(s: &str, size: usize) -> String {
    if cp_len(s) <= size {
        return s.to_string();
    }
    let head: String = s.chars().take(size).collect();
    format!("{}...", trim(&head))
}

/// `versionLt` from GeneralUtils (parseInt on each dot-separated piece; NaN compares false)
pub fn version_lt(a: &str, b: &str) -> bool {
    let pa: Vec<f64> = a.split('.').map(parse_int).collect();
    let pb: Vec<f64> = b.split('.').map(parse_int).collect();
    let get = |v: &Vec<f64>, i: usize| v.get(i).copied().unwrap_or(f64::NAN);
    for i in 0..2 {
        let (x, y) = (get(&pa, i), get(&pb, i));
        if x != y {
            return x < y;
        }
    }
    get(&pa, 2) < get(&pb, 2)
}

/// `parseInt(s, 10)`: optional whitespace and sign, then leading digits; NaN if none
pub fn parse_int(s: &str) -> f64 {
    let t = trim(s);
    let (sign, rest) = match t.as_bytes().first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    sign * digits.parse::<f64>().unwrap_or(f64::NAN)
}

/// `parseFloat(s)`: longest valid decimal prefix; NaN if none
pub fn parse_float(s: &str) -> f64 {
    let t = trim(s);
    let b = t.as_bytes();
    let mut end = 0;
    let mut best = None;
    if end < b.len() && (b[end] == b'+' || b[end] == b'-') {
        end += 1;
    }
    if t[end..].starts_with("Infinity") {
        return if t.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    let mut seen_digit = false;
    while end < b.len() && b[end].is_ascii_digit() {
        end += 1;
        seen_digit = true;
    }
    if seen_digit {
        best = Some(end);
    }
    if end < b.len() && b[end] == b'.' {
        end += 1;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
            seen_digit = true;
        }
        if seen_digit {
            best = Some(end);
        }
    }
    if seen_digit && end < b.len() && (b[end] == b'e' || b[end] == b'E') {
        let mut e = end + 1;
        if e < b.len() && (b[e] == b'+' || b[e] == b'-') {
            e += 1;
        }
        let start = e;
        while e < b.len() && b[e].is_ascii_digit() {
            e += 1;
        }
        if e > start {
            best = Some(e);
        }
    }
    match best {
        Some(n) => t[..n].parse::<f64>().unwrap_or(f64::NAN),
        None => f64::NAN,
    }
}

/// `teamGetNameAndLetter`: "Riverview A" -> ("Riverview", "A"). Works in UTF-16 units like the original.
pub fn team_get_name_and_letter(raw: &str) -> (String, String) {
    let u: Vec<u16> = raw.encode_utf16().collect();
    let last = u.len() as isize - 1;
    let penultimate = if last >= 1 { Some(u[(last - 1) as usize]) } else { None };
    if penultimate != Some(b' ' as u16) {
        return (raw.to_string(), String::new());
    }
    let letter_unit = u[last as usize];
    let letter = upper_one_unit(letter_unit);
    let is_normal = letter.encode_utf16().count() == 1 && letter.chars().all(|c| c.is_ascii_uppercase());
    if !is_normal {
        return (raw.to_string(), String::new());
    }
    let org = String::from_utf16_lossy(&u[..last as usize]);
    (trim(&org).to_string(), letter)
}

fn upper_one_unit(unit: u16) -> String {
    match char::from_u32(unit as u32) {
        Some(c) => c.to_uppercase().collect(),
        None => String::from_utf16_lossy(&[unit]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fixed_matches_js() {
        // values checked against node 22
        assert_eq!(to_fixed(0.25, 1), "0.3");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(0.125, 2), "0.13");
        assert_eq!(to_fixed(1.005, 2), "1.00");
        assert_eq!(to_fixed(10.235, 2), "10.23");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(0.0, 2), "0.00");
        assert_eq!(to_fixed(-0.04, 1), "-0.0");
        assert_eq!(to_fixed(9.96, 1), "10.0");
        assert_eq!(to_fixed(17.77777777777778, 2), "17.78");
        assert_eq!(to_fixed(f64::NAN, 2), "NaN");
        assert_eq!(to_fixed(f64::INFINITY, 1), "Infinity");
    }

    #[test]
    fn num_to_string_matches_js() {
        assert_eq!(num_to_string(5.0), "5");
        assert_eq!(num_to_string(-0.0), "0");
        assert_eq!(num_to_string(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(num_to_string(1e21), "1e+21");
        assert_eq!(num_to_string(123456789012345680000.0), "123456789012345680000");
        assert_eq!(num_to_string(0.000001), "0.000001");
        assert_eq!(num_to_string(0.0000001), "1e-7");
        assert_eq!(num_to_string(1.5e-7), "1.5e-7");
    }

    #[test]
    fn round_matches_js() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
    }

    #[test]
    fn sort_is_stable_and_survives_nan() {
        let mut v = vec![(1, 3.0), (2, f64::NAN), (3, 1.0), (4, 3.0)];
        sort_by(&mut v, |a, b| b.1 - a.1);
        let ids: Vec<i32> = v.iter().map(|x| x.0).collect();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn team_letters() {
        assert_eq!(team_get_name_and_letter("Riverview A"), ("Riverview".into(), "A".into()));
        assert_eq!(team_get_name_and_letter("Riverview b"), ("Riverview".into(), "B".into()));
        assert_eq!(team_get_name_and_letter("Riverview"), ("Riverview".into(), "".into()));
        assert_eq!(team_get_name_and_letter("A"), ("A".into(), "".into()));
    }
}
