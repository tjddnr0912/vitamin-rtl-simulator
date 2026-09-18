//! §4.5.513 (`WPROG-WHY`): run.json's `wprog` object — the per-EXPRESSION
//! tier-3 compile-decline tally. SPEC = docs/preview/19-ai-agent-observability.md
//! §5.2.1.
//!
//! Teeth. `codegen` is a per-BODY census and can read `able 1/1` while every
//! evaluation inside that body runs the generic walk; `wprog` is the object that
//! says where the compiled lane ends. Its audience is a machine, so a wrong
//! tally is a silent-wrong of its own kind (ENGINEERING_RULES §2.7) and the
//! numbers here are HAND-DERIVED from one census design rather than recorded
//! from a run: every declined right-hand side below was isolated in its own
//! design and confirmed to produce exactly the key asserted for it.
//!
//! The other half is that a reporting table must not be a semantic: `wprog`
//! names declines, so the stdout of the census design is pinned against the
//! iverilog/verilator line in the same file.
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with extra `args`; returns (stdout, exit_code, obs_dir).
fn run(src: &str, args: &[&str]) -> (String, i32, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_obs_wprog_{}_{n}", std::process::id()));
    // START CLEAN — `n` restarts at 0 in every test process and the OS recycles
    // PIDs, so two runs can land on the same directory (see `obs.rs`'s copy).
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join("obsout");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(&d);
    let out = cmd.output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
        obs,
    )
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// Grab the value token of a top-level `"key": <value>` line from run.json.
fn field<'a>(json: &'a str, key: &str) -> &'a str {
    for line in json.lines() {
        let t = line.trim().trim_end_matches(',');
        if let Some(rest) = t.strip_prefix(&format!("\"{key}\": ")) {
            return rest;
        }
    }
    ""
}

/// The `"k": <u64>` member of the `wprog` object's own text.
fn num(wprog: &str, k: &str) -> u64 {
    wprog
        .split(&format!("\"{k}\": "))
        .nth(1)
        .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|d| d.parse().ok())
        .unwrap_or_else(|| panic!("no {k} in {wprog}"))
}

/// The inside of the `wprog` object's `reasons` map, as `"k": n, "k": n`.
fn reasons_of(wprog: &str) -> &str {
    wprog
        .split("\"reasons\": {")
        .nth(1)
        .and_then(|r| r.split('}').next())
        .expect("reasons object")
}

/// 0-based index of the line carrying `"key": `, or `usize::MAX`.
fn line_of(json: &str, key: &str) -> usize {
    let pat = format!("\"{key}\": ");
    json.lines()
        .position(|l| l.trim_start().starts_with(&pat))
        .unwrap_or(usize::MAX)
}

/// The census design. Eleven right-hand sides under one clock, chosen so that
/// each decline REASON the vocabulary can produce from ordinary RTL appears at
/// most once, beside four shapes the compiled lane accepts.
///
/// Its output is pinned in `a_decline_does_not_change_a_value` below.
const CENSUS_SV: &str = r#"module top;
  function automatic int add1(input int x); return x + 1; endfunction
  reg clk = 0;
  always #1 clk = ~clk;
  logic [7:0] a = 8'd3, b = 8'd5;
  logic [3:0] i = 4'd2;
  logic [15:0] m = 0;
  logic [3:0]  ps = 0;
  int c = 0;
  int cl = 0;
  logic [79:0] wide = 80'd1, wsum = 0;
  logic signed [7:0] s = -8'sd16;
  logic signed [7:0] sa = 0;
  logic [7:0] sh = 0, nb = 0, tr = 0, cc = 0;
  logic [7:0] mem [0:3] = '{8'd1, 8'd2, 8'd3, 8'd4};
  logic [7:0] el = 0;
  always @(posedge clk) begin
    m  <= a * b;
    ps <= a[i +: 4];
    c  <= add1(c);
    cl <= $clog2(a);
    wsum <= wide + 80'd2;
    sa <= s >>> 1;
    sh <= a << i;
    nb <= ~i;
    tr <= a + b;
    cc <= {a[3:0], b[3:0]};
    el <= mem[i];
  end
  initial begin
    #10 $display("m=%0d ps=%0d c=%0d cl=%0d wsum=%0d sa=%0d sh=%0d nb=%0d tr=%0d cc=%0d el=%0d", m, ps, c, cl, wsum, sa, sh, nb, tr, cc, el);
    $finish;
  end
