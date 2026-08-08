//! IEEE 1364-2005 §12.2 — the IMPLICIT parameter port list.
//!
//! `module m; parameter W = 8; ... endmodule` (no ANSI `#(...)` header) declares its
//! overridable parameters in the body, and every override channel must reach them:
//! `#(.W(8))`, `#(8)`, `defparam u.W = 8`, and `-G W=8`. vita reached none of them —
//! they reported `override of unknown parameter`, `more positional parameter overrides
//! than module parameters`, or (the `-G` channel) `names no parameter of any top
//! module`: a loud refusal of core Verilog-2005.
//!
//! Every expected value here is what **iverilog 13.0** prints for the same source
//! (recorded per test). Asserting a VALUE and not merely "elaborates cleanly" is the
//! point: a binding that resolves the name but keeps the declared default, or one that
//! lands a positional value on the wrong declaration, would pass an exit-code-only
//! gate. Several tests below exist only because they DISCRIMINATE — the localparam-skip
//! and the dependent-parameter re-fold cannot be told apart from a naive fix by any
//! design whose parameters are independent and whose values coincide.
//!
//! The second half pins the STAGED half of `-G`: `velab -G` parsed the flag, reported
//! `errors=0` and elaborated the declared defaults, and `vcmp`/`vrun` accepted and
//! dropped it. doc-14 RULE B puts the override on the elaborate stage — so it applies
//! in `velab` and is loud on the other two. Those tests drive **argv** through
//! `cli::run`, not the library entry points: the flag was being dropped at the
//! argv→opts boundary in `dispatch_velab`, so a test that hands `VitaOpts` straight to
//! `run_velab` cannot see the bug it is guarding (measured — that test passes with the
//! fix reverted).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, extra: &[&str]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ipp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(extra)
        .current_dir(&d)
        .output()
        .expect("run vita");
    let r = (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    );
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn run(src: &str) -> (String, String, Option<i32>) {
    run_args(src, &[])
}

/// The design's `$display` lines, trimmed — the simulator's own epilogue
/// ("simulation ended …") is on stdout too and is not part of the pinned value.
fn lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

// ── the four override channels reach a body-declared parameter ──────────────

