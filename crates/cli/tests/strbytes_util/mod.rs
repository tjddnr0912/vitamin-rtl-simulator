//! Raw-byte harness for the Q-STRBYTES pins (`strbytes_*.rs`).
//!
//! RTL text is bytes from its producer to the sink write. A lossy reader cannot see a
//! byte >= 0x80 move (`String::from_utf8_lossy` folds every bad byte into U+FFFD), so
//! this harness never decodes: it spawns the binary and reads stdout, stderr, the
//! `--log` file and the side files a design writes as `Vec<u8>`.
//!
//! A cell prints one tagged line per value: `@TAG ...` ([`Mode::At`]) or `TAG ...` at the
//! start of the message ([`Mode::Word`]). A diagnostic line's `file:l:c: sev[VITA-..]
//! MNEM: ` head and its ` [in X]` / ` [at time N]` tail are not part of the content.
//! A repeated tag is `TAG#2`, `TAG#3`, ... in output order.
//!
//! Every lane must print the same content: native, interp, vm, the JIT (built with
//! `--features jit`: `VITA_JIT=1` on native and vm), the staged chain `vcmp -> velab ->
//! vrun` and the `--log` file of a native run (which holds stdout and the
//! diagnostics; a write to the STDERR descriptor is not logged).

#![allow(dead_code)]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    At,
    Word,
}

/// One design: its source, input files, an optional plusarg (raw bytes) and whether
/// the staged chain runs it.
pub struct Cell<'a> {
    pub src: &'a str,
    pub inputs: &'a [(&'a str, &'a [u8])],
    pub plus: Option<&'a [u8]>,
    pub mode: Mode,
    pub staged: bool,
}

/// What one lane printed and wrote.
pub struct Run {
    pub lane: String,
    pub rc: Option<i32>,
    pub out: Vec<u8>,
    pub files: HashMap<String, Vec<u8>>,
}

fn scratch() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strbytes_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn setup(c: &Cell) -> PathBuf {
    let d = scratch();
    std::fs::write(d.join("t.sv"), c.src).unwrap();
    for (n, b) in c.inputs {
        std::fs::write(d.join(n), b).unwrap();
    }
    d
}

fn written(d: &Path, c: &Cell) -> HashMap<String, Vec<u8>> {
    let mut m = HashMap::new();
    for e in std::fs::read_dir(d).unwrap() {
        let e = e.unwrap();
        let n = e.file_name().to_string_lossy().into_owned();
        if n == "t.sv"
            || n.starts_with("t.v")
            || c.inputs.iter().any(|(i, _)| *i == n)
            || !e.path().is_file()
        {
            continue;
        }
        m.insert(n, std::fs::read(e.path()).unwrap());
    }
    m
}

fn exec(d: &Path, args: &[&OsStr], env: &[(&str, &str)]) -> (Option<i32>, Vec<u8>) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.args(args).current_dir(d);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let o = cmd.output().expect("run vita");
    let mut out = o.stdout;
    out.extend_from_slice(&o.stderr);
    (o.status.code(), out)
}

/// One-shot run: `vita --backend <be> -Wno-W1017 [--log log.txt] t.sv [plusarg]`.
pub fn one_shot(c: &Cell, be: &str, env: &[(&str, &str)], log: bool) -> Run {
    let d = setup(c);
    let mut a: Vec<&OsStr> = vec![
        OsStr::new("--backend"),
        OsStr::new(be),
        OsStr::new("-Wno-W1017"),
    ];
    if log {
        a.extend([OsStr::new("--log"), OsStr::new("log.txt")]);
    }
    a.push(OsStr::new("t.sv"));
    if let Some(p) = c.plus {
        a.push(OsStr::from_bytes(p));
    }
    let (rc, out) = exec(&d, &a, env);
    let files = written(&d, c);
    let _ = std::fs::remove_dir_all(&d);
    let jit = env.iter().any(|(k, _)| *k == "VITA_JIT");
    let lane = format!(
        "{}{be}{}",
        if jit { "jit " } else { "" },
        if log { " --log" } else { "" }
    );
    Run {
        lane,
        rc,
        out,
        files,
    }
}

/// The staged chain; the stages' outputs concatenated (velab prints elaboration lines).
pub fn staged(c: &Cell) -> Run {
    let d = setup(c);
    let mut out = Vec::new();
    let mut rc = Some(0);
    let stages: [&[&str]; 3] = [
        &["vcmp", "-Wno-W1017", "-o", "t.vu", "t.sv"],
        &["velab", "-Wno-W1017", "-o", "t.velab", "t.vu"],
        &["vrun", "-Wno-W1017", "t.velab"],
    ];
    for (i, s) in stages.iter().enumerate() {
        let mut a: Vec<&OsStr> = s.iter().map(OsStr::new).collect();
        if i == 2 {
            if let Some(p) = c.plus {
                a.push(OsStr::from_bytes(p));
            }
        }
        let (r, o) = exec(&d, &a, &[]);
        out.extend_from_slice(&o);
        rc = r;
        if r != Some(0) {
            break;
        }
    }
    let files = written(&d, c);
    let _ = std::fs::remove_dir_all(&d);
    Run {
        lane: "staged".into(),
        rc,
        out,
        files,
    }
}

