//! [P6] Deterministic constrained-random corpus generator (shared test module).
//!
//! Emits N *synthesizable* SystemVerilog designs reproducibly: the same `seed`
//! yields byte-for-byte identical sources on every OS. Used as the design driver
//! for the P5 compiled-vs-interpreter differential gate (`backend_equiv.rs`) and as
//! a regression corpus.
//!
//! WHY TEMPLATES, NOT FREE-FORM RTL: arbitrary random Verilog is almost never
//! type/width-correct and would mostly fail to parse/elaborate, starving the gate.
//! Instead a fixed set of parameterized templates is filled from *valid* pools
//! (widths, depths, ops, stimulus), so every emitted design is guaranteed to build
//! and run while still varying across the corner cases the prereq enumerates:
//! out-of-range array index (E-RUN-RANGE), X/Z dynamic index, signed-over-64-bit
//! and unsigned-over-128-bit arithmetic (poison thresholds), NBA index-sampling
//! (`a[i] <= x; i = i+1`), same-net-multiple-writes-per-delta (VCD glitch), and
//! delayed continuous-assign value changes.
//!
//! DETERMINISM: a hand-rolled SplitMix64-style LCG (no `rand`/`getrandom` crate —
//! per the MSRV-1.82 / 3-OS reproducibility pin). No floats, no HashMap iteration.
//!
//! `#![allow(dead_code)]`: each test binary that does `mod common;` uses a subset
//! of the surface, so some helpers are unused per-binary.
#![allow(dead_code)]

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, Backend, SimOpts, SimResult};

/// Process-unique counter for temp VCD filenames. cargo runs test functions in
/// PARALLEL threads; two tests that generate the same-named design (e.g. the same
/// corpus seed) would otherwise write/read the SAME temp path and race — corrupting
/// the byte comparison. A monotonic suffix makes every run's path unique.
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_vcd_path(tag: &str, backend: Backend) -> std::path::PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_dir().join(format!("vitamin_{tag}_{backend:?}_{n}.vcd"))
}

/// Collects elaborate-time diagnostic strings so the build helper can assert no
/// hard (Error/Fatal) diagnostics were emitted.
#[derive(Default)]
struct DiagSink(RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

/// lex → parse → elaborate a generated design to a frozen `SimIr`. Panics with the
/// design source on any lex/parse/elaborate hard error — a generator that emits
/// invalid RTL fails loudly here, which is the point of the P6 validation test.
pub fn build(src: &str) -> sim_ir::SimIr {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}\n--- src ---\n{src}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}\n--- src ---\n{src}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(
        hard.is_empty(),
        "elaborate errors: {hard:?}\n--- src ---\n{src}"
    );
    ir.expect("elaborate returned None")
}

/// Cargo-guaranteed-writable scratch dir for integration tests. `std::env::temp_dir`
/// can be sandboxed/non-writable under the test harness (its `$dumpfile` `File::create`
/// would then silently fail and emit no VCD), so use `CARGO_TARGET_TMPDIR` instead.
fn tmp_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

/// Run `ir` on `backend`, capturing stdout. The VCD is redirected to a per-`tag`
/// temp file so the repo CWD is never littered (P6/P5 compare bytes via the
/// in-memory path, not these files). Returns `(SimResult, stdout)`.
pub fn run_on(ir: &sim_ir::SimIr, backend: Backend, tag: &str) -> (SimResult, String) {
    let tmp = unique_vcd_path(tag, backend);
    let opts = SimOpts {
        backend,
        vcd_path_override: Some(tmp.to_string_lossy().into_owned()),
        ..SimOpts::default()
    };
    simulate_capture(ir, opts)
}

