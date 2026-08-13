//! `--backend <interp|vm>` — the CLI surface for the bytecode VM.
//!
//! The VM has existed and been measured since Stage C, but nothing outside the library
//! could select it: `SimOpts::backend` defaulted to `Interpreter` and no flag reached it,
//! so a user had no way to run on it. This file pins the flag AND the one property that
//! makes exposing it defensible — selecting `vm` must not move a single output byte.
//!
//! That equivalence is already locked at the library level over the whole corpus by
//! `sim-engine/tests/backend_equiv.rs` (the P5 gate). What is pinned HERE is the part the
//! P5 gate cannot see: that the flag actually reaches `SimOpts` on both the one-shot and
//! the staged path — a flag that parses but is dropped on the floor would leave the P5
//! gate green and the feature dead, which is exactly the state this file was written to
//! end.
//!
//! Coverage note (measured 2026-07-31, `perf_baseline.rs::perf_p9_coverage`): the VM
//! claims 50–67% of process templates on real designs, including all four `examples/`.
//! The uncovered remainder is the `#delay`-bearing testbench half, which is correct — it
//! is the DUT bodies, the ones that run millions of times, that the VM takes.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A design with a codegen-able `always` body (no delay/fork/disable in it) plus a
/// delay-driven `initial` that is NOT codegen-able — so a single run exercises both
/// executors at once, which is the normal case and the one most likely to diverge.
const MIXED: &str = "module t;\n\
  reg clk = 0;\n\
  reg [31:0] a, b, c;\n\
  integer i;\n\
  always @(posedge clk) begin\n\
    a <= (a ^ {b[7:0], b[31:8]}) + 32'h9e3779b9;\n\
    b <= (b & c) ^ (~b & a);\n\
    c <= c + a;\n\
  end\n\
  initial begin\n\
    $dumpfile(\"w.vcd\"); $dumpvars(0, t);\n\
    a = 32'h1; b = 32'h2; c = 32'h3;\n\
    for (i = 0; i < 40; i = i + 1) begin clk = ~clk; #1; end\n\
    $display(\"a=%h b=%h c=%h\", a, b, c);\n\
    $finish;\n\
  end\n\
endmodule\n";

