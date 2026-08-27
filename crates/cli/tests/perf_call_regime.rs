//! The CALL-REGIME gate: what does writing a round behind `function` cost?
//!
//! Every perf row this repository had measured a module PROCESS body. None of
//! them measured a subroutine, and the blind spot is structural rather than an
//! oversight: `codegen_coverage` counts `ir.processes`, so a design whose work
//! lives in `ir.funcs` can report full coverage and still run none of its time on
//! a compiled path. `CodegenReport::frame_bodies` exists to say so, and run.json
//! publishes it under `codegen`.
//!
//! ## The two layers
//!
//! A user call is unrepresentable in BOTH compiled paths, and each absence costs
//! separately:
//!
//! 1. **The callee** — still open. A function body runs on
//!    `SimState::run_frame_call`, the generic `Value` tree-walk, whatever the
//!    backend.
//! 2. **The caller** — CLOSED (2026-08-27). `is_codegen_able` used to record
//!    `user_call_in_expr` and refuse the whole enclosing process body, so every
//!    statement around the call was demoted too. It no longer does; the op
//!    stream declines a call one level down to the generic evaluator, which is
//!    the one that has always run it. Measured on `bench/keccak`'s
//!    `keccak_f.sv`: **8.11 s → 6.91 s, digest identical**.
//!
//! `wprog::compile` still has no `Expr::Call` arm, so the ONE expression holding
//! the call is evaluated generically. That is a much smaller residue than layer
//! 2 was, because it no longer takes its neighbours down with it.
//!
//! ## ⚠️ Writing `function` is not the same as reaching the frame path
//!
//! The obvious pair to reach for is `perf_baseline.rs`'s `SHA256_INLINE` /
//! `SHA256_FUNCS`, and it does not work: those four transforms are straight-line,
//! `body_needs_frame` is false, the elaborator folds every call, and the two
//! designs produce a byte-identical `CodegenReport`. A "funcs" benchmark that
//! never makes a frame reads exactly like one that does. The pair below differs
//! from it by one `for` loop, which is the whole of what forces a frame.
//!
//! ## Why this lives in `cli/tests` rather than beside the other perf rows
//!
//! `sim-engine`'s test helper elaborates through `elaborate::elaborate`, which
//! drops the SIDECARS — and `func_table` is one of them. Without it
//! `run_frame_call` returns `None` and the call X-poisons, so the framed spelling
//! of this design prints `xxxxxxxx` there while the real binary prints the right
//! answer. That is the harness-sidecar gap this repository has been bitten by
//! before; running the shipped binary end-to-end cannot have it.
//!
//! ## Frozen baseline (2026-08-27, macOS arm64, release, native backend)
//!
//! Measured on `bench/keccak`, the same shape at a larger scale — `keccak_f.sv`
//! (three looping functions) against `keccak_f_flat.sv` (the identical
//! permutation with the calls expanded by hand, byte-identical digest):
//!
//! ```text
//!   keccak_f.sv        8.11 s   4055 us/perm   run_frame_call 53.7%   able 1/4  frame_bodies 3
//!   keccak_f_flat.sv   0.58 s    290 us/perm   run_frame_call  0.0%   able 2/4  frame_bodies 0
//! ```
//!
//! 14.0x, same digest, one difference. The timed row here is the in-repo version;
//! print it with
//! `cargo test --release -p cli --test perf_call_regime -- --ignored --nocapture`.
//! The `--release` matters: `CARGO_BIN_EXE_vita` is whichever profile the test
//! was built under, and a debug `vita` measures the debug build's overhead.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Transforms behind `function`, each body carrying a loop ⇒ FRAMED.
const FRAME_CALL: &str = r#"module top;
  reg clk = 0;
  reg [31:0] a,b,c,d,e,f,g,h,w,k;
  reg [31:0] t1,t2;
  integer i;
  function [31:0] mix1(input [31:0] x);
    integer j;
    begin
      mix1 = x;
      for (j = 0; j < 4; j = j + 1)
        mix1 = {mix1[6:0], mix1[31:7]} ^ (mix1 + 32'h9e3779b9);
    end
  endfunction
  function [31:0] mix0(input [31:0] x);
    integer j;
    begin
      mix0 = x;
      for (j = 0; j < 4; j = j + 1)
        mix0 = {mix0[1:0], mix0[31:2]} ^ (mix0 + 32'h7f4a7c15);
    end
  endfunction
  always @(posedge clk) begin
    t1 = h + mix1(e) + k + w;
    t2 = mix0(a);
    h <= g; g <= f; f <= e; e <= d + t1;
    d <= c; c <= b; b <= a; a <= t1 + t2;
    w <= {w[6:0],w[31:7]} ^ w ^ k;
    k <= k + 32'h9e3779b9;
  end
  initial begin
    a=32'h6a09e667; b=32'hbb67ae85; c=32'h3c6ef372; d=32'ha54ff53a;
    e=32'h510e527f; f=32'h9b05688c; g=32'h1f83d9ab; h=32'h5be0cd19;
    w=32'h428a2f98; k=32'h71374491;
    for (i=0;i<CYCLES;i=i+1) begin clk=~clk; #1; end
    $display("%h", a^b^c^d^e^f^g^h);
    $finish;
  end
endmodule
"#;

/// The SAME computation with both loops written where they are used ⇒ NO frame.
const FRAME_FLAT: &str = r#"module top;
  reg clk = 0;
  reg [31:0] a,b,c,d,e,f,g,h,w,k;
  reg [31:0] t1,t2,m1,m0;
  integer i, j;
  always @(posedge clk) begin
    m1 = e;
    for (j = 0; j < 4; j = j + 1)
      m1 = {m1[6:0], m1[31:7]} ^ (m1 + 32'h9e3779b9);
    m0 = a;
    for (j = 0; j < 4; j = j + 1)
      m0 = {m0[1:0], m0[31:2]} ^ (m0 + 32'h7f4a7c15);
    t1 = h + m1 + k + w;
    t2 = m0;
    h <= g; g <= f; f <= e; e <= d + t1;
    d <= c; c <= b; b <= a; a <= t1 + t2;
    w <= {w[6:0],w[31:7]} ^ w ^ k;
    k <= k + 32'h9e3779b9;
  end
  initial begin
    a=32'h6a09e667; b=32'hbb67ae85; c=32'h3c6ef372; d=32'ha54ff53a;
    e=32'h510e527f; f=32'h9b05688c; g=32'h1f83d9ab; h=32'h5be0cd19;
    w=32'h428a2f98; k=32'h71374491;
    for (i=0;i<CYCLES;i=i+1) begin clk=~clk; #1; end
    $display("%h", a^b^c^d^e^f^g^h);
    $finish;
  end
endmodule
"#;

/// A fresh directory per case — a parallel suite run must not collide.
fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_cr_{tag}_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// What one run of a spelling tells us. `sim_s` is run.json's own simulation
/// time, which excludes compile and elaborate — the framed spelling elaborates
/// two extra function bodies, and charging that to the ratio would flatter it.
struct Run {
    digest: String,
    sim_s: f64,
    able: u64,
    total: u64,
    frame_bodies: u64,
    reject: Vec<String>,
}

/// Build `src` at `cycles` and run the shipped binary with `--obs-dir`.
fn run(tag: &str, src: &str, cycles: u32) -> Run {
    let dir = scratch(tag);
    let sv = dir.join("d.sv");
    std::fs::write(&sv, src.replace("CYCLES", &cycles.to_string())).expect("write design");
    let obs = dir.join("obs");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args([
            sv.to_str().expect("path"),
            "--obs-dir",
            obs.to_str().expect("path"),
        ])
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{tag} did not exit 0:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let digest = stdout
        .lines()
        .find(|l| l.len() == 8 && l.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or_else(|| panic!("{tag} printed no digest line:\n{stdout}"))
        .to_string();

    let json = std::fs::read_to_string(obs.join("run.json")).expect("run.json");
    let r = Run {
        digest,
        sim_s: num(&json, "\"sim_s\":"),
        able: num(&json, "\"able\":") as u64,
        total: num(&json, "\"total\":") as u64,
        frame_bodies: num(&json, "\"frame_bodies\":") as u64,
        reject: reject_keys(&json),
    };
    let _ = std::fs::remove_dir_all(&dir);
    r
}

/// Pull one numeric field out of run.json. A hand parser rather than a JSON
/// dependency: `cli` has none today, and the two fields read here are emitted by
/// this repository's own writer, so their spelling is pinned by the tests that
/// own it rather than guessed at.
fn num(json: &str, key: &str) -> f64 {
    let at = json
        .find(key)
        .unwrap_or_else(|| panic!("run.json has no {key}"))
        + key.len();
    let rest = json[at..].trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| {
        panic!("run.json {key} is not a number: {:?}", &rest[..end.min(40)]);
    })
}

/// The `reject_reasons` object's keys, in file order.
fn reject_keys(json: &str) -> Vec<String> {
    let Some(at) = json.find("\"reject_reasons\":") else {
        return Vec::new();
    };
    let rest = &json[at + "\"reject_reasons\":".len()..];
    let Some(open) = rest.find('{') else {
        return Vec::new();
    };
    let Some(close) = rest[open..].find('}') else {
        return Vec::new();
    };
    rest[open + 1..open + close]
        .split(',')
        .filter_map(|kv| kv.split(':').next())
        .map(|k| k.trim().trim_matches('"').to_string())
        .filter(|k| !k.is_empty())
        .collect()
}

/// ⚠️ **The validity gate for the timed row, and it is not `#[ignore]`d.**
///
/// The two spellings are only a ratio if they compute the same thing. A pair that
/// has drifted apart still produces two timings and still divides them, so the
/// failure mode is a number that looks exactly like a measurement — the same
/// shape as a golden digest that survives a mutation of its own design.
///
/// It asserts the printed value, not merely that both runs exited 0: either
/// spelling could lose a term and still `$finish`. It also asserts the value is
/// not all-X, because that is what the framed spelling degrades to when the
/// `func_table` sidecar is missing, and an all-X pair would otherwise "match".
#[test]
fn the_two_spellings_compute_the_same_digest() {
    let called = run("eq_call", FRAME_CALL, 200);
    let flat = run("eq_flat", FRAME_FLAT, 200);
    assert_ne!(
        called.digest, "xxxxxxxx",
        "the framed spelling X-poisoned — the func_table sidecar did not reach the engine"
    );
    assert_eq!(
        called.digest, flat.digest,
        "the call-regime pair has drifted apart"
    );
}

/// Layer 2 is CLOSED: a body holding a user call is compiled like any other.
///
/// It was not, until this measurement. `is_codegen_able` inserted
/// `user_call_in_expr` and the whole body — every statement in it, call-bearing
/// or not — fell to `native::body::run_body`, the uncompiled walk. The reason
/// recorded for that exclusion described a two-backend world: it said the frame
/// evaluator "runs ONLY on the `&self` interpreter read path" and that "the
/// interpreter is then the SOLE executor of any Call", both of which stopped
/// being true when S3a wired `NativeKernel::eval_call`.
///
/// What the op stream actually does with a call is decline it, one level down,
/// to the evaluator that has always run it: `k_eval_for_lvalue` → `wprog_for`
/// (no `Call` arm) → `ctx().eval_ctx`. So this asserts the two spellings are now
/// indistinguishable to the gate EXCEPT for the frame bodies themselves.
#[test]
fn a_user_call_no_longer_refuses_the_body_that_holds_it() {
    let called = run("cg_call", FRAME_CALL, 20);
    let flat = run("cg_flat", FRAME_FLAT, 20);

    assert!(
        !called.reject.iter().any(|r| r == "user_call_in_expr"),
        "a user call must not refuse its body any more: {:?}",
        called.reject
    );
    assert_eq!(
        called.reject, flat.reject,
        "the pair must now be refused for exactly the same reasons"
    );
    assert_eq!(called.able, flat.able, "and admit the same count");
    assert_eq!(called.total, flat.total);
}

/// ⚠️ Layer 1 is still OPEN, and this is the assertion that says so.
///
/// `frame_bodies: 2` means `mix0` and `mix1` are still executed by
/// `SimState::run_frame_call` — the generic `Value` tree-walk — while everything
/// around them is compiled. That is the remaining half of the call regime, and
/// the number to watch when it is closed.
#[test]
fn the_callee_bodies_are_still_framed() {
    let called = run("fb_call", FRAME_CALL, 20);
    let flat = run("fb_flat", FRAME_FLAT, 20);
    assert_eq!(flat.frame_bodies, 0, "the flat spelling has no subroutine");
    assert_eq!(
        called.frame_bodies, 2,
        "mix0 and mix1 each still need a frame"
    );
}

#[test]
#[ignore = "perf probe (DATA, not a gate); run with --ignored --nocapture"]
fn perf_call_regime() {
    const CYCLES: u32 = 100_000;
    println!("\n[call regime] the same round, two spellings, native backend:\n");
    let flat = run("perf_flat", FRAME_FLAT, CYCLES);
    let called = run("perf_call", FRAME_CALL, CYCLES);
    for (name, r) in [("flat", &flat), ("called", &called)] {
        println!(
            "  {name:<8} sim {:>8.3} s   able {:>2}/{:<2}   frame_bodies {:<2}   reject {:?}",
            r.sim_s, r.able, r.total, r.frame_bodies, r.reject
        );
    }
    assert_eq!(called.digest, flat.digest, "pair drifted");
    println!(
        "\n  call regime cost = {:.2}x  (called / flat)\n",
        called.sim_s / flat.sim_s
    );
}