/// [P5] Run `ir` on `backend`, capturing stdout AND the emitted VCD bytes. The VCD
/// is written to a per-(tag,backend) temp file (so two backends never clash), read
/// back, then removed. Returns `(SimResult, stdout, Some(vcd_bytes))`, or `None` VCD
/// for a design with no `$dumpvars`. Because both backends run with IDENTICAL opts
/// (same `$date`/`$version`/timescale and a path that never appears in the file
/// body), the two VCDs are byte-identical iff the simulation behavior matches — no
/// normalization needed (unlike the iverilog oracle, whose preamble always differs).
pub fn run_capture(
    ir: &sim_ir::SimIr,
    backend: Backend,
    tag: &str,
) -> (SimResult, String, Option<Vec<u8>>) {
    let path = unique_vcd_path(tag, backend);
    let _ = std::fs::remove_file(&path); // clear stale so a no-dump design ⇒ None
    let opts = SimOpts {
        backend,
        vcd_path_override: Some(path.to_string_lossy().into_owned()),
        ..SimOpts::default()
    };
    let (res, out) = simulate_capture(ir, opts);
    let vcd = std::fs::read(&path).ok();
    let _ = std::fs::remove_file(&path);
    (res, out, vcd)
}

/// A generated design: a stable `name` (for diagnostics) and its SV `src`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Design {
    pub name: String,
    pub src: String,
}

/// Deterministic SplitMix64 PRNG. Reproducible on every platform (pure integer
/// ops; no platform-dependent seeding). NOT cryptographic — a test stimulus source.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng {
            // golden-ratio odd constant; any nonzero seed mixes well.
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        // SplitMix64 (Steele/Lea/Vigna). Fixed constants → byte-identical stream.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[lo, hi]` (inclusive). `lo <= hi` required.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(lo <= hi);
        let span = hi - lo + 1;
        lo + self.next_u64() % span
    }

    /// Pick an element of a non-empty slice.
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() as u64 - 1) as usize]
    }

    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The template set. Each fn fills a parameterized design from `rng`, producing
/// valid synthesizable RTL. Index in this array is stable (used to name designs).
type Template = fn(&mut Rng, usize) -> Design;
const TEMPLATES: &[Template] = &[
    gen_counter,
    gen_alu,
    gen_shift_register,
    gen_memory_oob,
    gen_nba_sampling,
    gen_wide_arith,
    gen_xz_index,
    gen_multi_write_glitch,
    gen_cont_assign_mixed,
];

/// Generate `n` designs from `seed`. Cycles templates round-robin with rng-filled
/// params, so the corpus always spans every corner regardless of `n >= TEMPLATES.len()`.
pub fn corpus(seed: u64, n: usize) -> Vec<Design> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|i| {
            let t = TEMPLATES[i % TEMPLATES.len()];
            t(&mut rng, i)
        })
        .collect()
}

// ── templates ────────────────────────────────────────────────────────────────