endmodule
"#;

/// The `unit` sentence, verbatim. Written out here rather than matched loosely
/// because it is the only thing in the file that tells a consumer what the two
/// numbers COUNT, and a rail that misdescribes itself is a silent-wrong.
const UNIT: &str = "distinct expression ids compile was asked about; a declined id is filed \
                    under its FIRST decline reason and counted once however many contexts or \
                    sites ask, so declined equals the sum of reasons and never exceeds asked; \
                    null when the native kernel did not run, because wprog is not the lane there";

/// The census design's whole `wprog` value, hand-derived. Each count is
/// attributed in the test body.
fn census_wprog() -> String {
    format!(
        "{{\"asked\": 33, \"declined\": 10, \"reasons\": {{\"call\": 1, \"operator\": 3, \
         \"select_offset\": 1, \"shift_amount\": 1, \"sysfunc\": 1, \"width\": 3}}, \
         \"unit\": \"{UNIT}\"}}"
    )
}

/// ⭐ Every decline in the census design names a reason, and every count is
/// hand-checkable.
///
/// The ten declines, each confirmed by isolating that shape in a design of its
/// own before this literal was written:
///
/// | expression | key | why |
/// |---|---|---|
/// | `a * b` | `operator` | `Mul` has no compile arm |
/// | `s >>> 1` | `operator` | a SIGNED `>>>` is the one shift whose bits read the sign |
/// | `-8'sd16` (the `s` declaration's initializer) | `operator` | `UnOp::Minus` has no arm |
/// | `wide + 80'd2` | `width` | the root context is 80 bits |
/// | `wide = 80'd1` (declaration initializer) | `width` | 80-bit root |
/// | `wsum = 0` (declaration initializer) | `width` | 80-bit root |
/// | `a[i +: 4]` | `select_offset` | a runtime part-select offset |
/// | `add1(c)` | `call` | a user function inside an expression |
/// | `$clog2(a)` | `sysfunc` | not the `$signed`/`$unsigned` seal |
/// | `a << i` | `shift_amount` | the amount is not a 2-state constant |
///
/// Admitted, and therefore absent from the map: `~i`, `a + b`,
/// `{a[3:0], b[3:0]}`, `mem[i]` (a runtime array index IS admitted), and the
/// `~clk` of `always #1 clk = ~clk`. The `$display` arguments contribute nothing
/// — task arguments go through `eval_task_arg`, which never asks this compiler.
#[test]
fn every_decline_reason_is_named_and_the_counts_are_hand_checkable() {
    let (_, code, obs) = run(CENSUS_SV, &[]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    assert_eq!(field(&m, "backend"), "\"native\"", "{m}");
    assert_eq!(field(&m, "wprog"), census_wprog(), "{m}");
    // The two invariants the object claims about itself, read back off the file
    // rather than off the struct: `declined` is the SUM of `reasons`, and never
    // exceeds `asked`.
    let v = field(&m, "wprog");
    let (asked, declined) = (num(v, "asked"), num(v, "declined"));
    let sum: u64 = reasons_of(v)
        .split(", ")
        .map(|kv| kv.rsplit(": ").next().unwrap().parse::<u64>().unwrap())
        .sum();
    assert_eq!(declined, sum, "declined must equal the sum of reasons: {v}");
    assert!(declined <= asked, "declined must not exceed asked: {v}");
}

/// `null`, not an empty object, when the tier-3 kernel is not the lane — and the
/// per-BODY census beside it does not move, because that one is static.
#[test]
fn the_object_is_null_when_the_native_kernel_did_not_run() {
    let (_, c1, o1) = run(CENSUS_SV, &["--backend", "native"]);
    let (_, c2, o2) = run(CENSUS_SV, &["--backend", "vm"]);
    let (_, c3, o3) = run(CENSUS_SV, &["--backend", "interp"]);
    assert_eq!((c1, c2, c3), (0, 0, 0));
    let (m1, m2, m3) = (
        read(&o1.join("run.json")),
        read(&o2.join("run.json")),
        read(&o3.join("run.json")),
    );
    assert_eq!(field(&m1, "wprog"), census_wprog(), "{m1}");
    assert_eq!(field(&m2, "wprog"), "null", "{m2}");
    assert_eq!(field(&m3, "wprog"), "null", "{m3}");
    // …while `codegen` is byte-identical across all three: it is a property of
    // the DESIGN, and this pair is what keeps the two objects from being read as
    // one census (soundness-review F1's rule, applied to the new object).
    assert_eq!(field(&m1, "codegen"), field(&m2, "codegen"), "{m1}\n{m2}");
    assert_eq!(field(&m1, "codegen"), field(&m3, "codegen"), "{m1}\n{m3}");
}

/// Deterministic, and not moved by an instrumentation flag. `--obs-procs` turns
/// on a per-body profile; the compile tally must not notice.
#[test]
fn the_tally_is_byte_identical_across_runs_and_does_not_move_with_obs_procs() {
    let (_, c1, o1) = run(CENSUS_SV, &[]);
    let (_, c2, o2) = run(CENSUS_SV, &[]);
    let (_, c3, o3) = run(CENSUS_SV, &["--obs-procs"]);
    assert_eq!((c1, c2, c3), (0, 0, 0));
    let (m1, m2, m3) = (
        read(&o1.join("run.json")),
        read(&o2.join("run.json")),
        read(&o3.join("run.json")),
    );
    assert_eq!(field(&m1, "wprog"), field(&m2, "wprog"), "two runs differ");
    assert_eq!(
        field(&m1, "wprog"),
        field(&m3, "wprog"),
        "--obs-procs moved"
    );
    assert_eq!(field(&m1, "wprog"), census_wprog());
}

/// The vocabulary is CLOSED: a consumer may pin these keys, so the engine's own
/// list is pinned here as a literal and the census map is checked against it.
#[test]
fn the_vocabulary_is_closed() {
    // Byte-lexicographic order, the same list doc-19 §5.2.1 documents.
    const EXPECTED: &[&str] = &[
        "array_whole",
        "call",
        "class_handle",
        "concat_width",
        "const_domain",
        "frame_net",
        "index_range",
        "index_unknown",
        "lazy_index",
        "malformed",
        "net_kind",
        "net_width",
        "node_kind",
        "operator",
        "replicate_count",
        "select_offset",
        "select_range",
        "shift_amount",
        "sign",
        "sysfunc",
        "truncation",
        "width",
    ];
    let all = sim_engine::wprog_decline_reasons();
    assert_eq!(all, EXPECTED, "the closed vocabulary moved");
    let mut sorted = EXPECTED.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted.as_slice(), EXPECTED, "not byte-lexicographic");
    let (_, code, obs) = run(CENSUS_SV, &[]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    let reasons = reasons_of(field(&m, "wprog"));
    for kv in reasons.split(", ") {
        let k = kv.split(": ").next().unwrap().trim_matches('"');
        assert!(all.contains(&k), "key {k} is outside the vocabulary");
    }
}

/// Position, because the two objects are meant to be read together: `wprog` is
/// the EXPRESSION-level counterpart of `codegen` and sits immediately after it,
/// immediately before `native`.
#[test]
fn the_object_sits_between_codegen_and_native() {
    let (_, code, obs) = run(CENSUS_SV, &[]);
    assert_eq!(code, 0);
    let m = read(&obs.join("run.json"));
    let (cg, wp, nat) = (
        line_of(&m, "codegen"),
        line_of(&m, "wprog"),
        line_of(&m, "native"),
    );
    assert_eq!(wp, cg + 1, "wprog is not directly after codegen\n{m}");
    assert_eq!(wp, nat - 1, "wprog is not directly before native\n{m}");
}

/// ⚠️ The reporting table is a reporting table. Naming a decline may not change
/// a value, so the census design's output is pinned against the line iverilog
/// and verilator both produce for it.
#[test]
fn a_decline_does_not_change_a_value() {
    const ORACLE: &str = "m=15 ps=0 c=5 cl=2 wsum=3 sa=-8 sh=12 nb=253 tr=8 cc=53 el=3";
    let (out, code, _) = run(CENSUS_SV, &[]);
    assert_eq!(code, 0);
    assert_eq!(out.lines().next().unwrap_or(""), ORACLE, "{out}");
    // …and the three executors agree on it, which is what says the tally did not
    // move the lane either.
    for bk in ["native", "vm", "interp"] {
        let (o, c, _) = run(CENSUS_SV, &["--backend", bk]);
        assert_eq!(c, 0, "{bk}");
        assert_eq!(o.lines().next().unwrap_or(""), ORACLE, "{bk}: {o}");
    }
}

/// Round-1 review (both lenses): a `Call` or `SysFunc` whose self width differs
/// from the context reaches the width branch's `CtxClass::Unknown` arm before
/// the node match, and that arm filed `node_kind` — so a 16-bit LHS fed by an
/// 8-bit function read as "no user-function declines". The reason is the
/// NODE's from both decline sites now; the value never moved (all four print
/// the oracle line).
#[test]
fn a_call_or_sysfunc_files_its_own_key_whatever_the_context_width() {
    const SV: &str = r#"module top;
  function automatic logic [7:0] f8(input logic [7:0] x); return x + 8'd1; endfunction
  reg clk = 0; always #1 clk = ~clk;
  logic [7:0] a = 8'd3;
  logic [7:0]  y8  = 0;
  logic [15:0] y16 = 0;
  logic [15:0] c16 = 0;
  logic [40:0] c41 = 0;
  always @(posedge clk) begin
    y8  <= f8(a);
    y16 <= f8(a);
    c16 <= $clog2(a);
    c41 <= $clog2(a);
  end
  initial begin #6 $display("y8=%0d y16=%0d c16=%0d c41=%0d", y8, y16, c16, c41); $finish; end
endmodule
"#;
    let (out, code, obs) = run(SV, &[]);
    assert_eq!(code, 0);
    assert_eq!(
        out.lines().next().unwrap_or(""),
        "y8=4 y16=4 c16=2 c41=2",
        "{out}"
    );
    let wp = field(&read(&obs.join("run.json")), "wprog").to_string();
    assert_eq!(
        reasons_of(&wp),
        "\"call\": 2, \"sysfunc\": 2",
        "a width-mismatched call or sysfunc must not read as node_kind: {wp}"
    );
}

/// Round-1 review (differential): only an index the IR holds as a plain
/// constant node reaches the compile-time `index_range` arm. A sized literal
/// takes the runtime-index lane: it COMPILES and reports E4002 at run time, so
/// it is not a decline. The SPEC row says so; this pins that the two spellings
/// keep the same value and diagnostic while only one of them tallies.
#[test]
fn a_sized_literal_index_compiles_while_an_unsized_one_declines_with_index_range() {
    const HEAD: &str = r#"module top;
  reg clk = 0; always #1 clk = ~clk;
  logic [7:0] mem [0:3] = '{8'd1, 8'd2, 8'd3, 8'd4};
  logic [7:0] y = 0;
"#;
    const TAIL: &str = r#"
  initial begin #4 $display("y=%b", y); $finish; end
endmodule
"#;
    let plain = format!("{HEAD}  always @(posedge clk) y <= mem[7];{TAIL}");
    let sized = format!("{HEAD}  always @(posedge clk) y <= mem[3'd7];{TAIL}");
    let (o1, c1, obs1) = run(&plain, &[]);
    let (o2, c2, obs2) = run(&sized, &[]);
    assert_eq!((c1, c2), (1, 1), "E4002 is an error on both spellings");
    assert_eq!(
        o1, o2,
        "the value and the diagnostics must not depend on the spelling"
    );
    assert!(o1.contains("y=xxxxxxxx"), "{o1}");
    let w1 = field(&read(&obs1.join("run.json")), "wprog").to_string();
    let w2 = field(&read(&obs2.join("run.json")), "wprog").to_string();
    assert_eq!(reasons_of(&w1), "\"index_range\": 1", "{w1}");
    assert_eq!(reasons_of(&w2), "", "a sized literal index compiles: {w2}");
}
