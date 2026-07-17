//! VCD → FST transcode (G2 waveform breadth).
//!
//! vita's simulation core emits a fully-correct VCD (the verified waveform
//! oracle). Rather than thread a second binary sink through every value-change
//! call site — which would duplicate the delicate 4-state / real / dump-control
//! logic and invite a silent-wrong divergence — an FST target is produced by
//! **transcoding that VCD**: FST ≡ VCD by construction. Every dump semantic
//! (`$dumpoff`→all-x, `$dumpon`, `$dumpall`, `$dumplimit`) is already baked into
//! the VCD value-change stream, so replaying that stream into an FST preserves
//! it for free. Correctness is pinned by reading the FST back with the
//! independent `fst-reader` crate and asserting waveform equivalence to the VCD
//! (see the cli `fst_waveform` test).
//!
//! The FST bytes themselves are produced by the pure-Rust `fst-writer` crate
//! (BSD-3-Clause, Cornell / Kevin Laeufer), so vita never hand-rolls the
//! compressed binary geometry/hierarchy/value-change blocks.

use fst_writer::{
    open_fst, FstFileType, FstInfo, FstScopeType, FstSignalId, FstSignalType, FstVarDirection,
    FstVarType,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A whitespace token stream over a `BufRead`, yielding one line into memory at
/// a time (bounded memory for large dumps) while still handling a value/id pair
/// that straddles a line boundary.
struct Tokens<R: BufRead> {
    r: R,
    line: Vec<String>,
    idx: usize,
}

impl<R: BufRead> Tokens<R> {
    fn new(r: R) -> Self {
        Tokens {
            r,
            line: Vec::new(),
            idx: 0,
        }
    }
    fn next_tok(&mut self) -> std::io::Result<Option<String>> {
        loop {
            if self.idx < self.line.len() {
                let t = std::mem::take(&mut self.line[self.idx]);
                self.idx += 1;
                return Ok(Some(t));
            }
            let mut buf = String::new();
            let n = self.r.read_line(&mut buf)?;
            if n == 0 {
                return Ok(None);
            }
            self.line = buf.split_whitespace().map(str::to_string).collect();
            self.idx = 0;
        }
    }
}

/// Map a VCD scope-type keyword to the FST enum (unknown → Module, the neutral
/// default GTKWave also falls back to).
fn scope_type(kw: &str) -> FstScopeType {
    match kw {
        "task" => FstScopeType::Task,
        "function" => FstScopeType::Function,
        "fork" => FstScopeType::Fork,
        "begin" => FstScopeType::Begin,
        "generate" => FstScopeType::Generate,
        "struct" => FstScopeType::Struct,
        "union" => FstScopeType::Union,
        "class" => FstScopeType::Class,
        "interface" => FstScopeType::Interface,
        "package" => FstScopeType::Package,
        "program" => FstScopeType::Program,
        _ => FstScopeType::Module,
    }
}

/// Map a VCD var-type keyword to the FST enum (unknown → Wire).
fn var_type(kw: &str) -> FstVarType {
    match kw {
        "reg" => FstVarType::Reg,
        "integer" => FstVarType::Integer,
        "real" => FstVarType::Real,
        "realtime" => FstVarType::RealTime,
        "time" => FstVarType::Time,
        "parameter" => FstVarType::Parameter,
        "supply0" => FstVarType::Supply0,
        "supply1" => FstVarType::Supply1,
        "tri" => FstVarType::Tri,
        "triand" => FstVarType::TriAnd,
        "trior" => FstVarType::TriOr,
        "trireg" => FstVarType::TriReg,
        "tri0" => FstVarType::Tri0,
        "tri1" => FstVarType::Tri1,
        "wand" => FstVarType::Wand,
        "wor" => FstVarType::Wor,
        "wire" => FstVarType::Wire,
        "event" => FstVarType::Event,
        _ => FstVarType::Wire,
    }
}

/// VCD `$timescale` unit/multiplier → the single power-of-ten exponent FST
/// stores (all file times are then in units of 10^exponent seconds). VCD admits
/// only 1|10|100 × s..fs, so the multiplier folds cleanly into the exponent
/// (`10ns` = 10^-8 s → -8). Unknown/garbage → -9 (ns), the vita default.
fn timescale_exponent(spec: &str) -> i8 {
    // spec is the concatenation of the tokens between `$timescale` and `$end`,
    // e.g. "1ns" or "10 ns" → we strip whitespace already (tokens joined).
    let s = spec.trim();
    let digits_end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num, unit) = s.split_at(digits_end);
    let mult: i8 = match num {
        "1" => 0,
        "10" => 1,
        "100" => 2,
        _ => 0,
    };
    let base: i8 = match unit.trim() {
        "s" => 0,
        "ms" => -3,
        "us" => -6,
        "ns" => -9,
        "ps" => -12,
        "fs" => -15,
        _ => -9,
    };
    base + mult
}