/// A fresh directory per test — a parallel suite run must not collide on `t.sv`/`w.vcd`.
fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_bk_{tag}_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run `vita` (optionally as a staged applet) in `dir`, returning stdout+stderr and ok.
fn vita_in(dir: &std::path::Path, args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// The whole point of the flag: it changes wall-clock, never a byte. Compared on BOTH
/// output channels, because the backends differ in process-body control flow and a
/// divergence could surface in the waveform without touching `$display` at all.
///
/// BOTH backends are named EXPLICITLY. This test used to compare "no flag" against
/// `--backend vm`, which was a real comparison only while the interpreter was the
/// default — the moment the default became `vm` it would have been comparing the VM
/// against itself and passing for free. A differential must never take one of its two
/// sides from a default.
#[test]
fn selecting_the_vm_moves_no_output_byte() {
    let dir = scratch("oneshot");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (oi, ok_i) = vita_in(&dir, &["--backend", "interp", "-o", "i.vcd", "t.sv"]);
    assert!(
        ok_i && !oi.contains("error[VITA"),
        "interp run failed:\n{oi}"
    );
    let (ov, ok_v) = vita_in(&dir, &["--backend", "vm", "-o", "v.vcd", "t.sv"]);
    assert!(ok_v && !ov.contains("error[VITA"), "vm run failed:\n{ov}");

    assert_eq!(oi, ov, "stdout differs between backends");
    let vi = std::fs::read(dir.join("i.vcd")).expect("interp VCD");
    let vv = std::fs::read(dir.join("v.vcd")).expect("vm VCD");
    assert_eq!(vi, vv, "VCD bytes differ between backends");
    // Guard against a vacuous pass: the design must actually have simulated.
    assert!(oi.contains("a="), "design produced no transcript:\n{oi}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// `--backend vm` is the default spelled out, so it must be byte-identical to passing
/// nothing. (A flag whose "default" value behaves differently from its absence is a
/// trap: CI scripts pin one and humans use the other.)
///
/// This is also the CLI-visible pin on WHICH backend is the default: if the default
/// reverted to `interp`, the `vm`-vs-absent comparison would still pass — but the
/// `interp`-vs-absent inequality below could not be asserted, so the two halves
/// together are what fix the default. The exact value lives in
/// `sim-engine/tests/backend_equiv.rs::the_default_backend_is_the_vm`.
#[test]
fn naming_the_default_explicitly_changes_nothing() {
    let dir = scratch("explicit");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (a, _) = vita_in(&dir, &["-o", "a.vcd", "t.sv"]);
    let (b, _) = vita_in(&dir, &["--backend", "vm", "-o", "b.vcd", "t.sv"]);
    assert_eq!(a, b, "the default must behave exactly like `--backend vm`");
    assert_eq!(
        std::fs::read(dir.join("a.vcd")).unwrap(),
        std::fs::read(dir.join("b.vcd")).unwrap()
    );

    // And naming the OTHER backend must also change nothing observable — that is the
    // equivalence the flag promises, in the direction the default does not cover.
    let (c, _) = vita_in(&dir, &["--backend", "interp", "-o", "c.vcd", "t.sv"]);
    assert_eq!(a, c, "`--backend interp` moved an output byte");
    assert_eq!(
        std::fs::read(dir.join("a.vcd")).unwrap(),
        std::fs::read(dir.join("c.vcd")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The staged path is a SEPARATE `VitaOpts` construction from the one-shot path, so it
/// gets its own pin — threading the field into one and forgetting the other is the
/// natural way for this to be half-wired.
#[test]
fn the_staged_run_honours_the_flag_and_still_matches() {
    let dir = scratch("staged");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (o, ok) = vita_in(&dir, &["vcmp", "-o", "t.vu", "t.sv"]);
    assert!(ok, "vcmp failed:\n{o}");
    let (o, ok) = vita_in(&dir, &["velab", "-o", "t.velab", "t.vu"]);
    assert!(ok, "velab failed:\n{o}");

    // Both backends named explicitly — see `selecting_the_vm_moves_no_output_byte`
    // on why a differential must not take one side from a default.
    let (oi, ok_i) = vita_in(
        &dir,
        &["vrun", "--backend", "interp", "-o", "i.vcd", "t.velab"],
    );
    assert!(
        ok_i && !oi.contains("error[VITA"),
        "vrun interp failed:\n{oi}"
    );
    let (ov, ok_v) = vita_in(&dir, &["vrun", "--backend", "vm", "-o", "v.vcd", "t.velab"]);
    assert!(ok_v && !ov.contains("error[VITA"), "vrun vm failed:\n{ov}");

    // ③층: `native` falls back on the staged path too, and must move nothing.
    // The staged flow has NO obs surface (`vrun --obs-dir` is rejected), so this
    // byte comparison is the ONLY thing standing between a fall-back and a
    // silent behaviour change there — which is why it is pinned here.
    let (on, ok_n) = vita_in(
        &dir,
        &["vrun", "--backend", "native", "-o", "n.vcd", "t.velab"],
    );
    assert!(
        ok_n && !on.contains("error[VITA"),
        "vrun native failed:\n{on}"
    );

    assert_eq!(oi, ov, "staged stdout differs between backends");
    assert_eq!(on, ov, "staged stdout moved when native was requested");
    assert_eq!(
        std::fs::read(dir.join("i.vcd")).unwrap(),
        std::fs::read(dir.join("v.vcd")).unwrap(),
        "staged VCD bytes differ between backends"
    );
    assert_eq!(
        std::fs::read(dir.join("n.vcd")).unwrap(),
        std::fs::read(dir.join("v.vcd")).unwrap(),
        "staged VCD bytes moved when native was requested"
    );
    assert!(
        oi.contains("a="),
        "staged design produced no transcript:\n{oi}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A compile applet must REJECT it rather than accept-and-ignore. `vcmp --backend vm`
/// that exits 0 would read as "I compiled that for the VM", and nothing in the artifact
/// depends on the backend — the misreading is the whole reason this is loud.
#[test]
fn a_compile_applet_rejects_a_simulate_side_flag() {
    let dir = scratch("reject");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    for stage in ["vcmp", "velab"] {
        let (o, ok) = vita_in(&dir, &[stage, "--backend", "vm", "-o", "x.out", "t.sv"]);
        assert!(!ok, "{stage} accepted a simulate-side flag:\n{o}");
        assert!(
            o.contains("simulate-side argument") && o.contains(stage),
            "{stage} rejection does not name the reason or the stage:\n{o}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unknown value is loud and names both spellings — a silent fallback to the
/// interpreter would make a typo look like the VM had run.
#[test]
fn an_unknown_backend_name_is_loud_and_says_what_is_accepted() {
    let dir = scratch("badval");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (o, ok) = vita_in(&dir, &["--backend", "jit", "t.sv"]);
    assert!(!ok, "an unknown backend name was accepted:\n{o}");
    assert!(o.contains("'interp'") && o.contains("'vm'"), "got:\n{o}");

    // A bare `--backend` at end-of-argv must not swallow a following source or run
    // silently on the default.
    let (o, ok) = vita_in(&dir, &["t.sv", "--backend"]);
    assert!(!ok, "a valueless --backend was accepted:\n{o}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// §4.5.272: a filelist rewrites PATHS relative to the list, so every value-taking flag
/// must be registered in `filelist::takes_value` or its value is mangled into a path.
/// The failure mode there was exit 0 with a silently wrong file, so this is pinned on the
/// value that would be rewritten (`vm`), not on the flag.
#[test]
fn a_filelist_does_not_rewrite_the_backend_value_as_a_path() {
    let dir = scratch("flist");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/list.f"), "--backend vm\n../t.sv\n").unwrap();

    let (o, ok) = vita_in(&dir, &["-F", "sub/list.f", "--dump-filelist"]);
    assert!(ok, "filelist expansion failed:\n{o}");
    assert!(
        !o.contains("sub/vm") && !o.contains("source vm"),
        "the flag VALUE was rewritten as a source path:\n{o}"
    );
    assert!(o.contains("t.sv"), "the real source went missing:\n{o}");

    // And it must still actually run from inside the list.
    let (o, ok) = vita_in(&dir, &["-F", "sub/list.f", "-o", "f.vcd"]);
    assert!(ok && o.contains("a="), "filelist run failed:\n{o}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The help text must state the property a user needs to decide: same output, only
/// wall-clock. Advertising a speedup without that is how a correctness-critical tool
/// gets used in a mode nobody verified.
#[test]
fn the_help_states_that_output_is_identical() {
    let dir = scratch("help");
    let (o, ok) = vita_in(&dir, &["--help"]);
    assert!(ok, "--help failed:\n{o}");
    assert!(o.contains("--backend"), "--backend is undocumented:\n{o}");
    assert!(
        o.contains("byte-identical"),
        "help does not state that output is unchanged:\n{o}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Continuous assigns are evaluated through a pre-compiled native program when one
/// exists. An RHS that reaches a user FUNCTION call must NOT take that path.
///
/// The body path excludes calls at the BODY level (`is_codegen_able`'s B1 rule), not
/// inside `try_compile` — the frame evaluator runs only on the `&self` interpreter read
/// path, with a re-entrant frame arena and the left-to-right operand order static
/// recursion depends on. Reusing `try_compile` for continuous assigns without
/// reproducing that caller-side precondition broke 18 tests across package-scoped calls,
/// enum-returning functions and a cont-assign-originated runaway.
///
/// Pinned at the CLI level on purpose: the library test harness elaborates WITHOUT
/// sidecars, so it has no `func_table` and every function call reads X there — a
/// library-level version of this test would pass vacuously on a broken build.
///
/// Both backends, because the settle is backend-INDEPENDENT: the guard has to hold on
/// the default interpreter too. Values verified against iverilog 13.
#[test]
fn a_cont_assign_calling_a_function_still_evaluates() {
    let dir = scratch("ca_fn");
    let src = "module t;\n\
      function automatic [31:0] twice(input [31:0] x);\n\
        twice = x + x;\n\
      endfunction\n\
      reg  [31:0] a;\n\
      wire [31:0] y = twice(a) + 32'd1;\n\
      wire [31:0] z = a ^ 32'hff;\n\
      initial begin\n\
        a = 32'd21;\n\
        #1 $display(\"y=%0d z=%0h\", y, z);\n\
        a = 32'd100;\n\
        #1 $display(\"y=%0d z=%0h\", y, z);\n\
        $finish;\n\
      end\n\
    endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [vec!["t.sv"], vec!["--backend", "vm", "t.sv"]] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        // iverilog 13: y=43 z=ea then y=201 z=9b.
        assert!(
            o.contains("y=43 z=ea") && o.contains("y=201 z=9b"),
            "{args:?}: a function-calling continuous assign must keep evaluating on the \
             interpreter and re-evaluate when its input moves — got:\n{o}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// NATIVE-TYPE GUARD — the shapes whose value is not a plain 4-state integer.
///
/// A native program's register is a bare `(val, unk)` word pair, so it structurally
/// cannot carry `is_real` / `is_str` / a heap handle. Until the guard landed,
/// `try_compile` had no type test at all — only a width test — and every one of these
/// silently lost the flag the write path dispatches on. Measured at `7937c94`, all under
/// `--backend vm`, all silent (exit 0, no diagnostic):
///
/// | source                    | interpreter | VM before the guard      |
/// |---------------------------|-------------|--------------------------|
/// | `q = r;` (real→real)      | `2.750000`  | `4613374868287651840.0`  |
/// | `i = r;` (real→int)       | `3`         | `0`                      |
/// | `s2 = s;` (string→string) | `hi`        | `` (empty)               |
/// | `s2 = arr[1];`            | `bb`        | `` (empty)               |
///
/// The string case is the guard's clearest signature: `"a"` is 0x61 and the destination
/// `string`'s net width is 0, so `lvalue_width().max(1)` handed the compiler a ONE-BIT
/// context and the whole string became the single bit 0x61 & 1 = 1.
///
/// A class field read is the same hole wearing different clothes: it is
/// `Signal { word: Some(field_id) }`, and the indexed funnel would read the FIELD id as
/// an ARRAY word.
///
/// This lives at CLI level ON PURPOSE. `class_is_handle`, `dyn_is_handle` and the
/// `real r[]` / `string s[]` element flags are installed as out-of-band sidecars from
/// `SimOpts`, which the library test harness does not build — a library-level pin would
/// run with every sidecar empty and pass without testing the thing it names.
const NONINTEGRAL: &str = "module t;\n\
  real r = 2.75, q;\n\
  string s = \"hi\", s2;\n\
  string arr [2] = '{\"aa\",\"bb\"};\n\
  byte  dq [];\n\
  int   i;\n\
  initial begin\n\
    q = 2.75;    $display(\"L1 %0f\", q);\n\
    q = r;       $display(\"L2 %0f\", q);\n\
    q = r + 1.0; $display(\"L3 %0f\", q);\n\
    q = r * 2;   $display(\"L4 %0f\", q);\n\
    i = r;       $display(\"L5 %0d\", i);\n\
    s2 = s;      $display(\"L6 [%s]\", s2);\n\
    s2 = arr[1]; $display(\"L7 [%s]\", s2);\n\
    s2 = \"lit\";  $display(\"L8 [%s]\", s2);\n\
    dq = new[3]; dq[0] = 8'h5a;\n\
    i = dq[0];   $display(\"L9 %0h\", i);\n\
    i = s[0];    $display(\"L10 %0h\", i);\n\
  end\n\
endmodule\n";

#[test]
fn a_non_integral_value_never_rides_the_native_path() {
    let dir = scratch("nonint");
    std::fs::write(dir.join("t.sv"), NONINTEGRAL).unwrap();

    let (oi, ok_i) = vita_in(&dir, &["t.sv"]);
    assert!(
        ok_i && !oi.contains("error[VITA"),
        "interp run failed:\n{oi}"
    );
    let (ov, ok_v) = vita_in(&dir, &["--backend", "vm", "t.sv"]);
    assert!(ok_v && !ov.contains("error[VITA"), "vm run failed:\n{ov}");
    assert_eq!(oi, ov, "backends differ on non-integral values");

    // Absolute values too, not just agreement — both backends agreeing on the SAME
    // wrong answer is exactly what a pure differential cannot see.
    for want in [
        "L1 2.750000",
        "L2 2.750000",
        "L3 3.750000",
        "L4 5.500000",
        "L5 3",
        "L6 [hi]",
        "L7 [bb]",
        "L8 [lit]",
        "L9 5a",
        "L10 68",
    ] {
        assert!(oi.contains(want), "missing `{want}` in:\n{oi}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same hole, reached through a CONTINUOUS ASSIGN — which is not backend-specific.
///
/// Compiling cont-assign right-hand sides natively (`7937c94`) put every assign RHS
/// through `try_compile` on BOTH backends, so this shape turned a correct DEFAULT-path
/// result into a silent-wrong one. Three-way at the time: iverilog `3`, vita before the
/// change `3`, vita after `4613374868287651840` — the raw IEEE-754 bits of 2.75, because
/// the native program dropped `is_real` and `write_lvalue`'s real→int rounding
/// (IEEE 1364-2005 §6.2, round half away from zero) never ran.
///
/// Pinned on the interpreter as well as the VM: this is not a VM bug, and running it
/// only under `--backend vm` would have left the regression invisible.
#[test]
fn a_real_valued_continuous_assign_still_rounds() {
    let dir = scratch("ca_real");
    let src = "module t;\n\
      real r = 2.75;\n\
      wire [63:0] w;\n\
      assign w = r;\n\
      initial #1 $display(\"w=%0d\", w);\n\
    endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [vec!["t.sv"], vec!["--backend", "vm", "t.sv"]] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        assert!(
            o.contains("w=3"),
            "{args:?}: a real continuous-assign RHS must round to the integer net \
             (iverilog prints 3), not deliver its IEEE bit pattern — got:\n{o}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The per-body PROLOGUE — the `SimState` fields that say which process is running.
///
/// `vm_run_body` used to carry a hand-copied excerpt of `run_process`'s prologue that
/// set `cur_time_mult` and dropped the other two. Both omissions were silently
/// observable under `--backend vm`, at exit 0 with no diagnostic:
///
/// - `%m` in a submodule rendered whatever scope another process had left behind
///   (`in tb` instead of `in tb.u1`);
/// - `$time` in a module with its own `timescale` PRECISION rendered at whatever
///   precision ran previously.
///
/// Both now go through one `exec::enter_body`, called by both executors. This is a CLI
/// test because the second half needs per-module `timescale`, which means the
/// preprocessor, which means the real pipeline.
#[test]
fn the_per_body_prologue_reaches_both_executors() {
    let dir = scratch("prologue");
    let src = "`timescale 1ns/1ps\n\
      module sub;\n\
        initial begin $display(\"in %m\"); #1 $display(\"sub t=%0t\", $time); end\n\
      endmodule\n\
      `timescale 1ns/1ns\n\
      module tb;\n\
        sub u1();\n\
        initial begin\n\
          $display(\"at %m\");\n\
          #2 $display(\"tb t=%0t\", $time);\n\
          $finish;\n\
        end\n\
      endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [
        vec!["--top", "tb", "t.sv"],
        vec!["--backend", "vm", "--top", "tb", "t.sv"],
    ] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        // iverilog 13 prints exactly these four lines.
        for want in ["at tb", "in tb.u1", "sub t=1000", "tb t=2000"] {
            assert!(
                o.contains(want),
                "{args:?}: missing `{want}` — the per-body prologue did not reach this \
                 executor:\n{o}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `c = new` — an allocation site whose meaning lives in a StmtId-keyed sidecar.
///
/// In the IR this is an ordinary `BlockingAssign` with a placeholder const rhs;
/// `compute_effect` checks `class_new_sites` FIRST and never evaluates that placeholder.
/// The VM's classifier was IR-only and could not see the sidecar, so it compiled the
/// placeholder: the handle stayed X, every later field write was dropped with a
/// null-dereference warning, and the run still exited 0 printing `a=X b=0X`.
///
/// Note the shape: a class with an EXPLICIT constructor was fine, because `new(7)` is a
/// call and the B1 rule already kept it off the VM. Only the implicit-default `new`
/// reached the hole.
#[test]
fn a_bare_class_new_allocates_on_both_backends() {
    let dir = scratch("class_new");
    let src = "class C; int a; logic [7:0] b; endclass\n\
      module t; C c;\n\
      initial begin c = new; c.a = 5; c.b = 8'hAB;\n\
        $display(\"a=%0d b=%h\", c.a, c.b);\n\
      end endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [vec!["t.sv"], vec!["--backend", "vm", "t.sv"]] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok, "{args:?} failed:\n{o}");
        assert!(
            o.contains("a=5 b=ab"),
            "{args:?}: bare `new` did not allocate:\n{o}"
        );
        assert!(
            !o.contains("W4020"),
            "{args:?}: null/X handle dereference — the allocation was skipped:\n{o}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A seeded `$dist_*` writes its seed variable back, so the draw MUST advance.
///
/// The VM's intercept list was a hand-written copy of `sim_ir::sysfunc_is_stmt_effect`
/// that named `DistUniform` and none of its six siblings. `$dist_normal(seed, …)` was
/// therefore compiled, the seed was never written back, and every draw repeated the
/// first — silently, at exit 0. `$dist_uniform` in the same program was correct, which
/// is what made it look like an RNG quirk rather than a classifier hole.
#[test]
fn a_seeded_dist_draw_advances_its_seed_on_both_backends() {
    let dir = scratch("dist_seed");
    let src = "module t;\n\
      integer seed = 5; integer i, v;\n\
      initial for (i = 0; i < 4; i = i + 1) begin\n\
        v = $dist_normal(seed, 50, 10); $display(\"N %0d\", v);\n\
      end\n\
    endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    let mut seen: Vec<String> = Vec::new();
    for args in [vec!["t.sv"], vec!["--backend", "vm", "t.sv"]] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        let draws: Vec<&str> = o.lines().filter(|l| l.starts_with("N ")).collect();
        assert_eq!(draws.len(), 4, "{args:?}: expected 4 draws:\n{o}");
        // Teeth: a frozen seed makes every draw identical. That is the bug's signature.
        assert!(
            draws
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 1,
            "{args:?}: every draw identical — the seed was never written back:\n{o}"
        );
        seen.push(draws.join("|"));
    }
    assert_eq!(seen[0], seen[1], "backends produced different seed streams");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The NBA queue's destination is stored INLINE for a single chunk and only heap-boxed
/// for a concat LHS. This exercises the rare arm.
///
/// A nonblocking assignment's destination has to outlive the activation that scheduled
/// it, so it used to be an owned `Lvalue` — a heap `Vec` allocated at every `<=` and
/// freed at every NBA flush. Measured on picorv32 + testbench (40000 cycles), 99.5% of
/// 2,474,446 updates are a SINGLE chunk, so the chunk now travels by value and only
/// `{a,b} <= x` still allocates.
///
/// Pinned here because nothing else guarantees the multi-chunk arm is ever taken: the
/// P5 corpus generator emits no concat LHS, so the split could have shipped with the
/// rare half never executed. Values are absolute (iverilog 13 agrees), not merely equal
/// across backends — two backends sharing one wrong split is what a pure differential
/// cannot see.
#[test]
fn a_concat_lhs_nonblocking_assign_splits_correctly() {
    let dir = scratch("nba_concat");
    let src = "module t;\n\
      reg clk = 0;\n\
      reg [7:0] a, b;\n\
      reg [3:0] c, d, e;\n\
      reg [15:0] src = 16'hA5C3;\n\
      reg [11:0] s2 = 12'h123;\n\
      reg [7:0] mem [0:3];\n\
      integer i;\n\
      always @(posedge clk) begin\n\
        {a, b}    <= src;\n\
        {c, d, e} <= s2;\n\
        mem[1]    <= 8'h77;\n\
      end\n\
      initial begin\n\
        for (i = 0; i < 3; i = i + 1) begin clk = ~clk; #1; end\n\
        $display(\"a=%h b=%h c=%h d=%h e=%h mem1=%h\", a, b, c, d, e, mem[1]);\n\
        $finish;\n\
      end\n\
    endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [
        vec!["--backend", "interp", "t.sv"],
        vec!["--backend", "vm", "t.sv"],
    ] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        assert!(
            o.contains("a=a5 b=c3 c=1 d=2 e=3 mem1=77"),
            "{args:?}: concat-LHS nonblocking split is wrong (iverilog 13 prints \
             `a=a5 b=c3 c=1 d=2 e=3 mem1=77`):\n{o}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The two conditions the scalar-write specialisation must NOT bake in.
///
/// `Op::WriteScalar` is chosen once per template, from the lvalue's SHAPE and the net's
/// STORAGE — both fixed for the run. Two things it depends on are NOT fixed, so they are
/// tested live inside the op, and this pins both:
///
/// - `force` re-targets a net for the rest of the run, and a forced net must IGNORE
///   every ordinary driver (IEEE 1364-2005 §9.3.2). Baking that in would let a
///   specialised write overwrite a forced value.
/// - a real-valued source into an integer net must ROUND (§6.2, half away from zero),
///   which only the general funnel does; the specialised store would deliver the raw
///   IEEE-754 bits.
///
/// SHAPE OF THE TEST IS LOAD-BEARING. The first version put the `force` and the writes
/// in ONE `initial` block — and `is_codegen_able` excludes any body containing
/// `Stmt::Force`, so that whole block ran on the interpreter and `Op::WriteScalar` never
/// executed. It passed with the live checks deleted. The force must therefore come from
/// a DIFFERENT process than the codegen-able body doing the writes, which is what this
/// version does: the `always` block is blocking-assign-only (so it compiles, and its
/// destination `a` is a plain scalar, so it specialises) while the `initial` drives the
/// clock and applies the force.
///
/// iverilog 13 prints exactly the three lines asserted below.
#[test]
fn the_scalar_write_specialisation_still_honours_force_and_real() {
    let dir = scratch("spec_live");
    let src = "module t;\n\
      reg clk = 0;\n\
      reg [31:0] a;\n\
      reg [63:0] w;\n\
      real r = 2.75;\n\
      always @(posedge clk) begin\n\
        a = a + 32'd1;\n\
        w = r;\n\
      end\n\
      initial begin\n\
        a = 32'd0;\n\
        #1 clk = 1; #1 clk = 0;\n\
        $display(\"t1 a=%0d w=%0d\", a, w);\n\
        force a = 32'd99;\n\
        #1 clk = 1; #1 clk = 0;\n\
        $display(\"forced a=%0d\", a);\n\
        release a;\n\
        #1 clk = 1; #1 clk = 0;\n\
        $display(\"released a=%0d\", a);\n\
        $finish;\n\
      end\n\
    endmodule\n";
    std::fs::write(dir.join("t.sv"), src).unwrap();

    for args in [
        vec!["--backend", "interp", "t.sv"],
        vec!["--backend", "vm", "t.sv"],
    ] {
        let (o, ok) = vita_in(&dir, &args);
        assert!(ok && !o.contains("error[VITA"), "{args:?} failed:\n{o}");
        // `w=3` is the real→int rounding; `forced a=99` is the always block's blocking
        // write being ignored; `released a=100` is it taking effect again from 99.
        for want in ["t1 a=1 w=3", "forced a=99", "released a=100"] {
            assert!(o.contains(want), "{args:?}: missing `{want}`:\n{o}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ── ③층 (doc-21 S1d): `--backend native` selects a backend that does not execute
// yet. The contract while that is true: it NEVER changes an output byte, and the
// observability rail says which executor actually ran — a fallback only the
// wall-clock could reveal would be the wrong-log doc-19 §3 forbids.

/// Extract the value token of a top-level `"key": <value>` line from run.json.
fn manifest_field<'a>(json: &'a str, key: &str) -> &'a str {
    for line in json.lines() {
        let t = line.trim().trim_end_matches(',');
        if let Some(rest) = t.strip_prefix(&format!("\"{key}\": ")) {
            return rest;
        }
    }
    ""
}

#[test]
fn requesting_native_falls_back_without_moving_an_output_byte() {
    let dir = scratch("native_fb");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (vm_out, ok1) = vita_in(&dir, &["--backend", "vm", "-o", "vm.vcd", "t.sv"]);
    let (nat_out, ok2) = vita_in(&dir, &["--backend", "native", "-o", "nat.vcd", "t.sv"]);
    assert!(ok1 && ok2, "runs failed:\n{vm_out}\n---\n{nat_out}");
    assert_eq!(vm_out, nat_out, "requesting native moved stdout");
    let vm_vcd = std::fs::read(dir.join("vm.vcd")).unwrap();
    let nat_vcd = std::fs::read(dir.join("nat.vcd")).unwrap();
    assert_eq!(vm_vcd, nat_vcd, "requesting native moved VCD bytes");

    // …and run.json names the executor that RAN, not the one requested.
    let (o, ok) = vita_in(
        &dir,
        &[
            "--backend",
            "native",
            "--obs-dir",
            "obs",
            "-o",
            "o.vcd",
            "t.sv",
        ],
    );
    assert!(ok, "obs run failed:\n{o}");
    let m = std::fs::read_to_string(dir.join("obs/run.json")).unwrap();
    // ⚠️ `MIXED` used to FALL BACK here, on the `$dumpvars` row. S1d-4d-2 wired
    // the dump tasks, so it now RUNS natively — and the property this test is
    // named for holds more strongly for it: requesting native moved neither a
    // stdout byte nor a VCD byte, on a design the native backend executed.
    assert_eq!(manifest_field(&m, "backend"), "\"native\"", "{m}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The runtime gate has TWO halves and run.json reports both, because their
/// answers differ: a design with subroutines is within v1's SCOPE (calls are
/// core — S3 compiles them) but outside what today's STORAGE can hold (frame
/// locals live in an activation window, not a net slot). Folding them into one
/// flag would let an upper bound read as a capability.
#[test]
fn the_native_verdict_reports_scope_and_storage_separately() {
    let dir = scratch("native_verdict");

    // NOTHING refuses `MIXED` any more, and the value here has now moved twice:
    // `null` → the system-task row when S1d-4c-2c gave `refused` a third source
    // → back to `null` when S1d-4d-2 wired `$dumpfile`/`$dumpvars`. The field is
    // the answer of all three layers; this design passes all three.
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();
    let (o, ok) = vita_in(&dir, &["--obs-dir", "obs1", "-o", "a.vcd", "t.sv"]);
    assert!(ok, "{o}");
    let m = std::fs::read_to_string(dir.join("obs1/run.json")).unwrap();
    assert_eq!(
        manifest_field(&m, "native"),
        "{\"eligible\": true, \"buildable\": true, \"refused\": null, \"reject_reasons\": {}}",
        "{m}"
    );

    // …and `null` must still be REACHABLE, or the update above would have
    // quietly retired the case it changed. A design nothing refuses: no dump,
    // no continuous assign, no in-body waiter.
    std::fs::write(
        dir.join("clean.sv"),
        "module t;\n\
           reg [7:0] n;\n\
           initial begin n = 8'd0; #1 n = 8'd1; $display(\"n=%0d\", n); $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let (o, ok) = vita_in(&dir, &["--obs-dir", "obsc", "clean.sv"]);
    assert!(ok, "{o}");
    let m = std::fs::read_to_string(dir.join("obsc/run.json")).unwrap();
    assert_eq!(
        manifest_field(&m, "native"),
        "{\"eligible\": true, \"buildable\": true, \"refused\": null, \"reject_reasons\": {}}",
        "{m}"
    );

    // Subroutine design, STORE-INDEPENDENT: S3a admits it, so both halves are
    // true and the run.json verdict says so. This assertion used to read
    // `buildable: false, refused: "frame-local storage: S3 (subroutine
    // frames)"` — the value changed because the capability did, and the arm
    // that keeps the two-half property under test moved to the design below.
    std::fs::write(
        dir.join("f.sv"),
        "module t;\n\
           function automatic integer inc(input integer x);\n\
             integer loc; begin loc = x + 1; inc = loc; end\n\
           endfunction\n\
           integer r;\n\
           initial begin r = inc(3); $display(\"r=%0d\", r); #1 $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let (o, ok) = vita_in(&dir, &["--obs-dir", "obs2", "-o", "b.vcd", "f.sv"]);
    assert!(ok && o.contains("r=4"), "{o}");
    let m = std::fs::read_to_string(dir.join("obs2/run.json")).unwrap();
    assert_eq!(
        manifest_field(&m, "native"),
        "{\"eligible\": true, \"buildable\": true, \"refused\": null, \"reject_reasons\": {}}",
        "{m}"
    );

    // …and the scope-vs-storage SPLIT must still be reachable, or the update
    // above retired the case this test is named for. A subroutine that reads a
    // MODULE net is eligible (calls are core) and not buildable: the engine's
    // frame executor would read it from the flat store, which a native run
    // never writes.
    //
    // ⚠️ A3-iii re-picked this design, for the reason the paragraph above now
    // half-states: a subroutine READING a module net is delegated with the
    // caller's store and builds. What is left is a WRITE to a module-scope
    // class field — the only out-of-window destination elaborate permits from a
    // subroutine body (a plain `g = …` is E3009 a phase earlier).
    std::fs::write(
        dir.join("g.sv"),
        "module t;\n\
           class C; int v; endclass\n\
           C c;\n\
           function automatic integer addg(input integer x);\n\
             begin c.v = x; addg = x + 5; end\n\
           endfunction\n\
           integer r;\n\
           initial begin c = new(); r = addg(3); $display(\"r=%0d\", r); #1 $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let (o, ok) = vita_in(&dir, &["--obs-dir", "obs2b", "-o", "b2.vcd", "g.sv"]);
    assert!(ok && o.contains("r=8"), "{o}");
    let m = std::fs::read_to_string(dir.join("obs2b/run.json")).unwrap();
    assert_eq!(
        manifest_field(&m, "native"),
        "{\"eligible\": true, \"buildable\": false, \
         \"refused\": \"a subroutine that WRITES a net outside its own frame: S3b\", \
         \"reject_reasons\": {}}",
        "{m}"
    );

    // Design-gate refusal: `refused` names the family, the map keeps the detail.
    //
    // ⚠️ `real`, not the `string s; int q[$]` this used to use — V1 slice 2
    // admitted every heap kind, and a refusal pin whose shape became eligible
    // asserts nothing.
    std::fs::write(
        dir.join("q.sv"),
        "module t;\n\
           real r;\n\
           initial begin r = 1.5;\n\
             $display(\"%f\", r); #1 $finish; end\n\
         endmodule\n",
    )
    .unwrap();
    let (o, ok) = vita_in(&dir, &["--obs-dir", "obs3", "-o", "c.vcd", "q.sv"]);
    assert!(ok, "{o}");
    let m = std::fs::read_to_string(dir.join("obs3/run.json")).unwrap();
    assert_eq!(
        manifest_field(&m, "native"),
        "{\"eligible\": false, \"buildable\": false, \"refused\": \"real\", \
         \"reject_reasons\": {\"real\": 1}}",
        "{m}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
