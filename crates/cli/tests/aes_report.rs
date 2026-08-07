//! The aes_top external report (2026-08-07) — every item, pinned to iverilog 13.0.
//!
//! Twelve items arrived as "vita rejects what iverilog runs" plus five "vita is silent
//! where it should not be". The fixes touched the parser, the elaborator's name
//! resolution, three parameter-binding paths, the port wiring and the runtime range
//! diagnostics — and the adversarial review that followed found that **none of the new
//! behaviour had a test**. Every mutation it built against those features survived the
//! whole suite. This file is that gate.
//!
//! Each test asserts a VALUE that iverilog 13.0 produces (recorded per test), not merely
//! that elaboration succeeds: a fix that wires the wrong element, drops the second
//! import term or keeps the declared default would still "work" under an
//! elaborates-cleanly assertion.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` (plus extra argv), return (stdout, stderr, exit code).
fn run_args(src: &str, extra: &[&str]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_aesrep_{}_{n}", std::process::id()));
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
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn run(src: &str) -> (String, String, Option<i32>) {
    run_args(src, &[])
}

/// The single `R=…` line a design prints, without the prefix.
fn r(src: &str) -> String {
    let (o, e, c) = run(src);
    assert_eq!(c, Some(0), "expected a clean run.\nstdout:{o}\nstderr:{e}");
    o.lines()
        .find_map(|l| l.trim().strip_prefix("R=").map(str::to_owned))
        .unwrap_or_else(|| panic!("no `R=` line.\nstdout:{o}\nstderr:{e}"))
}

// ─────────────────────────── §3.2 unpacked array ports ───────────────────────────

/// ANSI `output logic [7:0] o [4]` — parsed, sized, and wired ELEMENT BY ELEMENT.
///
/// vita rejected this at the `[` (E2002) in both the ANSI header and the non-ANSI body
/// form, so 15×128-bit round keys had to be flattened to a packed bus by hand. There is
/// no whole-array value in this IR, so one whole-net cont-assign would have connected
/// word 0 only — the assertion is the LAST element, which only per-element wiring
/// reaches. iverilog 13: `R=1 4`.
#[test]
fn unpacked_array_port_ansi_wires_every_element() {
    assert_eq!(
        r("module ch (input logic c, output logic [7:0] o [4]);\n\
             always_comb for (int i=0;i<4;i++) o[i] = 8'(i+1);\n\
           endmodule\n\
           module t; logic c=0; logic [7:0] a [4]; ch u(.c(c), .o(a));\n\
             initial begin #1 $display(\"R=%0d %0d\", a[0], a[3]); $finish; end\n\
           endmodule\n"),
        "1 4"
    );
}

/// The NON-ANSI body form (`output logic [7:0] o [4];`) must size the port the same
/// way. Its AST node carries `names: Vec<Ident>` with no room for dims, which is why
/// `PortDecl` grew a parallel `unpacked: Vec<Vec<Dim>>`. iverilog 13: `R=1 4`.
#[test]
fn unpacked_array_port_non_ansi_wires_every_element() {
    assert_eq!(
        r("module ch (c, o);\n\
             input logic c; output logic [7:0] o [4];\n\
             always_comb for (int i=0;i<4;i++) o[i] = 8'(i+1);\n\
           endmodule\n\
           module t; logic c=0; logic [7:0] a [4]; ch u(.c(c), .o(a));\n\
             initial begin #1 $display(\"R=%0d %0d\", a[0], a[3]); $finish; end\n\
           endmodule\n"),
        "1 4"
    );
}

/// A multi-dimensional port array. The per-dim extents have to be recorded, not just
/// the flattened count: without `array_dims` a two-index write `o[i][0]` cannot compute
/// its flat element and the port read back all-X while the same array declared as an
/// ordinary net worked. iverilog 13: `R=1 4`.
#[test]
fn unpacked_array_port_multi_dim_keeps_its_geometry() {
    assert_eq!(
        r("module ch (output logic [7:0] o [4][2]);\n\
             always_comb for (int i=0;i<4;i++) o[i][0] = 8'(i+1);\n\
           endmodule\n\
           module t; logic [7:0] a [4][2]; ch u(.o(a));\n\
             initial begin #1 $display(\"R=%0d %0d\", a[0][0], a[3][0]); $finish; end\n\
           endmodule\n"),
        "1 4"
    );
}

/// An INPUT array port, the direction the round-key case actually needs.
/// iverilog 13: `R=ff`.
#[test]
fn unpacked_array_port_input_direction() {
    assert_eq!(
        r(
            "module ch (input logic [7:0] i [4], output logic [7:0] o);\n\
             always_comb o = i[0] ^ i[3];\n\
           endmodule\n\
           module t; logic [7:0] a [4]; logic [7:0] q; ch u(.i(a), .o(q));\n\
             initial begin a[0]=8'h0f; a[3]=8'hf0; #1 $display(\"R=%h\", q); $finish; end\n\
           endmodule\n"
        ),
        "ff"
    );
}

/// The four shapes `wire_array_port` must REFUSE rather than wire.
///
/// Every one was a survivor: with only the working tests above, dropping the length
/// check walks off the end of the shorter array, dropping the geometry check wires
/// `[4]` to `[2][2]` silently (iverilog: "Unpacked dimensions are not compatible"),
/// dropping the element-width check truncates silently (iverilog: "Element types are
/// not compatible"), and connecting `[0:3]` to `[3:0]` REVERSES the elements — IEEE
/// 1800 §7.6 pairs them by POSITION, so iverilog answers 1 where flat-index wiring
/// answers 4. That last one was measured as a live silent-wrong in this slice's own
/// new code, which is why the direction test is here rather than in a comment.
#[test]
fn unpacked_array_port_mismatches_are_loud() {
    let child = "module ch (output logic [7:0] o [4]);\n\
                   always_comb for (int i=0;i<4;i++) o[i] = 8'(i+1);\n\
                 endmodule\n";
    for (what, decl) in [
        ("length", "logic [7:0] a [2]"),
        ("geometry", "logic [7:0] a [2][2]"),
        ("element width", "logic [3:0] a [4]"),
        ("direction", "logic [7:0] a [3:0]"),
    ] {
        let (_o, e, c) = run(&format!(
            "{child}module t; {decl}; ch u(.o(a));\n\
               initial begin #1 $display(\"R=%0d\", a[0]); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(c, Some(1), "a {what} mismatch must be loud:\n{e}");
        assert!(
            e.contains("VITA-E3002"),
            "a {what} mismatch must be a port-mismatch diagnostic:\n{e}"
        );
    }
}

// ───────────────────── §3.3 / §3.3b package-scope resolution ─────────────────────

/// A selectively imported function's body resolves in ITS OWN package.
///
/// `import p::f2;` left `f2`'s call to same-package `f1` unresolvable (E3010) while
/// `import p::*` worked BY ACCIDENT — the wildcard happens to put every sibling in the
/// table. The third column is the one that makes this more than "find it at all": the
/// module declares its own `helper`, and before the fix `f2`'s body called THAT one and
/// answered 1002. iverilog 13: `R=4 1001` for both import spellings.
#[test]
fn package_routine_body_resolves_in_its_own_package() {
    let pkg = "package p;\n\
                 function automatic int helper(input int a); return a+1; endfunction\n\
                 function automatic int f2(input int a); return helper(a)*2; endfunction\n\
               endpackage\n";
    for imp in ["import p::f2;", "import p::*;"] {
        assert_eq!(
            r(&format!(
                "{pkg}module t; {imp}\n\
                   function automatic int helper(input int a); return a+1000; endfunction\n\
                   initial begin #1 $display(\"R=%0d %0d\", f2(1), helper(1)); $finish; end\n\
                 endmodule\n"
            )),
            "4 1001",
            "with `{imp}`: the package body must call the PACKAGE's helper, and the \
             module must keep its own"
        );
    }
}

/// Same rule for a package's own CONSTANTS, and for a package TASK.
///
/// The task half of `resolve_rtn_key` was left unwired: a package task calling a
/// same-package task reported "undeclared task", and with a module-local task of the
/// same name it called that one silently (1001 vs iverilog 43). iverilog 13: `R=7 43`.
#[test]
fn package_task_body_sees_its_own_constants_and_siblings() {
    assert_eq!(
        r("package p;\n\
             localparam int K = 7;\n\
             task automatic inner(output int o); o = 43; endtask\n\
             task automatic tk(output int a, output int b); begin a = K; inner(b); end endtask\n\
           endpackage\n\
           module t; import p::tk;\n\
             localparam int K = 999;\n\
             task automatic inner(output int o); o = 1001; endtask\n\
             int x, y;\n\
             initial begin #1 tk(x, y); $display(\"R=%0d %0d\", x, y); $finish; end\n\
           endmodule\n"),
        "7 43"
    );
}

/// A `pkg::f()` call with CONTROL FLOW and a nested same-package call.
///
/// The admission used to demand a "self-contained, straight-line" body. The
/// no-control-flow half was never a name-resolution property — the frame path lowers
/// arbitrary CFGs — and the nested-call half fell once the body resolved in its own
/// package. The `return h(m)` spelling is deliberate: the callee-collection walk had no
/// `Return` arm, so `h` stayed invisible and this exact design still failed after the
/// scope fix while the byte-identical `g = h(m)` worked. iverilog 13: `R=11 1`.
#[test]
fn package_scoped_call_admits_control_flow_and_nested_calls() {
    assert_eq!(
        r("package p;\n\
             function automatic int h(input int x); return x+1; endfunction\n\
             function automatic int g(input int m); return h(m); endfunction\n\
             function automatic int cf(input int m); if (m>10) return 1; else return 0; endfunction\n\
           endpackage\n\
           module t; int a, b;\n\
             initial begin #1 a = p::g(10); b = p::cf(20);\n\
               $display(\"R=%0d %0d\", a, b); $finish; end\n\
           endmodule\n"),
        "11 1"
    );
}

/// A callee hidden inside a CAST is admitted by the purity walk, so the collection walk
/// must reach it too — otherwise it is never injected under its scoped key, the bare
/// name falls back to the caller module's function, and `p::g(1)` answers 1001 at exit
/// 0 where it had been loud. An admission walk and a collection walk that disagree
/// about the node set is the recorded "accept-gate walker completeness" hazard.
/// iverilog 13: `R=2`.
#[test]
fn package_callee_inside_a_cast_is_still_collected() {
    assert_eq!(
        r("package p;\n\
             function automatic int h(input int x); return x+1; endfunction\n\
             function automatic int g(input int x); return int'(h(x)); endfunction\n\
           endpackage\n\
           module t;\n\
             function automatic int h(input int x); return x+1000; endfunction\n\
             initial begin #1 $display(\"R=%0d\", p::g(1)); $finish; end\n\
           endmodule\n"),
        "2"
    );
}

/// Purity is checked on the ROOT *and* on every transitively injected callee.
///
/// The injection walk is transitive while the check was not, so a sibling whose body
/// names something FREE was injected anyway and then lowered with the caller module's
/// tables live — binding that free name to the caller's net at exit 0. iverilog rejects
/// it ("Unable to bind wire/reg/memory `zz` in `p.h`").
#[test]
fn a_package_callee_with_a_free_name_is_loud_not_silent() {
    let (o, e, c) = run("package p;\n\
           function automatic int h(input int x); return x + zz; endfunction\n\
           function automatic int g(input int m); return h(m); endfunction\n\
         endpackage\n\
         module t; int zz; int o;\n\
           initial begin zz = 100; o = p::g(1); $display(\"R=%0d\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(1), "a free name in a callee must be loud:\n{o}{e}");
    assert!(
        e.contains("VITA-E3009") && e.contains("p::h"),
        "the diagnostic must name the callee that is not closed:\n{e}"
    );
}

// ────────────────────────────── §3.4 comma imports ──────────────────────────────

/// `import p::f1, p::f2;` — IEEE 1800 §26.8's `package_import_declaration` is a comma
/// list. Asserting the VALUE matters: a list that dropped its second term would still
/// elaborate, with `B` unresolved only at use. iverilog 13: `R=3 4`.
#[test]
fn comma_import_binds_every_term() {
    assert_eq!(
        r(
            "package p; localparam int A=3; localparam int B=4; endpackage\n\
           module t; import p::A, p::B;\n\
             initial begin #1 $display(\"R=%0d %0d\", A, B); $finish; end\n\
           endmodule\n"
        ),
        "3 4"
    );
}

// ─────────────────── §3.5 function calls in a port connection ───────────────────

/// A port connection's actual is a PARENT expression, but `wire_ports` runs inside the
/// CHILD's elaboration with the child's routine tables installed. So `.i(mk(2))` looked
/// `mk` up in the child — "call to undeclared function" — and when the child happened
/// to declare its own `mk`, the parent's connection bound the CHILD's silently. The
/// child here declares a different `mk` on purpose: that is the only column that
/// separates "resolves" from "resolves to the right one". iverilog 13: `R=0100`.
#[test]
fn port_connection_calls_the_parents_function() {
    assert_eq!(
        r("module ch (input logic [3:0] i, output logic [3:0] o);\n\
             function automatic logic [3:0] mk(input int k); return 4'(1 << (k+1)); endfunction\n\
             assign o = i;\n\
           endmodule\n\
           module t;\n\
             function automatic logic [3:0] mk(input int k); return 4'(1 << k); endfunction\n\
             logic [3:0] q; ch u(.i(mk(2)), .o(q));\n\
             initial begin #1 $display(\"R=%b\", q); $finish; end\n\
           endmodule\n"),
        "0100"
    );
}

// ───────────────────────────── §3.6 parameter-width casts ─────────────────────────

/// `RPS'(1)` — a size cast whose size is a NAME. The parser makes a bare identifier a
/// `Named` cast target whatever it turns out to be, and only the runtime lowering
/// resolved the parameter reading; the constant domain did not, so the expression
/// lowered fine and then failed to FOLD. Not generate-specific — module scope too.
/// iverilog 13: `R=01 10`.
#[test]
fn size_cast_with_a_parameter_width_folds() {
    assert_eq!(
        r("module t;\n\
             localparam int RPS = 2;\n\
             localparam logic [RPS-1:0] A = RPS'(1);\n\
             localparam logic [RPS-1:0] B = RPS'(1) << (RPS-1);\n\
             initial begin #1 $display(\"R=%b %b\", A, B); $finish; end\n\
           endmodule\n"),
        "01 10"
    );
}

/// A generate-scope localparam must record its DECLARED width, like every other scope.
/// It did not, so `logic [1:0] M` read back as 32 bits — `%b` printed thirty leading
/// zeros, and the same width fed concats and comparisons. iverilog 13: `R=01`.
#[test]
fn generate_scope_localparam_keeps_its_declared_width() {
    assert_eq!(
        r("module t;\n\
             genvar k;\n\
             generate for (k=0;k<1;k++) begin : g\n\
               localparam logic [1:0] M = 2'b01;\n\
               initial begin #1 $display(\"R=%b\", M); $finish; end\n\
             end endgenerate\n\
           endmodule\n"),
        "01"
    );
}

// ──────────────────────────── §3.7 string parameters ────────────────────────────

/// A `string` parameter binds, a string OVERRIDE applies, and `MODE == "Y"` folds as a
/// generate-if condition. All three were needed: the header path never routed strings
/// at all (so the declared DEFAULT was E3009), the override channel is i64-only (so the
/// override was dropped), and the equality had no string domain to fold in.
/// iverilog 13: `a` for the override, `5` for the default.
#[test]
fn string_parameter_binds_and_its_equality_folds() {
    let leaf = "module leaf #(parameter string MODE=\"X\") (output logic [3:0] o);\n\
                  generate if (MODE==\"Y\") assign o = 4'hA; else assign o = 4'h5; endgenerate\n\
                endmodule\n";
    assert_eq!(
        r(&format!(
            "{leaf}module t; logic [3:0] q; leaf #(.MODE(\"Y\")) u(.o(q));\n\
               initial begin #1 $display(\"R=%h\", q); $finish; end endmodule\n"
        )),
        "a"
    );
    assert_eq!(
        r(&format!(
            "{leaf}module t; logic [3:0] q; leaf u(.o(q));\n\
               initial begin #1 $display(\"R=%h\", q); $finish; end endmodule\n"
        )),
        "5",
        "the declared default must bind too — it was E3009 with no override present"
    );
}

/// Every route that sends a parameter to a side map must run the unfoldable-override
/// escalation FIRST.
///
/// The escalation was added for this report and then placed after the string / wide /
/// real routes, each of which `continue`s — so the very types most likely to carry an
/// unfoldable override skipped it. Two lenses found that independently. iverilog
/// rejects all three of these; a numeric override on a string parameter is a type
/// mismatch it also refuses.
#[test]
fn an_override_that_does_not_apply_is_never_silent() {
    for (what, src) in [
        (
            "string parameter, non-constant override",
            "module ch #(parameter string M=\"X\")(); initial $display(\"M=%s\",M); endmodule\n\
             module t; logic [3:0] sig; ch #(.M(sig)) u();\n\
               initial begin sig=1; #1 $finish; end endmodule\n",
        ),
        (
            "string parameter, numeric override",
            "module ch #(parameter string M=\"X\")(); initial $display(\"M=%s\",M); endmodule\n\
             module t; ch #(.M(5)) u(); initial begin #1 $finish; end endmodule\n",
        ),
        (
            "int parameter, non-constant override",
            "module ch #(parameter int W=5)(); initial $display(\"W=%0d\",W); endmodule\n\
             module t; logic [3:0] sig; ch #(.W(sig)) u();\n\
               initial begin sig=1; #1 $finish; end endmodule\n",
        ),
    ] {
        let (o, e, c) = run(src);
        assert_eq!(c, Some(1), "{what} must be loud:\n{o}{e}");
    }
    // …and `.W()` with NO expression legally means "keep the default": still silent.
    let (o, _e, c) = run(
        "module ch #(parameter int W=5)(); initial $display(\"R=%0d\",W); endmodule\n\
         module t; ch #(.W()) u(); initial begin #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        o.contains("R=5"),
        "an empty override keeps the default: {o}"
    );
}

// ─────────────────────── §3.8 parameters wider than 64 bits ───────────────────────

/// A 256-bit key parameter keeps its full bit pattern — the AES-256 case.
///
/// The i64 constant domain refuses any literal with a set bit above word 0, so this was
/// E3009 on a value that is perfectly known. iverilog 13: `R=00…1e1f 0001020304050607 1f`.
#[test]
fn parameter_wider_than_64_bits_keeps_its_value() {
    assert_eq!(
        r("module t;\n\
             localparam logic [255:0] K =\n\
               256'h000102030405060708090a0b0c0d0e0f_101112131415161718191a1b1c1d1e1f;\n\
             initial begin #1 $display(\"R=%h %h\", K[255:192], K[7:0]); $finish; end\n\
           endmodule\n"),
        "0001020304050607 1f"
    );
}

/// The boundary is the VALUE, not the declared width.
///
/// Gating the wide route on the declared width took four declaration scopes from
/// correct to loud: a 96-bit parameter holding 3 lost its integer identity, so
/// `localparam int Y = X+1`, `logic [X-1:0]` and `generate if (X > 3)` all became
/// E3009 — while the field doc asserted the opposite. iverilog 13: `R=4 3`.
#[test]
fn a_wide_parameter_whose_value_fits_is_still_an_integer() {
    assert_eq!(
        r("module t;\n\
             localparam logic [95:0] X = 96'd3;\n\
             localparam int Y = X + 1;\n\
             logic [X-1:0] bus;\n\
             initial begin #1 $display(\"R=%0d %0d\", Y, $bits(bus)); $finish; end\n\
           endmodule\n"),
        "4 3"
    );
}

/// A wide parameter carries its declared SIGN, and its declared WIDTH on read-back.
///
/// `signed: false` was hard-coded with the note that claiming a sign "would flip a
/// comparison"; measurement says the unsigned choice is what flips it when the
/// declaration says `signed`. And a wide parameter holding an i64 used to read back at
/// the value-inferred width, so `%h` printed 8 digits where iverilog prints 24.
/// iverilog 13: `R=NEG 000000000000000000000007`.
#[test]
fn a_wide_parameter_carries_its_sign_and_width() {
    assert_eq!(
        r("module t;\n\
             localparam signed [127:0] K = 128'sh8000_0000_0000_0000_0000_0000_0000_0001;\n\
             localparam logic [95:0] S = 96'd7;\n\
             initial begin #1 $display(\"R=%s %h\", (K < 0) ? \"NEG\" : \"POS\", S); $finish; end\n\
           endmodule\n"),
        "NEG 000000000000000000000007"
    );
}

/// A >64-bit parameter has no integral constant value for a width or bound, and says so
/// by name rather than calling a declared name "undefined".
#[test]
fn a_wide_parameter_used_as_a_width_is_loud() {
    let (_o, e, c) = run("module t;\n\
           localparam logic [255:0] K =\n\
             256'hffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;\n\
           logic [K-1:0] v;\n\
           initial begin #1 $display(\"R=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(1));
    assert!(
        e.contains("wider than 64 bits"),
        "the diagnostic must say what `K` actually is:\n{e}"
    );
}

// ───────────────────────── §4.4 use before declaration ─────────────────────────

/// IEEE 1800 §6.10: a net or variable may not be used above its declaration. vita
/// lowers by pass, so the name resolved anyway and the design ran at errors=0 — the
/// worst shape a gap can take, because a synthesis tool either rejects the file or
/// gives the name a 1-bit implicit net. iverilog rejects all four shapes below.
#[test]
fn use_before_declaration_is_loud() {
    for (what, body) in [
        ("continuous-assign rhs", "logic o; assign o = ~q; logic q;"),
        ("continuous-assign lhs", "assign q = 1'b1; logic q;"),
        ("procedural rhs", "logic o; initial o = ~q; logic q;"),
        ("always_comb", "logic o; always_comb o = ~q; logic q;"),
    ] {
        let (_o, e, c) = run(&format!(
            "module t; {body}\n  initial begin #1 $finish; end endmodule\n"
        ));
        assert_eq!(c, Some(1), "{what} must be loud:\n{e}");
        assert!(e.contains("used before it is declared"), "{what}:\n{e}");
    }
}

/// …and the forward references that stay LEGAL must be untouched. Subroutines, module
/// definitions and instance names resolve through other tables; a plain-static BLOCK
/// LOCAL is the trap, because vita's flatten model publishes it on the module key, so
/// the naive check rejected an ordinary testbench five times over. iverilog runs all of
/// these. `R=4 1 1 3`.
#[test]
fn legal_forward_references_are_untouched() {
    assert_eq!(
        r("module t;\n\
             logic [3:0] fw; assign fw = mk(2);\n\
             function automatic logic [3:0] mk(input int k); return 4'(1<<k); endfunction\n\
             logic li; leaf u(.y(li));\n\
             logic hi; assign hi = u.inner;\n\
             initial begin : blk integer i;\n\
               for (i=0;i<3;i=i+1) ;\n\
               #1 $display(\"R=%0d %0d %0d %0d\", fw, li, hi, i); $finish;\n\
             end\n\
             integer i;\n\
           endmodule\n\
           module leaf(output logic y); logic inner = 1'b1; assign y = inner; endmodule\n"),
        "4 1 1 3"
    );
}

// ───────────────────── §4.1 unknown vs out-of-range array index ─────────────────

/// An UNKNOWN (x/z) index and a KNOWN out-of-range index are different facts.
///
/// Both were E4002 at Error severity, so reading `mem[idx_q]` during reset filled the
/// log with errors and set exit 1 on correct RTL. IEEE 1364 §5.2.1 makes an unknown
/// index read X and drop the write — which is what vita already did; only the
/// diagnostic was wrong about it. A KNOWN index past the end stays an error.
///
/// The third row is the one a single `u32::MAX` sentinel cannot get right: 1073741825
/// is a perfectly known value that happens to equal the unknown sentinel on the WRITE
/// side, and it was classified "unknown (x/z)" at exit 0.
#[test]
fn unknown_and_out_of_range_indexes_are_different_diagnostics() {
    for backend in ["interp", "bytecode", "native"] {
        let (_o, e, c) = run_args(
            "module t; reg [7:0] m[0:3]; reg [7:0] o; reg [1:0] i;\n\
               initial begin i = 2'bxx; o = m[i]; $display(\"R=%h\", o); $finish; end\n\
             endmodule\n",
            &["--backend", backend],
        );
        assert_eq!(
            c,
            Some(0),
            "[{backend}] an unknown index is not an error:\n{e}"
        );
        assert!(e.contains("VITA-W4029"), "[{backend}]:\n{e}");

        for (what, idx) in [
            ("known out of range", "9"),
            ("sentinel collision", "1073741825"),
        ] {
            let (_o, e, c) = run_args(
                &format!(
                    "module t; reg [7:0] m[0:3]; integer i;\n\
                       initial begin i = {idx}; m[i] = 8'hEE; $display(\"R=%h\", m[0]); $finish; end\n\
                     endmodule\n"
                ),
                &["--backend", backend],
            );
            assert_eq!(c, Some(1), "[{backend}] {what} stays loud:\n{e}");
            assert!(e.contains("VITA-E4002"), "[{backend}] {what}:\n{e}");
        }
    }
}

/// The deferred drain must preserve SOURCE ORDER across backends. Two per-kind counters
/// cannot: native replayed every E4002 before every W4029 while the other two backends
/// followed the source, and the split is what made that observable.
#[test]
fn deferred_range_diagnostics_keep_their_order() {
    let mut seen = Vec::new();
    for backend in ["interp", "bytecode", "native"] {
        let (_o, e, _c) = run_args(
            "module t; reg [7:0] m[0:3]; integer xi, oi;\n\
               initial begin xi = 'x; oi = 9;\n\
                 $display(\"R=%h %h\", m[xi], m[oi]); $finish; end\n\
             endmodule\n",
            &["--backend", backend],
        );
        let order: Vec<&str> = e
            .lines()
            .filter_map(|l| {
                if l.contains("VITA-W4029") {
                    Some("W")
                } else if l.contains("VITA-E4002") {
                    Some("E")
                } else {
                    None
                }
            })
            .collect();
        seen.push((backend, order.join("")));
    }
    for (backend, order) in &seen {
        assert_eq!(
            order, "WE",
            "[{backend}] the unknown index comes first in the source: {seen:?}"
        );
    }
}

// ─────────────────────────── §4.2 `$sscanf` scanset ───────────────────────────

/// `%[...]` — a C scanset, which IEEE 1800 §21.3.4.2 defers to.
///
/// Neither vita nor iverilog implemented it; iverilog at least REFUSES it loudly
/// ("invalid format code: %["), while vita matched nothing and returned 0 with no
/// diagnostic. That is how an AES vector file's `#keylen=256` header silently fell back
/// to defaults and left the 192/256-bit and decrypt paths untested behind a PASS.
/// Expectations are C's.
#[test]
fn sscanf_scanset_matches_c() {
    for (what, src, want) in [
        (
            "negated set, the reported header",
            "\"#keylen=256\"",
            "2 keylen 256",
        ),
        ("range", "\"abc9\"", "1 abc "),
    ] {
        let fmt = if want.starts_with('2') {
            "\"#%[^=]=%s\""
        } else {
            "\"%[a-z]\""
        };
        let got = r(&format!(
            "module t; string s = {src}; string a, b; int n;\n\
               initial begin n = $sscanf(s, {fmt}, a, b);\n\
                 $display(\"R=%0d %s %s\", n, a, b); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(got.trim_end(), want.trim_end(), "{what}");
    }
    // A `]` first is a literal; an explicit width bounds the run; matching NOTHING
    // fails the conversion (n=0) rather than returning an empty success.
    assert_eq!(r("module t; string a; int n;\n\
           initial begin n = $sscanf(\"]ab\", \"%[]a]\", a); $display(\"R=%0d %s\", n, a); $finish; end\n\
         endmodule\n"), "1 ]a");
    assert_eq!(r("module t; string a; int n;\n\
           initial begin n = $sscanf(\"abcdef\", \"%3[a-z]\", a); $display(\"R=%0d %s\", n, a); $finish; end\n\
         endmodule\n"), "1 abc");
    assert_eq!(r("module t; string a; int n;\n\
           initial begin n = $sscanf(\"999\", \"%[a-z]\", a); $display(\"R=%0d\", n); $finish; end\n\
         endmodule\n"), "0");
    // …and assignment suppression works on it like any other conversion.
    assert_eq!(r("module t; int v, n;\n\
           initial begin n = $sscanf(\"abc 7\", \"%*[a-z] %d\", v); $display(\"R=%0d %0d\", n, v); $finish; end\n\
         endmodule\n"), "1 7");
}

// ────────────────────── §4.3 a ternary of string literals ──────────────────────

/// ⚠️ NOT a value fix — vita and iverilog print the identical number, because IEEE 1800
/// §5.9 makes a string literal a packed integral and a single non-format argument
/// prints as decimal. The warning exists because the shape is always a mistake and it
/// silently made a whole test log unreadable. Narrow on purpose: a `%s` format, a
/// ternary of string VARIABLES and a numeric ternary must NOT trip it.
#[test]
fn a_display_ternary_of_string_literals_warns_without_changing_the_value() {
    let (o, e, c) = run("module t; int n = 1;\n\
           initial begin $display(n == 1 ? \"[PASS] a\" : \"[FAIL] b\"); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(e.contains("VITA-W3058"), "the shape must warn:\n{e}");
    // The exact number depends on the literal lengths; what must hold is that the
    // value is still the INTEGRAL one (a decimal run, no text) — iverilog prints the
    // identical digits for the identical source.
    let printed = o
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    assert!(
        printed.chars().all(|c| c.is_ascii_digit()) && !printed.is_empty(),
        "…and the VALUE must not change — it stays the packed integer:\n{o}"
    );
    assert!(!o.contains("PASS"), "the text must NOT be printed:\n{o}");

    let (_o, e, c) = run("module t; int n = 1; string a = \"x\", b = \"y\";\n\
           initial begin\n\
             $display(\"%s\", n==1 ? \"A\" : \"B\");\n\
             $display(n==1 ? a : b);\n\
             $display(n==1 ? 8'd1 : 8'd2);\n\
             $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(!e.contains("VITA-W3058"), "no false positives:\n{e}");
}

// ─────────────────────── §4.5 partial `timescale` coverage ───────────────────────

/// IEEE 1800-2017 §3.14.2.2: if any module has a `timescale, every module must.
///
/// vita ran the mixed design at errors=0 warnings=0 — and so does iverilog — but xrun
/// refuses to elaborate it (`*F,CUMSTS`) and Verilator reports `Error-TIMESCALEMOD`. A
/// user shipped a vita-green design to sign-off on that. The message must NAME the
/// ungoverned modules; "somewhere in ten files" is not actionable.
#[test]
fn a_partial_timescale_is_reported_and_names_the_modules() {
    let (o, e, c) = run(
        "module leaf(input logic i, output logic o); assign o = ~i; endmodule\n\
         `timescale 1ns/1ps\n\
         module top; logic a=0, b; leaf u(.i(a), .o(b));\n\
           initial begin #1 a=1; #1 $display(\"R=%0b\", b); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(c, Some(0), "the design still runs:\n{o}{e}");
    assert!(
        e.contains("VITA-W1018"),
        "the mixed form must be reported:\n{e}"
    );
    assert!(
        e.contains("leaf"),
        "…and must name the ungoverned module:\n{e}"
    );

    // A design with NO timescale anywhere is the OTHER diagnostic, not this one.
    let (_o, e, _c) = run("module t; initial begin #1 $display(\"R=1\"); $finish; end endmodule\n");
    assert!(e.contains("VITA-W1017") && !e.contains("VITA-W1018"), "{e}");
}

// ──────────────────── §3.11 CLI top-parameter override (-G) ────────────────────

/// `-G NAME=VALUE` / `--param NAME=VALUE`, in every spelling.
///
/// Without it a configuration sweep needs one hand-written wrapper module per
/// combination — four for two parameters, sixteen for four — and the same filelist
/// cannot be shared with a tool that does support overrides.
#[test]
fn cli_parameter_override_applies_to_the_top_module() {
    let src = "module t #(parameter int RPS = 2, parameter int CF = 1,\n\
                           parameter string MODE = \"def\") ();\n\
                 initial begin #1 $display(\"R=%0d %0d %s\", RPS, CF, MODE); $finish; end\n\
               endmodule\n";
    let get = |extra: &[&str]| {
        let (o, e, c) = run_args(src, extra);
        assert_eq!(c, Some(0), "{extra:?}:\n{o}{e}");
        o.lines()
            .find_map(|l| l.trim().strip_prefix("R=").map(str::to_owned))
            .unwrap_or_default()
    };
    assert_eq!(get(&[]), "2 1 def");
    assert_eq!(get(&["-G", "RPS=1"]), "1 1 def");
    assert_eq!(get(&["-GRPS=1"]), "1 1 def", "the attached spelling too");
    assert_eq!(get(&["--param", "RPS=1", "--param", "CF=0"]), "1 0 def");
    assert_eq!(get(&["-G", "RPS=8'd4"]), "4 1 def", "sized literals");
    assert_eq!(get(&["-G", "MODE=\"lut\""]), "2 1 lut", "strings");
}

/// An override that cannot apply is loud — the same rule as `#(.W(sig))`. A CLI
/// override that silently did not apply would be the identical failure mode.
#[test]
fn a_cli_parameter_override_that_cannot_apply_is_loud() {
    let src = "module t #(parameter int RPS = 2) ();\n\
                 initial begin #1 $display(\"R=%0d\", RPS); $finish; end\n\
               endmodule\n";
    for extra in [
        vec!["-G", "NOPE=1"],
        vec!["-G", "RPS=zzz"],
        vec!["-G", "RPS"],
    ] {
        let (o, e, c) = run_args(src, &extra);
        assert_ne!(c, Some(0), "{extra:?} must be loud:\n{o}{e}");
    }
}