/// Every lane of `c`.
pub fn lanes(c: &Cell) -> Vec<Run> {
    let mut v = Vec::new();
    for be in ["native", "interp", "vm"] {
        v.push(one_shot(c, be, &[], false));
    }
    if cfg!(feature = "jit") {
        for be in ["native", "vm"] {
            v.push(one_shot(c, be, &[("VITA_JIT", "1")], false));
        }
    }
    if c.staged {
        v.push(staged(c));
    }
    v
}

fn strip_diag(l: &[u8]) -> (usize, usize) {
    let (mut s, mut e) = (0, l.len());
    if let Some(i) = find(l, b"[VITA-") {
        if let Some(j) = find(&l[i..], b"] ") {
            let k = i + j + 2;
            if let Some(m) = find(&l[k..], b": ") {
                s = k + m + 2;
            }
        }
        // ` [in X]` then ` [at time N]` at the end
        for suf in [&b" [at time "[..], &b" [in "[..]] {
            if l[..e].ends_with(b"]") {
                if let Some(p) = rfind(&l[s..e], suf) {
                    e = s + p;
                }
            }
        }
    }
    (s, e)
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

fn rfind(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).rposition(|w| w == n)
}

/// tag -> content (the tag's line from the tag on, diagnostic head / tail removed).
pub fn tags(out: &[u8], mode: Mode) -> HashMap<String, Vec<u8>> {
    let mut m = HashMap::new();
    let mut cnt: HashMap<String, usize> = HashMap::new();
    for l in out.split(|&b| b == b'\n') {
        let (s, e) = strip_diag(l);
        let body = &l[s..e];
        let (start, tag) = match mode {
            Mode::At => {
                let Some(i) = body.iter().position(|&b| b == b'@') else {
                    continue;
                };
                let t: Vec<u8> = body[i + 1..]
                    .iter()
                    .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_')
                    .copied()
                    .collect();
                if t.is_empty() || !t[0].is_ascii_uppercase() {
                    continue;
                }
                (i, t)
            }
            Mode::Word => {
                if body.starts_with(b"simulation ended") || body.starts_with(b"errors=") {
                    continue;
                }
                let t: Vec<u8> = body.iter().take_while(|&&b| b != b' ').copied().collect();
                let ok = !t.is_empty()
                    && t.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    && (t[0].is_ascii_uppercase() || t.contains(&b'_'));
                if !ok {
                    continue;
                }
                (0, t)
            }
        };
        let tag = String::from_utf8(tag).unwrap();
        let k = cnt.entry(tag.clone()).or_insert(0);
        *k += 1;
        let key = if *k == 1 { tag } else { format!("{tag}#{k}") };
        m.insert(key, body[start..].to_vec());
    }
    m
}

/// Bytes as printable ASCII with `\xNN` for everything else (a failure message that
/// shows the bytes, not a lossy rendering of them).
pub fn esc(b: &[u8]) -> String {
    b.iter()
        .map(|&c| {
            if (0x20..0x7f).contains(&c) && c != b'\\' {
                (c as char).to_string()
            } else {
                format!("\\x{c:02x}")
            }
        })
        .collect()
}

/// Every tagged line of `want` on every lane (and in a native run's `--log` file), and
/// every side file, byte for byte.
pub fn check(c: &Cell, want: &[(&str, &[u8])], files: &[(&str, &[u8])]) {
    let mut runs = lanes(c);
    let logged = one_shot(c, "native", &[], true);
    let log = logged.files.get("log.txt").cloned().unwrap_or_default();
    runs.push(Run {
        lane: "native --log file".into(),
        rc: logged.rc,
        out: log,
        files: HashMap::new(),
    });
    let mut bad = Vec::new();
    for r in &runs {
        let t = tags(&r.out, c.mode);
        let is_log = r.lane == "native --log file";
        for (tag, w) in want {
            let got = match tag.strip_prefix('#') {
                // `#n`: the n-th output line (an untagged line), not counting the
                // `errors=` trailers a staged chain prints per stage
                Some(n) => r
                    .out
                    .split(|&b| b == b'\n')
                    .filter(|l| !l.starts_with(b"errors="))
                    .nth(n.parse::<usize>().unwrap() - 1)
                    .map(esc),
                None => t.get(*tag).map(|g| esc(g)),
            };
            // the `--log` file holds stdout and the diagnostics, not a write to the
            // STDERR descriptor: a tag absent there is not a mismatch
            if got.is_none() && is_log {
                continue;
            }
            let got = got.unwrap_or_else(|| "<missing>".into());
            if got != esc(w) {
                bad.push(format!(
                    "[{}] {tag}\n   got  {got}\n   want {}",
                    r.lane,
                    esc(w)
                ));
            }
        }
        if is_log {
            continue;
        }
        for (f, w) in files {
            let got = r
                .files
                .get(*f)
                .map(|g| esc(g))
                .unwrap_or_else(|| "<missing>".into());
            if got != esc(w) {
                bad.push(format!(
                    "[{}] file {f}\n   got  {got}\n   want {}",
                    r.lane,
                    esc(w)
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} byte mismatches:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