/// Width-parameterized up-counter with sync reset, dumped to VCD.
fn gen_counter(rng: &mut Rng, idx: usize) -> Design {
    let w = rng.range(2, 16);
    let cycles = rng.range(3, 12);
    let src = format!(
        "module top;\n\
           reg clk, rst;\n\
           reg [{hi}:0] cnt;\n\
           integer k;\n\
           always @(posedge clk) if (rst) cnt <= 0; else cnt <= cnt + 1'b1;\n\
           initial begin\n\
             $dumpfile(\"c.vcd\"); $dumpvars(0, top);\n\
             clk = 0; rst = 1; cnt = 0;\n\
             #1 rst = 0;\n\
             for (k = 0; k < {cycles}; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
             $display(\"%0d\", cnt); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        cycles = cycles,
    );
    Design {
        name: format!("counter_{idx}_w{w}_c{cycles}"),
        src,
    }
}

/// Two random operands through a random op; result $display'd %h and %0d.
fn gen_alu(rng: &mut Rng, idx: usize) -> Design {
    let w = rng.range(4, 32);
    let ops = ["+", "-", "&", "|", "^", "<<", ">>", "*"];
    let op = rng.pick(&ops);
    let a = rng.range(0, (1u64 << w.min(32)) - 1);
    let b = rng.range(0, (1u64 << w.min(32)) - 1).min(w - 1); // keep shift amounts sane
    let src = format!(
        "module top;\n\
           reg [{hi}:0] a, b, y;\n\
           initial begin\n\
             $dumpfile(\"a.vcd\"); $dumpvars(0, top);\n\
             a = {a}; b = {b}; #1 y = a {op} b;\n\
             #1 $display(\"%h %0d\", y, y); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        a = a,
        b = b,
        op = op,
    );
    Design {
        name: format!("alu_{idx}_w{w}_op{}", op_tag(op)),
        src,
    }
}

/// Shift register depth D fed a random bit pattern via a clock.
fn gen_shift_register(rng: &mut Rng, idx: usize) -> Design {
    let depth = rng.range(2, 8);
    let bits = rng.range(0, (1u64 << depth) - 1);
    let src = format!(
        "module top;\n\
           reg clk, din;\n\
           reg [{hi}:0] sr;\n\
           integer k;\n\
           always @(posedge clk) sr <= {{sr[{hi2}:0], din}};\n\
           initial begin\n\
             $dumpfile(\"s.vcd\"); $dumpvars(0, top);\n\
             clk = 0; sr = 0;\n\
             for (k = 0; k < {depth}; k = k + 1) begin\n\
               din = ({bits} >> k) & 1; #1 clk = 1; #1 clk = 0;\n\
             end\n\
             $display(\"%h\", sr); $finish;\n\
           end\n\
         endmodule",
        hi = depth - 1,
        hi2 = depth.saturating_sub(2),
        depth = depth,
        bits = bits,
    );
    Design {
        name: format!("shiftreg_{idx}_d{depth}"),
        src,
    }
}

/// Memory with a deliberate out-of-range index (E-RUN-RANGE: read X / write drop).
fn gen_memory_oob(rng: &mut Rng, idx: usize) -> Design {
    let w = rng.range(4, 16);
    let depth = rng.range(2, 6);
    let oob = depth + rng.range(1, 4); // strictly out of range
    let valid = rng.range(0, depth - 1);
    let v1 = rng.range(0, (1u64 << w.min(16)) - 1);
    let v2 = rng.range(0, (1u64 << w.min(16)) - 1);
    let src = format!(
        "module top;\n\
           reg [{hi}:0] mem [0:{dhi}];\n\
           reg [{hi}:0] r;\n\
           integer i;\n\
           initial begin\n\
             for (i = 0; i <= {dhi}; i = i + 1) mem[i] = 0;\n\
             mem[{valid}] = {v1};\n\
             mem[{oob}] = {v2};      // OOB write: dropped\n\
             #1 r = mem[{oob}];      // OOB read: X\n\
             $display(\"%0d\", mem[{valid}]);\n\
             #1 r = mem[{valid}]; $display(\"%0d\", r); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        dhi = depth - 1,
        valid = valid,
        oob = oob,
        v1 = v1,
        v2 = v2,
    );
    Design {
        name: format!("mem_oob_{idx}_w{w}_d{depth}"),
        src,
    }
}

/// NBA index-sampling: `a[i] <= x; i = i+1;` must use the OLD `i` (sample moment).
fn gen_nba_sampling(rng: &mut Rng, idx: usize) -> Design {
    let depth = rng.range(3, 6);
    let v = rng.range(1, 200);
    let src = format!(
        "module top;\n\
           reg [7:0] a [0:{dhi}];\n\
           integer i, j;\n\
           initial begin\n\
             for (j = 0; j <= {dhi}; j = j + 1) a[j] = 0;\n\
             i = 0;\n\
             a[i] <= {v}; i = i + 1;   // NBA samples i=0; blocking bumps i\n\
             #1 $display(\"%0d %0d\", a[0], a[1]); $finish;\n\
           end\n\
         endmodule",
        dhi = depth - 1,
        v = v,
    );
    Design {
        name: format!("nba_sample_{idx}_d{depth}"),
        src,
    }
}

/// Wide arithmetic across the poison thresholds (>64-signed / >128-unsigned).
fn gen_wide_arith(rng: &mut Rng, idx: usize) -> Design {
    let w = *rng.pick(&[96u64, 128, 160, 200]);
    let sh = rng.range(40, 70);
    let src = format!(
        "module top;\n\
           reg [{hi}:0] a, b, c;\n\
           initial begin\n\
             a = 1; a = a << {sh};\n\
             b = 1; b = b << {sh};\n\
             #1 c = a + b;\n\
             #1 $display(\"%h\", c); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        sh = sh,
    );
    Design {
        name: format!("wide_{idx}_w{w}_sh{sh}"),
        src,
    }
}

/// X/Z dynamic index: read → X, write → no-op (unified OOR semantics).
fn gen_xz_index(rng: &mut Rng, idx: usize) -> Design {
    let depth = rng.range(2, 6);
    let src = format!(
        "module top;\n\
           reg [7:0] mem [0:{dhi}];\n\
           reg [7:0] r;\n\
           reg [7:0] xi;\n\
           integer i;\n\
           initial begin\n\
             for (i = 0; i <= {dhi}; i = i + 1) mem[i] = i + 1;\n\
             xi = 8'hxx;\n\
             mem[xi] = 8'h99;   // X index write: no-op\n\
             #1 r = mem[xi];    // X index read: X\n\
             $display(\"%0d\", mem[0]); $finish;\n\
           end\n\
         endmodule",
        dhi = depth - 1,
    );
    Design {
        name: format!("xz_index_{idx}_d{depth}"),
        src,
    }
}

/// Same net written multiple times within one delta (VCD glitch faithfulness).
fn gen_multi_write_glitch(rng: &mut Rng, idx: usize) -> Design {
    let w = rng.range(4, 12);
    let a = rng.range(0, (1u64 << w.min(12)) - 1);
    let b = rng.range(0, (1u64 << w.min(12)) - 1);
    let c = rng.range(0, (1u64 << w.min(12)) - 1);
    let src = format!(
        "module top;\n\
           reg [{hi}:0] y;\n\
           initial begin\n\
             $dumpfile(\"g.vcd\"); $dumpvars(0, top);\n\
             y = {a}; y = {b}; y = {c};   // three writes, same delta\n\
             #1 $display(\"%0d\", y); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        a = a,
        b = b,
        c = c,
    );
    Design {
        name: format!("glitch_{idx}_w{w}"),
        src,
    }
}

/// MIXED-backend design (P8 moments 1/5 + P9b): a continuous assign and a delayed
/// continuous assign (both stay interpreted — cont-assigns are not process bodies),
/// a codegen-able `always @(posedge clk)` (the suspend-free P9 class), and an
/// `initial` with `#1` (not codegen-able). One run therefore exercises compiled +
/// interpreted + cont-assign on SHARED nets.
fn gen_cont_assign_mixed(rng: &mut Rng, idx: usize) -> Design {
    let w = rng.range(4, 12);
    let a = rng.range(0, (1u64 << w.min(12)) - 1);
    let b = rng.range(0, (1u64 << w.min(12)) - 1);
    let src = format!(
        "module top;\n\
           reg clk;\n\
           reg [{hi}:0] a, b;\n\
           wire [{hi}:0] sum;\n\
           wire [{hi}:0] dly;\n\
           reg [{hi}:0] q;\n\
           integer k;\n\
           assign sum = a + b;        // cont-assign: interpreted (moment 1)\n\
           assign #2 dly = a;         // delayed cont-assign: interpreted (moment 5)\n\
           always @(posedge clk) q <= sum;  // codegen-able (always_ff)\n\
           initial begin\n\
             $dumpfile(\"m.vcd\"); $dumpvars(0, top);\n\
             clk = 0; a = {a}; b = {b};\n\
             for (k = 0; k < 3; k = k + 1) begin #1 clk = 1; #1 clk = 0; end\n\
             #3 $display(\"%0d %0d %0d\", sum, q, dly); $finish;\n\
           end\n\
         endmodule",
        hi = w - 1,
        a = a,
        b = b,
    );
    Design {
        name: format!("cont_mixed_{idx}_w{w}"),
        src,
    }
}

fn op_tag(op: &str) -> &'static str {
    match op {
        "+" => "add",
        "-" => "sub",
        "&" => "and",
        "|" => "or",
        "^" => "xor",
        "<<" => "shl",
        ">>" => "shr",
        "*" => "mul",
        _ => "op",
    }
}