/// Left-extend a VCD vector value to the signal's declared bit width using the
/// IEEE-1364 §18.2.1 extension rule (a value shorter than the vector is padded
/// on the left: `x`→`x`, `z`→`z`, else `0`). vita already emits full-width
/// vectors, but this keeps the transcode correct for any minimal-form producer
/// and guarantees `value.len() == width` so fst-writer never has to guess.
fn extend_vector(bits: &[u8], width: usize) -> Vec<u8> {
    let mut out = if bits.len() >= width {
        // already full (or over-wide → trust the low `width`, MSB-aligned):
        // VCD never over-emits, so this branch is just the equality fast path.
        bits[bits.len() - width..].to_vec()
    } else {
        let pad = match bits.first() {
            Some(b'x') | Some(b'X') => b'x',
            Some(b'z') | Some(b'Z') => b'z',
            _ => b'0',
        };
        let mut v = vec![pad; width - bits.len()];
        v.extend_from_slice(bits);
        v
    };
    // normalise case on EVERY branch: fst 4-state chars are lowercase. (The
    // fast path used to return `X`/`Z` verbatim — latent only, since vita's own
    // writer always emits lowercase full-width, but the doc-comment promises
    // correctness for any minimal-form producer.)
    for c in out.iter_mut() {
        if *c == b'X' {
            *c = b'x';
        } else if *c == b'Z' {
            *c = b'z';
        }
    }
    out
}

