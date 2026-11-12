//! yf-parity: compare yf-core against the TypeScript app on real tournament files.
//!
//!   yf-parity dump <out-dir> <file>...   write <name>.rs.json per file (same shape as rust/oracle/dump.ts)
//!   yf-parity dump <out-dir> --list <f>  same, reading NUL-separated paths from <f>
//!   yf-parity diff <out-dir>             compare every <name>.ts.json with <name>.rs.json
//!
//! Exit code of `diff` is 1 if any file differs.

mod dump;

use std::collections::HashSet;
use std::path::Path;
use std::process::ExitCode;

use serde_json::{json, Value};

const CUR_YF_VERSION: &str = "4.0.18";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("dump") if args.len() == 4 && args[2] == "--list" => {
            let list = std::fs::read_to_string(&args[3]).expect("read file list");
            let files: Vec<String> = list.split('\0').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
            cmd_dump(&args[1], &files)
        }
        Some("dump") if args.len() >= 3 => cmd_dump(&args[1], &args[2..]),
        Some("diff") if args.len() == 2 => cmd_diff(&args[1], false),
        Some("diff-all") if args.len() == 2 => cmd_diff(&args[1], true),
        _ => {
            eprintln!("usage:\n  yf-parity dump <out-dir> <file>...\n  yf-parity diff <out-dir>\n  yf-parity diff-all <out-dir>   (every difference, not just the first few)");
            ExitCode::from(2)
        }
    }
}

/// Same naming as uniqueName() in dump.ts
fn unique_name(file: &str, used: &mut HashSet<String>) -> String {
    let base = Path::new(file).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let base = match base.rfind('.') {
        Some(i) if i + 1 < base.len() => base[..i].to_string(),
        _ => base,
    };
    let mut clean = String::new();
    let mut in_bad = false;
    for c in base.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
            clean.push(c);
            in_bad = false;
        } else if !in_bad {
            clean.push('_');
            in_bad = true;
        }
    }
    let mut name = clean.clone();
    let mut i = 2;
    while used.contains(&name) {
        name = format!("{clean}__{i}");
        i += 1;
    }
    used.insert(name.clone());
    name
}

fn cmd_dump(out_dir: &str, files: &[String]) -> ExitCode {
    std::fs::create_dir_all(out_dir).expect("create out dir");
    let mut used = HashSet::new();
    let mut manifest = vec![];
    let mut failed = 0;
    for file in files {
        let out = format!("{}.rs.json", unique_name(file, &mut used));
        let mut entry = json!({ "file": file, "out": out });
        let dump = match std::fs::read_to_string(file) {
            Err(e) => json!({ "error": format!("read: {e}"), "format": "" }),
            Ok(contents) => {
                let (format, res) = yf_core::open_file(&contents, CUR_YF_VERSION);
                entry["format"] = json!(format.as_str());
                match res {
                    Err(e) => {
                        failed += 1;
                        entry["error"] = json!(e);
                        json!({ "error": e, "format": format.as_str() })
                    }
                    Ok(mut loaded) => {
                        let mut d = dump::dump_tournament(&mut loaded.tournament);
                        if format == yf_core::Format::Yft {
                            d["parserWarnings"] = json!(loaded.warnings);
                        }
                        d["format"] = json!(format.as_str());
                        d
                    }
                }
            }
        };
        std::fs::write(Path::new(out_dir).join(&out), format!("{}\n", serde_json::to_string_pretty(&dump).unwrap())).expect("write dump");
        manifest.push(entry);
    }
    std::fs::write(Path::new(out_dir).join("manifest.rs.json"), serde_json::to_string_pretty(&manifest).unwrap()).expect("write manifest");
    println!("wrote {} dumps to {out_dir} ({failed} failed to load)", manifest.len());
    ExitCode::SUCCESS
}

fn cmd_diff(dir: &str, all: bool) -> ExitCode {
    let read = |name: &str| -> Option<Value> {
        let s = std::fs::read_to_string(Path::new(dir).join(name)).ok()?;
        serde_json::from_str(&s).ok()
    };
    let Some(Value::Array(manifest)) = read("manifest.ts.json") else {
        eprintln!("no manifest.ts.json in {dir}; run the TS oracle first");
        return ExitCode::from(2);
    };
    let (mut same, mut both_fail, mut differ) = (0, 0, 0);
    for entry in &manifest {
        let ts_out = entry["out"].as_str().unwrap_or("");
        let rs_out = ts_out.replace(".ts.json", ".rs.json");
        let file = entry["file"].as_str().unwrap_or("");
        let short = file.rsplit('/').next().unwrap_or(file);
        let (Some(ts), Some(rs)) = (read(ts_out), read(&rs_out)) else {
            differ += 1;
            println!("MISSING  {short}: no {rs_out}");
            continue;
        };
        let ts_err = ts.get("error").and_then(|v| v.as_str());
        let rs_err = rs.get("error").and_then(|v| v.as_str());
        match (ts_err, rs_err) {
            (Some(a), Some(b)) => {
                both_fail += 1;
                if a != b {
                    println!("bothfail {short}\n           ts: {a}\n           rs: {b}");
                }
                continue;
            }
            (Some(a), None) => {
                differ += 1;
                println!("DIFF     {short}: TS failed to load ({a}) but Rust loaded it");
                continue;
            }
            (None, Some(b)) => {
                differ += 1;
                println!("DIFF     {short}: Rust failed to load ({b}) but TS loaded it");
                continue;
            }
            (None, None) => {}
        }
        let mut diffs = vec![];
        compare("", &ts, &rs, &mut diffs);
        if diffs.is_empty() {
            same += 1;
        } else {
            differ += 1;
            println!("DIFF     {short}: {} differences", diffs.len());
            for d in diffs.iter().take(if all { usize::MAX } else { 8 }) {
                println!("           {d}");
            }
        }
    }
    println!(
        "\n{} files: {same} identical, {both_fail} fail to load in both, {differ} differ",
        manifest.len()
    );
    if differ > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.len() > 120 {
        format!("{}…", &s[..s.char_indices().nth(117).map(|x| x.0).unwrap_or(s.len())])
    } else {
        s
    }
}

/// Structural compare. Numbers compare as f64 so `5` (JS) equals `5.0` (serde).
fn compare(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if x.as_f64() != y.as_f64() {
                out.push(format!("{path}: ts={x} rs={y}"));
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            for (k, va) in x {
                match y.get(k) {
                    Some(vb) => compare(&format!("{path}.{k}"), va, vb, out),
                    None => out.push(format!("{path}.{k}: missing in rs (ts={})", short(va))),
                }
            }
            for k in y.keys() {
                if !x.contains_key(k) {
                    out.push(format!("{path}.{k}: missing in ts (rs={})", short(&y[k])));
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}: length ts={} rs={}", x.len(), y.len()));
            }
            for (i, (va, vb)) in x.iter().zip(y.iter()).enumerate() {
                compare(&format!("{path}[{i}]"), va, vb, out);
            }
        }
        _ => {
            if a != b {
                out.push(format!("{path}: ts={} rs={}", short(a), short(b)));
            }
        }
    }
}