/// iverilog: `W=11 D=12` / `W=21 D=22` / `W=5 D=6`.
/// Named and positional in one design, plus an un-overridden instance so a fix that
/// wrote the override into the MODULE (rather than the instance) is caught by `u3`.
#[test]
fn named_and_positional_override_reach_body_parameters() {
    let (o, e, c) = run("module sub;
           parameter W = 5;
           parameter D = 6;
           initial $display(\"W=%0d D=%0d\", W, D);
         endmodule
         module tb;
           sub #(.W(11), .D(12)) u1 ();
           sub #(21, 22) u2 ();
           sub u3 ();
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["W=11 D=12", "W=21 D=22", "W=5 D=6"]);
}

/// iverilog: `W=44 L=88` — `defparam` reaches a body parameter, and the localparam
/// that derives from it follows (88, not 2*5).
#[test]
fn defparam_reaches_a_body_parameter_and_derived_localparam_follows() {
    let (o, e, c) = run("module sub;
           parameter W = 5;
           localparam L = W * 2;
           initial $display(\"W=%0d L=%0d\", W, L);
         endmodule
         module tb;
           sub u1 ();
           defparam u1.W = 44;
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["W=44 L=88"]);
}

/// iverilog: `P=9 L=2`. The CLI channel — `-G` against the TOP module's body
/// parameter, which is the spelling picorv32-style testbenches use
/// (`module tb; parameter CYCLES = 40000;`).
#[test]
fn cli_g_override_reaches_a_top_body_parameter() {
    let (o, e, c) = run_args(
        "module tb;
           localparam L = 2;
           parameter P = 1;
           initial $display(\"P=%0d L=%0d\", P, L);
         endmodule",
        &["-G", "P=9"],
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["P=9 L=2"]);
}

/// `-v` echoes the EFFECTIVE values of the stage's inputs, and `-G` is the one flag
/// whose effect is a different design — so it belongs in that block beside its
/// compile-stage (`defines`) and run-stage (`plusargs`) siblings, not only inside the
/// raw `invocation:` line where a filelist or the attached `-GW=9` spelling hides it.
#[test]
fn the_effective_invocation_echo_reports_the_parameter_overrides() {
    let (o, e, c) = run_args(
        "module tb; parameter W = 1;
           initial begin $display(\"W=%0d\", W); #1 $finish; end
         endmodule",
        &["-G", "W=9", "-v"],
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    let echoed = format!("{o}{e}");
    assert!(
        echoed
            .lines()
            .any(|l| l.starts_with("params:") && l.contains("W=9")),
        "no `params:` row in the -v echo; got:\n{echoed}"
    );
}

// ── the two rules a naive fix gets wrong ────────────────────────────────────

/// iverilog: `A=10 L=2 B=30` — a `localparam` between two parameters is NOT a
/// positional slot. DISCRIMINATOR: if it were counted, `30` would land on `L` and the
/// design goes loud ("cannot override localparam"), so this cannot pass by accident.
/// The ANSI-header twin below shares the rule and shared the bug.
#[test]
fn a_localparam_does_not_consume_a_positional_slot() {
    let body = run("module sub;
           parameter A = 1;
           localparam L = 2;
           parameter B = 3;
           initial $display(\"A=%0d L=%0d B=%0d\", A, L, B);
         endmodule
         module tb; sub #(10, 30) u1 (); endmodule");
    assert_eq!(body.2, Some(0), "stderr: {}", body.1);
    assert_eq!(lines(&body.0), ["A=10 L=2 B=30"]);

    // Same rule, ANSI header (`localparam` is legal in an SV parameter port list).
    // iverilog: `A=10 L=2 B=30`. This one was loud BEFORE the implicit list existed.
    let ansi = run(
        "module sub #(parameter A = 1, localparam L = 2, parameter B = 3);
           initial $display(\"A=%0d L=%0d B=%0d\", A, L, B);
         endmodule
         module tb; sub #(10, 30) u1 (); endmodule",
    );
    assert_eq!(ansi.2, Some(0), "stderr: {}", ansi.1);
    assert_eq!(lines(&ansi.0), ["A=10 L=2 B=30"]);
}

/// iverilog: `A=5 B=6 C=60` / `A=1 B=50 C=500` / `A=5 B=50 C=500`.
/// DISCRIMINATOR: the later declarations READ the earlier ones, so an override that
/// is installed anywhere other than in the decl-order fold (say, patched in after the
/// body walk) leaves `B`/`C` computed from the declared defaults.
#[test]
fn later_body_parameters_refold_from_the_overridden_value() {
    let (o, e, c) = run("module sub;
           parameter A = 1;
           parameter B = A + 1;
           parameter C = B * 10;
           initial $display(\"A=%0d B=%0d C=%0d\", A, B, C);
         endmodule
         module tb;
           sub #(.A(5)) u1 ();
           sub #(.B(50)) u2 ();
           sub #(5, 50) u3 ();
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        lines(&o),
        ["A=5 B=6 C=60", "A=1 B=50 C=500", "A=5 B=50 C=500"]
    );
}

/// iverilog: `sub W=8` / `sub W=4` / `a=ff b=f u1.W=8 u2.W=4`.
/// Three things at once, all of which a per-module (rather than per-instance) binding
/// gets wrong: a non-ANSI PORT whose width comes from the body parameter, two sibling
/// instances with different values, and a hierarchical read of each.
#[test]
fn body_parameter_sizes_a_port_and_two_instances_keep_their_own_values() {
    let (o, e, c) = run("module sub(o);
           parameter W = 4;
           output [W-1:0] o;
           assign o = {W{1'b1}};
           initial $display(\"sub W=%0d\", W);
         endmodule
         module tb;
           wire [7:0] a; wire [3:0] b;
           sub #(8) u1 (a);
           sub      u2 (b);
           initial #1 $display(\"a=%h b=%h u1.W=%0d u2.W=%0d\", a, b, u1.W, u2.W);
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["sub W=8", "sub W=4", "a=ff b=f u1.W=8 u2.W=4"]);
}

/// iverilog: `W=7` — `defparam` beats `#()` on the same parameter (IEEE §23.10.1),
/// and that ordering must survive on the body-parameter path too.
#[test]
fn defparam_still_wins_over_a_hash_override_on_a_body_parameter() {
    let (o, e, c) = run("module sub;
           parameter W = 1;
           initial $display(\"W=%0d\", W);
         endmodule
         module tb;
           sub #(.W(5)) u1 ();
           defparam u1.W = 7;
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["W=7"]);
}

/// iverilog: three lines for `N=3`, one for `N=1` — the override reaches a generate
/// loop bound, i.e. it is in place before the body is unrolled.
#[test]
fn body_parameter_override_reaches_a_generate_bound() {
    let (o, e, c) = run("module sub;
           parameter N = 2;
           genvar i;
           generate for (i = 0; i < N; i = i + 1) begin : g
             initial $display(\"inst %0d of %0d\", i, N);
           end endgenerate
         endmodule
         module tb;
           sub #(.N(3)) u1 ();
           sub #(.N(1)) u2 ();
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        lines(&o),
        ["inst 0 of 3", "inst 1 of 3", "inst 2 of 3", "inst 0 of 1"]
    );
}

/// iverilog: `K=a5 S=xyz` / `K=ff S=def` — a sized literal, a string and a `'1` fill
/// all re-fold at the BODY parameter's declared width, exactly as on the header path.
#[test]
fn typed_body_parameters_take_sized_string_and_fill_overrides() {
    let (o, e, c) = run("module sub;
           parameter [7:0] K = 8'h00;
           parameter S = \"def\";
           initial $display(\"K=%h S=%s\", K, S);
         endmodule
         module tb;
           sub #(.K(8'hA5), .S(\"xyz\")) u1 ();
           sub #(.K('1)) u2 ();
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["K=a5 S=xyz", "K=ff S=def"]);
}

/// iverilog: `W=9` twice — an instance ARRAY of a module whose parameter is
/// body-declared.
#[test]
fn instance_array_applies_a_body_parameter_override() {
    let (o, e, c) = run("module sub(input x, output y);
           parameter W = 3;
           assign y = x;
           initial $display(\"W=%0d\", W);
         endmodule
         module tb;
           wire [1:0] a = 2'b10;
           wire [1:0] b;
           sub #(9) u[1:0] (a, b);
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["W=9", "W=9"]);
}

/// iverilog: `d=ff` — an INTERFACE's body parameter. Its own body-parameter fold is a
/// third code path that ignores overrides, so without the same routing a resolved
/// override would silently keep `W=4` and `i1.d` would print `f`.
#[test]
fn interface_body_parameter_takes_an_override() {
    let (o, e, c) = run("interface ifc;
           parameter W = 4;
           logic [W-1:0] d;
         endinterface
         module tb;
           ifc #(.W(8)) i1 ();
           initial begin i1.d = 8'hFF; #1 $display(\"d=%h\", i1.d); end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["d=ff"]);
}

/// A body parameter must keep the correct-or-loud escalation the header path has: an
/// override that was WRITTEN but does not fold must not fall back to the declared
/// default at exit 0 — that is a different design, silently. Nothing tested this on the
/// body path (measured: deleting the `unfoldable` term from `ParamOverrides::targets`
/// passed the entire 5264-test suite before this test existed).
#[test]
fn a_non_constant_override_of_a_body_parameter_is_loud() {
    let (o, e, c) = run("module sub;
           parameter W = 5;
           initial $display(\"W=%0d\", W);
         endmodule
         module tb;
           wire [3:0] sig;
           sub #(.W(sig)) u1 ();
         endmodule");
    assert_ne!(
        c,
        Some(0),
        "must not run with the declared default; got {o}"
    );
    // The distinctive TAIL, not "is not a constant": the pre-existing W3056 warning
    // ("override of parameter `W` is not a constant; default kept") carries that
    // substring too, so the shorter assertion passed against the PRE binary and could
    // not tell the escalation from the warning it replaced.
    assert!(
        e.contains("that is a different design, not a smaller one"),
        "expected the not-a-constant escalation, got: {e}"
    );
}

/// iverilog answers an `'x`/`'z` fill with x/z. vita's override channel is an i64 with
/// no unknown plane, so it must REFUSE — the packed value word of such a fill is a
/// plausible `0` (`'x`) or all-ones (`'z`) with the mask discarded, i.e. a wrong number
/// rather than a missing one, and every channel used to install it silently.
///
/// The refusal is keyed on the fill BIT, in the ordering block above every early
/// return, so it covers all three spellings. Keying it on "the fill did not re-fold"
/// instead only caught the NAMED one: the positional arm always populates `by_name`, so
/// it took a different branch and installed `K=00` / `K=ff`, and `-G` never populated
/// `fill` at all. Each row below is a channel that was silently wrong.
#[test]
fn an_x_or_z_fill_override_is_refused_on_every_channel() {
    let decl = "module sub; parameter [7:0] K = 8'h11; initial $display(\"K=%h\", K); endmodule";
    for (what, src) in [
        (
            "named",
            format!("{decl}\nmodule tb; sub #(.K('x)) u(); endmodule"),
        ),
        (
            "positional",
            format!("{decl}\nmodule tb; sub #('x) u(); endmodule"),
        ),
        (
            "positional z",
            format!("{decl}\nmodule tb; sub #('z) u(); endmodule"),
        ),
        (
            "ansi header",
            "module sub #(parameter [7:0] K = 8'h11); initial $display(\"K=%h\", K); endmodule
             module tb; sub #(.K('x)) u(); endmodule"
                .to_string(),
        ),
        (
            "real target",
            "module sub #(parameter real K = 1.0); initial $display(\"K=%0d\", K); endmodule
             module tb; sub #(.K('x)) u(); endmodule"
                .to_string(),
        ),
    ] {
        let (o, e, c) = run(&src);
        assert_ne!(c, Some(0), "{what}: kept the default silently; got {o}");
        assert!(
            e.contains("cannot be applied") && e.contains("no x/z plane"),
            "{what}: expected the fill refusal, got: {e}"
        );
    }
    // the CLI channel — `-G K='x` installed 0 and `-G K='z` all-ones
    for v in ["K='x", "K='z"] {
        let (o, e, c) = run_args(
            "module tb; parameter [7:0] K = 8'h11; initial $display(\"K=%h\", K); endmodule",
            &["-G", v],
        );
        assert_ne!(c, Some(0), "-G {v}: kept the default silently; got {o}");
        assert!(e.contains("no x/z plane"), "-G {v}: got {e}");
    }
    // …and naming a localparam reports the LOCALPARAM, which is the whole reason
    // localparams stay in the resolved list. Which reason wins is the oracle's call:
    // iverilog answers this exact source with `Cannot override localparam 'K'` and
    // says nothing about the x/z plane — a declaration that cannot be overridden at
    // all is not reached by the question of whether THIS override is representable.
    // (This assertion used to demand the x/z wording, i.e. the old behaviour where
    // the fill refusal ran first and called a localparam a "parameter".)
    let (_, e, c) = run(
        "module sub; localparam K = 2; initial $display(\"K=%0d\", K); endmodule
         module tb; sub #(.K('x)) u(); endmodule",
    );
    assert_ne!(c, Some(0));
    assert!(e.contains("cannot override localparam `K`"), "got: {e}");

    // `'0`/`'1` are representable and must still apply — iverilog `K=ff` / `K=00`.
    for (fill, want) in [("'1", "K=ff"), ("'0", "K=00")] {
        let (o, e, c) = run(&format!(
            "{decl}\nmodule tb; sub #(.K({fill})) u(); endmodule"
        ));
        assert_eq!(c, Some(0), "stderr: {e}");
        assert_eq!(lines(&o), [want]);
    }
    // including through `-G`
    let (o, e, c) = run_args(
        "module tb; parameter [7:0] K = 8'h11; initial $display(\"K=%h\", K); endmodule",
        &["-G", "K='1"],
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["K=ff"]);
}

/// A filelist path-resolves every token that is not a known flag VALUE, so a `.f`
/// carrying the SEPARATED spelling `-G W=8` turned `W=8` into an absolute path and the
/// run died with ``-G /abs/…/W=8` names no parameter of any top module``. The attached
/// `-GW=8` was unaffected — one flag, two spellings, one of them broken, which is how
/// this survived the sweep that registered every other value-taking flag.
#[test]
fn a_filelist_carries_a_separated_g_override() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ippf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let src = d.join("t.sv");
    std::fs::write(
        &src,
        "module tb; parameter W = 1;
           initial begin $display(\"W=%0d\", W); #1 $finish; end
         endmodule",
    )
    .unwrap();
    let f = d.join("build.f");
    std::fs::write(&f, format!("-G W=8\n{}\n", src.display())).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-f")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let o = String::from_utf8_lossy(&out.stdout).into_owned();
    let e = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["W=8"]);
    let _ = std::fs::remove_dir_all(&d);
}

/// A fill literal has no width of its own — it takes the target's — so every channel
/// must carry it VERBATIM to the binder, the only place the declared width is known.
/// `#(.K('1))` did. `defparam` and `-G` folded it in the parent at the 32-bit
/// self-determined default and installed `0000_0000_ffff_ffff` in a 64-bit parameter,
/// so two spellings of one override disagreed inside a single design at exit 0.
///
/// iverilog prints `K=ffffffffffffffff` for all three. The **64-bit** target is the
/// whole point: at 8 bits the 32-bit fold and the declared-width fold coincide, and a
/// test written at that width passes with this fix reverted (measured — the `-G K='1`
/// row of the test above is 8-bit and is vacuous for exactly this defect).
#[test]
fn a_fill_override_refolds_at_the_target_width_on_every_channel() {
    let decl = "module sub; parameter [63:0] K = 64'h1; initial $display(\"K=%h\", K); endmodule";
    // `#()` and `defparam` in ONE design, so a fix that repairs only one is caught.
    let (o, e, c) = run(&format!(
        "{decl}
         module tb;
           sub #(.K('1)) u1();
           sub u2();
           defparam u2.K = '1;
           initial #1 $finish;
         endmodule"
    ));
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["K=ffffffffffffffff", "K=ffffffffffffffff"]);
    // the CLI channel, against a top-level body parameter
    let (o, e, c) = run_args(
        "module tb; parameter [63:0] K = 64'h1;
           initial begin $display(\"K=%h\", K); #1 $finish; end
         endmodule",
        &["-G", "K='1"],
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["K=ffffffffffffffff"]);
}

/// `defparam` supersedes the instance's own parameter assignment (IEEE §23.10.1), and
/// that has to hold when the two arrive on DIFFERENT channels. `#(.K('0))` writes the
/// fill channel and `defparam u.K = 6` writes the value channel; because the binder
/// prefers the declared-width fill re-fold, the stale `'0` won and the `defparam` was
/// silently ignored. iverilog: `K=06`.
#[test]
fn a_later_defparam_supersedes_an_earlier_fill_override() {
    let (o, e, c) = run(
        "module sub; parameter [7:0] K = 8'h11; initial $display(\"K=%h\", K); endmodule
         module top; sub #(.K('0)) u(); defparam u.K = 6; initial #1 $finish; endmodule",
    );
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["K=06"]);
}

/// A hierarchical read of a `real` parameter is honestly LOUD — and this test exists to
/// keep it that way, because the obvious fix descends the ladder.
///
/// Publishing the value to the i64 `hier_params` makes the NAME resolve but not the
/// VALUE readable (`a.P/2` then divides in the integer domain: `P=9` → 4, iverilog
/// 4.5). Patching the resolved placeholder with a REAL constant fixes that and breaks
/// the integral consumers instead — `lower_cast` already committed `int'(a.P)` to the
/// integral path while the operand was still a placeholder, so it reads the IEEE-754
/// bits (`int'` → 0, `longint'` → 4616752568008179712; `integer'` happens to survive).
/// Both were measured; each trades one silent-wrong for another, which is exactly what
/// correct-or-loud forbids. The whole axis — including the identical, PRE-existing
/// breakage for a hierarchical real VARIABLE (`a.rv`) — is deferred in ROADMAP §2.
///
/// iverilog answers `a.P/2 = 4.5`; vita says so loudly instead of guessing.
#[test]
fn a_hierarchical_read_of_a_real_parameter_is_loud() {
    let (_, e, c) = run("module sub; parameter real P = 4.5; endmodule
         module top;
           sub #(.P(9)) a();
           sub b();
           real ra;
           initial begin ra = a.P / 2; $display(\"%f\", ra); #1 $finish; end
         endmodule");
    assert_ne!(
        c,
        Some(0),
        "a real parameter must not be readable hierarchically yet"
    );
    assert!(e.contains("undeclared hierarchical name `a.P`"), "got: {e}");

    // The INTERFACE spelling is the exception, and it is kept deliberately. Its body
    // fold has always published an i64 twin to `hier_params`, and the measured value of
    // that twin is high: across 13 consumers x 6 exact-integer values it is CORRECT in
    // 72 cells and wrong in 6, and every wrong cell is `/` with a fractional quotient.
    // Removing it (which an earlier revision of this slice did, on the theory that the
    // whole thing was a silent-wrong) turned 72 correct cells loud to remove 6 — a
    // regression by the accuracy-ladder rule.
    //
    // iverilog: `int=5 x1.5=7.500000 gt=1`.
    let (o, e, c) = run("interface ifc; parameter real P = 5; endinterface
         module top;
           ifc i0();
           initial begin
             $display(\"int=%0d x1.5=%0f gt=%0d\", int'(i0.P), i0.P * 1.5, i0.P > 4);
             #1 $finish;
           end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["int=5 x1.5=7.500000 gt=1"]);
}

/// The one cell the interface i64 twin gets wrong, pinned so it cannot drift unnoticed:
/// a hierarchical `/` with a fractional quotient divides in the INTEGER domain.
/// iverilog says `2.500000`; vita says `2.000000`, in PRE and POST alike.
///
/// The BARE read of the same declaration is already correct (2.5 — see
/// `an_interface_real_parameter_divides_in_the_real_domain`), so this is the visible
/// half of the hierarchical-real axis recorded in ROADMAP §2. It is pinned rather than
/// fixed because every alternative representation breaks strictly more cells.
#[test]
fn a_hierarchical_division_of_an_interface_real_is_a_recorded_divergence() {
    let (o, e, c) = run("interface ifc; parameter real P = 5; endinterface
         module top;
           ifc i0();
           real r;
           initial begin r = i0.P / 2; $display(\"div=%0f\", r); #1 $finish; end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        lines(&o),
        ["div=2.000000"],
        "iverilog says 2.500000 — ROADMAP §2"
    );
}

/// The same primitive, reached from a DECLARATION rather than an override:
/// `parameter [7:0] A = 'x;` folded to `00` at exit 0 (iverilog: `xx`). `fill_to_i64`
/// declines an unknown fill, so the refusal covers it — and the refusal must name the
/// DOMAIN, because `'x` is a perfectly foldable constant and "not a foldable constant
/// expression" sends the reader after a syntax error that is not there. `'1` still
/// folds — iverilog `C=ff`.
///
/// (This test was deleted by accident during a revert and the loss was invisible: the
/// mutation that reverts the wording passed the whole suite while it was gone.)
#[test]
fn an_x_fill_as_a_declared_default_is_not_folded_to_zero() {
    let (_, e, c) =
        run("module tb; parameter [7:0] A = 'x; initial $display(\"A=%h\", A); endmodule");
    assert_ne!(c, Some(0));
    assert!(
        e.contains("is declared `'x`") && e.contains("no x/z plane"),
        "got: {e}"
    );

    let (o, e, c) =
        run("module tb; parameter [7:0] C = '1; initial $display(\"C=%h\", C); endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["C=ff"]);
}

/// A `-G` fill must present the SAME shape to the binder that `#(.K('1))` does — both a
/// re-foldable `fill` and the parent-side `value`. Carrying only the fill emptied
/// `by_name`, and every guard that decides a fill cannot apply reads `by_name`: these
/// three then became silent no-ops at exit 0, byte-identical to passing no flag at all.
///
/// The values are vita's own (a fill still folds at 32 bits on a width-less target —
/// ROADMAP §2); what this pins is that the flag has an EFFECT and that the string
/// target is loud, which is where PRE was.
#[test]
fn a_cli_fill_override_is_not_silently_dropped_on_a_width_less_target() {
    let src = "module tb;
           parameter S = \"abc\";
           parameter real R = 1.5;
           parameter time T = 5;
           initial begin $display(\"S=%s R=%g T=%0d\", S, R, T); #1 $finish; end
         endmodule";
    // a string target refuses a numeric override — loud, as it is for `#()`
    let (_, e, c) = run_args(src, &["-G", "S='1"]);
    assert_ne!(c, Some(0), "-G S='1 was dropped silently");
    assert!(e.contains("is a string"), "got: {e}");
    // real and time targets APPLY it (the value is vita's 32-bit fold, not iverilog's,
    // but "applies" is the property under test — dropping it is the regression)
    for (g, want) in [
        ("R='1", "S=abc R=4.29497e+09 T=5"),
        ("T='1", "S=abc R=1.5 T=4294967295"),
    ] {
        let (o, e, c) = run_args(src, &["-G", g]);
        assert_eq!(c, Some(0), "stderr: {e}");
        assert_eq!(lines(&o), [want], "-G {g} had no effect");
    }
}

/// The interface unification must not lose what the reduced fold delivered. It kept a
/// `real` body parameter usable INSIDE the interface — and that half is now better than
/// PRE: `parameter real P = 5;` divides in the real domain (`P/2 = 2.5`, matching
/// iverilog) where PRE's integer twin gave 2.0.
///
/// The cross-instance read of a real parameter is a separate axis and is honestly loud;
/// `a_hierarchical_read_of_a_real_parameter_is_loud` owns it and explains why.
#[test]
fn an_interface_real_parameter_divides_in_the_real_domain() {
    for ty in ["real", "realtime"] {
        let (o, e, c) = run(&format!(
            "interface ifc;
               parameter {ty} P = 5;
               initial $display(\"P/2=%0f d=%0d\", P / 2, P);
             endinterface
             module top; ifc i0(); initial #1 $finish; endmodule"
        ));
        assert_eq!(c, Some(0), "{ty}: stderr: {e}");
        assert_eq!(lines(&o), ["P/2=2.500000 d=5"], "{ty}");
    }
}

/// The interface body loop binds EVERY declaration through the shared binder, not just
/// the overridden ones. Its own reduced fold recorded neither `param_meta` (declared
/// width and sign) nor `param_range`, so whether an override happened to target a
/// parameter changed that parameter's WIDTH — two instances of one interface disagreed
/// inside a single run at exit 0, with the SAME value.
///
/// iverilog: `a.X=-1  b.X=-1`.
#[test]
fn an_interface_parameter_has_one_width_whether_or_not_it_is_overridden() {
    let (o, e, c) = run("interface ifc;
           parameter signed [7:0] P = -3;
           localparam X = P / 2;
         endinterface
         module top;
           ifc #(.P(-3)) a();
           ifc           b();
           initial begin #1; $display(\"a.X=%0d  b.X=%0d\", a.X, b.X); $finish; end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["a.X=-1  b.X=-1"]);
}

/// The same repair, measured on an interface parameter that is never overridden at all:
/// its declared width and sign were simply lost. iverilog: `P=a5 P[15:12]=a cat=a5a5`
/// (vita printed `P=000000a5 P[15:12]=0 cat=000000a5000000a5` — silently 32-bit), and
/// a `string` interface parameter was loud on its own declared default.
#[test]
fn an_interface_parameter_keeps_its_declared_width_and_its_string_route() {
    let (o, e, c) = run("interface ifc;
           parameter [15:8] P = 8'hA5;
           initial $display(\"P=%h P[15:12]=%h cat=%h\", P, P[15:12], {P,P});
         endinterface
         module top; ifc i0(); initial begin #1; $finish; end endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["P=a5 P[15:12]=a cat=a5a5"]);

    // iverilog: `S=abc`
    let (o2, e2, c2) = run(
        "interface ifc; parameter S = \"abc\"; initial $display(\"S=%s\", S); endinterface
         module top; ifc i0(); initial begin #1; $finish; end endmodule",
    );
    assert_eq!(c2, Some(0), "stderr: {e2}");
    assert_eq!(lines(&o2), ["S=abc"]);
}

// ── what must STAY loud (iverilog rejects each of these too) ────────────────

/// iverilog: "Cannot override localparam `L`". Naming a body localparam must give the
/// PRECISE message, not "unknown parameter" — which is why localparams are in the
/// resolved list even though they are not positional slots.
#[test]
fn overriding_a_body_localparam_is_loud_and_names_the_reason() {
    let (_, e, c) = run("module sub;
           localparam L = 2;
           initial $display(\"L=%0d\", L);
         endmodule
         module tb; sub #(.L(9)) u1 (); endmodule");
    assert_ne!(c, Some(0));
    assert!(
        e.contains("cannot override localparam `L`"),
        "expected the localparam reason, got: {e}"
    );
}

/// iverilog: "`-G L=…` targets a localparam" equivalent ("Cannot override localparam").
#[test]
fn cli_g_on_a_body_localparam_is_loud() {
    let (_, e, c) = run_args(
        "module tb; localparam L = 2; initial $display(\"L=%0d\", L); endmodule",
        &["-G", "L=9"],
    );
    assert_ne!(c, Some(0));
    assert!(
        e.contains("targets a localparam"),
        "expected the localparam reason, got: {e}"
    );
}

/// iverilog: "parameter `GP` not found in `tb.u1`" — a parameter declared inside a
/// `generate` block is in a different scope and is not part of the port list.
#[test]
fn a_generate_scope_parameter_is_not_overridable() {
    let (_, e, c) = run("module sub;
           parameter A = 1;
           generate if (1) begin : g
             parameter GP = 5;
           end endgenerate
           initial $display(\"A=%0d\", A);
         endmodule
         module tb; sub #(.GP(9)) u1 (); endmodule");
    assert_ne!(c, Some(0));
    assert!(
        e.contains("unknown parameter `GP`"),
        "expected an unknown-parameter refusal, got: {e}"
    );
}

/// iverilog: "Cannot override parameter `W` in `tb.u1`. Parameter cannot be overridden
/// in the scope it has been declared in." A module that HAS an ANSI header keeps its
/// body parameters non-overridable — the implicit list exists only when there is no
/// explicit one.
#[test]
fn a_body_parameter_of_an_ansi_header_module_stays_non_overridable() {
    let (_, e, c) = run("module sub #(parameter A = 1);
           parameter W = 5;
           initial $display(\"A=%0d W=%0d\", A, W);
         endmodule
         module tb; sub #(.A(7), .W(8)) u1 (); endmodule");
    assert_ne!(c, Some(0));
    assert!(
        e.contains("unknown parameter `W`"),
        "expected `W` to be refused, got: {e}"
    );
}

/// A `-G` naming nothing at all stays loud (the implicit list must not turn an
/// unmatched name into a silent no-op).
#[test]
fn cli_g_with_no_matching_parameter_is_loud() {
    let (_, e, c) = run_args(
        "module tb; parameter P = 1; initial $display(\"P=%0d\", P); endmodule",
        &["-G", "ZZ=9"],
    );
    assert_ne!(c, Some(0));
    assert!(e.contains("names no parameter of any top module"), "{e}");
}

// ── staged `-G` (doc-14 RULE B) ─────────────────────────────────────────────