/// Transcode the VCD at `vcd_path` into an FST at `fst_path`. `version`/`date`
/// populate the FST header meta. Errors (I/O, malformed FST) surface to the
/// caller so a failed transcode is loud, never a silently-missing waveform.
pub fn transcode_vcd_to_fst(
    vcd_path: &Path,
    fst_path: &Path,
    version: &str,
    date: &str,
) -> std::io::Result<()> {
    let f = std::fs::File::open(vcd_path)?;
    let mut toks = Tokens::new(BufReader::new(f));

    // ── Phase A: header (up to `$enddefinitions`) ───────────────────────────
    let mut timescale = String::from("1ns");
    // Defer opening the FST until we know the timescale (it lands in the header
    // meta), so buffer the scope/var ops we discover first.
    enum HierOp {
        Scope(FstScopeType, String),
        UpScope,
        Var(FstVarType, String, u32, String), // tpe, name, width, id-code
    }
    let mut hier: Vec<HierOp> = Vec::new();

    while let Some(tok) = toks.next_tok()? {
        match tok.as_str() {
            "$enddefinitions" => {
                // consume the trailing `$end`
                let _ = toks.next_tok()?;
                break;
            }
            "$timescale" => {
                let mut parts = String::new();
                while let Some(t) = toks.next_tok()? {
                    if t == "$end" {
                        break;
                    }
                    parts.push_str(&t);
                }
                timescale = parts;
            }
            "$scope" => {
                let ty = toks.next_tok()?.unwrap_or_default();
                let name = toks.next_tok()?.unwrap_or_default();
                // consume `$end`
                let _ = toks.next_tok()?;
                hier.push(HierOp::Scope(scope_type(&ty), name));
            }
            "$upscope" => {
                let _ = toks.next_tok()?; // `$end`
                hier.push(HierOp::UpScope);
            }
            "$var" => {
                // $var <type> <width> <id> <name> [<range>] $end
                let ty = toks.next_tok()?.unwrap_or_default();
                let width: u32 = toks.next_tok()?.and_then(|s| s.parse().ok()).unwrap_or(1);
                let id = toks.next_tok()?.unwrap_or_default();
                let mut name = toks.next_tok()?.unwrap_or_default();
                // Optional reference range and the closing `$end` follow; fold a
                // bit-range into the name (GTKWave convention: `cnt[3:0]`).
                for t in std::iter::from_fn(|| toks.next_tok().ok().flatten()) {
                    if t == "$end" {
                        break;
                    }
                    name.push_str(&t);
                }
                hier.push(HierOp::Var(var_type(&ty), name, width, id));
            }
            // $date/$version/$comment blocks: skip to their `$end`.
            "$date" | "$version" | "$comment" => {
                while let Some(t) = toks.next_tok()? {
                    if t == "$end" {
                        break;
                    }
                }
            }
            _ => { /* stray token — ignore */ }
        }
    }

    let info = FstInfo {
        start_time: 0,
        timescale_exponent: timescale_exponent(&timescale),
        version: version.to_string(),
        date: date.to_string(),
        file_type: FstFileType::Verilog,
    };
    let mut hw =
        open_fst(fst_path, &info).map_err(|e| std::io::Error::other(format!("fst open: {e}")))?;

    // id-code (VCD identifier) → FST signal handle. A repeated id-code is an
    // alias (one net dumped under several names) → same handle.
    let mut ids: HashMap<String, FstSignalId> = HashMap::new();
    // id-code → declared bit width (0 ⇒ real), for value extension in the body.
    let mut widths: HashMap<String, u32> = HashMap::new();
    let mut is_real: HashMap<String, bool> = HashMap::new();

    for op in hier {
        match op {
            HierOp::Scope(ty, name) => hw
                .scope(&name, "", ty)
                .map_err(|e| std::io::Error::other(format!("fst scope: {e}")))?,
            HierOp::UpScope => hw
                .up_scope()
                .map_err(|e| std::io::Error::other(format!("fst upscope: {e}")))?,
            HierOp::Var(ty, name, width, id) => {
                let real = matches!(ty, FstVarType::Real | FstVarType::RealTime);
                let sig_ty = if real {
                    FstSignalType::real()
                } else {
                    FstSignalType::bit_vec(width)
                };
                let alias = ids.get(&id).copied();
                let handle = hw
                    .var(&name, sig_ty, ty, FstVarDirection::Implicit, alias)
                    .map_err(|e| std::io::Error::other(format!("fst var: {e}")))?;
                ids.entry(id.clone()).or_insert(handle);
                widths
                    .entry(id.clone())
                    .or_insert(if real { 0 } else { width });
                is_real.entry(id).or_insert(real);
            }
        }
    }

    let mut body = hw
        .finish()
        .map_err(|e| std::io::Error::other(format!("fst finish header: {e}")))?;

    // ── Phase B: value changes ──────────────────────────────────────────────
    while let Some(tok) = toks.next_tok()? {
        let bytes = tok.as_bytes();
        match bytes[0] {
            b'#' => {
                if let Ok(t) = tok[1..].parse::<u64>() {
                    body.time_change(t)
                        .map_err(|e| std::io::Error::other(format!("fst time: {e}")))?;
                }
            }
            b'b' | b'B' => {
                // vector: `b<bits>` then the id-code as the next token
                let id = toks.next_tok()?.unwrap_or_default();
                if let Some(&handle) = ids.get(&id) {
                    if *is_real.get(&id).unwrap_or(&false) {
                        // A REAL dumped as a bit-vector only happens for the
                        // "unknown real": `$dumpoff` emits an all-x vector (vita)
                        // rather than `rx`. An FST real holds an f64, not x-bits,
                        // so map it to NaN — the real-domain unknown, exactly as
                        // iverilog's `rNaN` reads back. (Feeding the x-bytes to a
                        // real signal_change would reinterpret them as a garbage
                        // float — a silent-wrong waveform.)
                        body.signal_change(handle, &f64::NAN.to_le_bytes())
                            .map_err(|e| std::io::Error::other(format!("fst real-x: {e}")))?;
                    } else {
                        let width = *widths.get(&id).unwrap_or(&1) as usize;
                        let val = extend_vector(&bytes[1..], width.max(1));
                        body.signal_change(handle, &val)
                            .map_err(|e| std::io::Error::other(format!("fst vec: {e}")))?;
                    }
                }
            }
            b'r' | b'R' => {
                // real: `r<float>` then the id-code
                let id = toks.next_tok()?.unwrap_or_default();
                if let Some(&handle) = ids.get(&id) {
                    let v: f64 = tok[1..].parse().unwrap_or(0.0);
                    body.signal_change(handle, &v.to_le_bytes())
                        .map_err(|e| std::io::Error::other(format!("fst real: {e}")))?;
                }
            }
            b'0' | b'1' | b'x' | b'X' | b'z' | b'Z' => {
                // scalar: `<value><id-code>` in ONE token
                let v = match bytes[0] {
                    b'X' => b'x',
                    b'Z' => b'z',
                    other => other,
                };
                let id = &tok[1..];
                if let Some(&handle) = ids.get(id) {
                    body.signal_change(handle, &[v])
                        .map_err(|e| std::io::Error::other(format!("fst scalar: {e}")))?;
                }
            }
            // `$dumpvars`/`$dumpoff`/`$dumpon`/`$dumpall`/`$end`/`$comment` — the
            // control keywords carry no value themselves; their inner value lines
            // are handled by the arms above. A `$comment … $end` in the body must
            // skip its content.
            b'$' => {
                if tok == "$comment" {
                    while let Some(t) = toks.next_tok()? {
                        if t == "$end" {
                            break;
                        }
                    }
                }
            }
            _ => { /* unknown record — ignore (loud path is the round-trip test) */ }
        }
    }

    body.finish()
        .map_err(|e| std::io::Error::other(format!("fst finish body: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn extend_vector_normalises_case_on_every_branch() {
        // The equality/over-wide fast path used to return `X`/`Z` verbatim
        // (only the pad branch folded case) — latent, since vita's own writer
        // emits lowercase full-width, but the fn documents itself as correct
        // for ANY minimal-form producer.
        assert_eq!(super::extend_vector(b"XZ01", 4), b"xz01".to_vec());
        assert_eq!(super::extend_vector(b"XZ01", 3), b"z01".to_vec());
        assert_eq!(super::extend_vector(b"X1", 4), b"xxx1".to_vec());
        assert_eq!(super::extend_vector(b"Z1", 4), b"zzz1".to_vec());
    }

    use super::*;
    use fst_reader::{FstFilter, FstHierarchyEntry, FstReader, FstSignalValue};
    use std::sync::atomic::{AtomicU32, Ordering};

    static N: AtomicU32 = AtomicU32::new(0);

    fn tmp(ext: &str) -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("vita_fst_{}_{n}.{ext}", std::process::id()))
    }

    /// Read an FST back and return sorted (time, var-name, value-string) tuples —
    /// the effective waveform, independent of vita's writer.
    fn read_fst(path: &std::path::Path) -> Vec<(u64, String, String)> {
        let f = std::io::BufReader::new(std::fs::File::open(path).unwrap());
        let mut r = FstReader::open(f).unwrap();
        let mut idx2name: std::collections::HashMap<usize, String> =
            std::collections::HashMap::new();
        r.read_hierarchy(|e| {
            if let FstHierarchyEntry::Var { name, handle, .. } = e {
                idx2name.entry(handle.get_index()).or_insert(name);
            }
        })
        .unwrap();
        let mut out = Vec::new();
        r.read_signals(&FstFilter::all(), |t, h, v| {
            let s = match v {
                FstSignalValue::String(b) => String::from_utf8_lossy(b).into_owned(),
                FstSignalValue::Real(x) => format!("r{x}"),
            };
            out.push((t, idx2name[&h.get_index()].clone(), s));
        })
        .unwrap();
        out.sort();
        out
    }

    /// The transcode's correct-or-loud pin: a representative vita VCD (scalar +
    /// 4-bit + real + 8-bit vector, x/z bits, real change) transcodes to an FST
    /// whose read-back waveform matches the VCD exactly. Values are the effective
    /// per-time signal values (the `$dumpvars` all-x snapshot folds into the `#0`
    /// post-init frame — last write at t0 wins, as GTKWave would render).
    #[test]
    fn vcd_to_fst_roundtrips() {
        let vcd = "\
$date\n   vitamin-sim\n$end\n\
$version\n   vitamin-sim 0.0.0\n$end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var reg 1 ! clk $end\n\
$var reg 4 \" cnt [3:0] $end\n\
$var real 64 # r $end\n\
$var reg 8 $ wide [7:0] $end\n\
$upscope $end\n\
$enddefinitions $end\n\
$dumpvars\nx!\nbxxxx \"\nr0 #\nbxxxxxxxx $\n$end\n\
#0\n0!\nb0000 \"\nb00000000 $\n\
#5\n1!\nb1010 \"\nr1.5 #\nb11111111 $\n\
#10\n0!\nbxz01 \"\nb00001111 $\n";
        let vcd_path = tmp("vcd");
        let fst_path = tmp("fst");
        std::fs::write(&vcd_path, vcd).unwrap();
        transcode_vcd_to_fst(&vcd_path, &fst_path, "vitamin-sim", "").unwrap();

        let got = read_fst(&fst_path);
        let expected: Vec<(u64, String, String)> = {
            let mut e = vec![
                (0, "clk", "0"),
                (0, "cnt[3:0]", "0000"),
                (0, "r", "r0"),
                (0, "wide[7:0]", "00000000"),
                (5, "clk", "1"),
                (5, "cnt[3:0]", "1010"),
                (5, "r", "r1.5"),
                (5, "wide[7:0]", "11111111"),
                (10, "clk", "0"),
                (10, "cnt[3:0]", "xz01"),
                (10, "wide[7:0]", "00001111"),
            ]
            .into_iter()
            .map(|(t, n, v)| (t as u64, n.to_string(), v.to_string()))
            .collect::<Vec<_>>();
            e.sort();
            e
        };
        assert_eq!(
            got, expected,
            "FST read-back must match the source VCD waveform"
        );
        let _ = std::fs::remove_file(&vcd_path);
        let _ = std::fs::remove_file(&fst_path);
    }

    /// A REAL signal dumped as an all-x vector during `$dumpoff` must become NaN
    /// in the FST (the real-domain unknown), NOT a garbage float from
    /// reinterpreting the x-bytes. Regression for the silent-wrong the adversarial
    /// review caught: vita emits `bxxx…x $` (a 64-bit x-vector) for a real under
    /// `$dumpoff`; feeding that to a real signal_change corrupts the value.
    #[test]
    fn real_dumpoff_maps_to_nan() {
        let vcd = "\
$timescale 1ns $end\n\
$scope module top $end\n\
$var real 64 ! rr $end\n\
$upscope $end\n\
$enddefinitions $end\n\
$dumpvars\nr0 !\n$end\n\
#0\nr2.5 !\n\
#5\n$dumpoff\nbxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx !\n$end\n\
#10\n$dumpon\nr2.5 !\n$end\n";
        let vp = tmp("vcd");
        let fp = tmp("fst");
        std::fs::write(&vp, vcd).unwrap();
        transcode_vcd_to_fst(&vp, &fp, "v", "").unwrap();
        let got = read_fst(&fp);
        // at t5 the real is unknown → NaN (formatted "rNaN"); t0/t10 are 2.5.
        assert!(
            got.iter().any(|(t, _, v)| *t == 5 && v == "rNaN"),
            "dumpoff real must read back as NaN, got: {got:?}"
        );
        assert!(got.iter().any(|(t, _, v)| *t == 0 && v == "r2.5"));
        assert!(got.iter().any(|(t, _, v)| *t == 10 && v == "r2.5"));
        let _ = std::fs::remove_file(&vp);
        let _ = std::fs::remove_file(&fp);
    }

    /// Timescale unit/multiplier → FST power-of-ten exponent.
    #[test]
    fn timescale_maps_to_exponent() {
        assert_eq!(timescale_exponent("1ns"), -9);
        assert_eq!(timescale_exponent("10ns"), -8);
        assert_eq!(timescale_exponent("100ps"), -10);
        assert_eq!(timescale_exponent("1fs"), -15);
        assert_eq!(timescale_exponent("1s"), 0);
    }

    /// VCD left-extension rule (short value padded per its MSB).
    #[test]
    fn vector_extends_per_msb() {
        assert_eq!(extend_vector(b"0", 4), b"0000");
        assert_eq!(extend_vector(b"1", 4), b"0001");
        assert_eq!(extend_vector(b"x", 4), b"xxxx");
        assert_eq!(extend_vector(b"z1", 4), b"zzz1");
        assert_eq!(extend_vector(b"1010", 4), b"1010");
    }
}
