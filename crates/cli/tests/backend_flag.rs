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
#[test]
fn selecting_the_vm_moves_no_output_byte() {
    let dir = scratch("oneshot");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (oi, ok_i) = vita_in(&dir, &["-o", "i.vcd", "t.sv"]);
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

/// `--backend interp` is the default spelled out, so it must be byte-identical to
/// passing nothing. (A flag whose "default" value behaves differently from its absence
/// is a trap: CI scripts pin one and humans use the other.)
#[test]
fn naming_the_default_explicitly_changes_nothing() {
    let dir = scratch("explicit");
    std::fs::write(dir.join("t.sv"), MIXED).unwrap();

    let (a, _) = vita_in(&dir, &["-o", "a.vcd", "t.sv"]);
    let (b, _) = vita_in(&dir, &["--backend", "interp", "-o", "b.vcd", "t.sv"]);
    assert_eq!(a, b);
    assert_eq!(
        std::fs::read(dir.join("a.vcd")).unwrap(),
        std::fs::read(dir.join("b.vcd")).unwrap()
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

    let (oi, ok_i) = vita_in(&dir, &["vrun", "-o", "i.vcd", "t.velab"]);
    assert!(
        ok_i && !oi.contains("error[VITA"),
        "vrun interp failed:\n{oi}"
    );
    let (ov, ok_v) = vita_in(&dir, &["vrun", "--backend", "vm", "-o", "v.vcd", "t.velab"]);
    assert!(ok_v && !ov.contains("error[VITA"), "vrun vm failed:\n{ov}");

    assert_eq!(oi, ov, "staged stdout differs between backends");
    assert_eq!(
        std::fs::read(dir.join("i.vcd")).unwrap(),
        std::fs::read(dir.join("v.vcd")).unwrap(),
        "staged VCD bytes differ between backends"
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
