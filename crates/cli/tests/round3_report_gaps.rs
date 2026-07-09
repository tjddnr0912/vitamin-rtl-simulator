//! Round-3 external-report gaps (single-file `ifdef repros → four fixes).
//!
//! From an external tester's round-3 report against 901f967. Each gap is one
//! IEEE-legal construct vita loud-rejected; the report's priority is G > D >
//! E1 > B. All four now end correct-or-loud:
//!
//! - **G** (elaborate): a `localparam` declared INSIDE a generate block (IEEE
//!   §27) whose value indexes an unpacked-array param (`R = ROT[g]`), plus a
//!   generate-if on it. Two prerequisites were missing: `const_eval_in_scope`
//!   had no element-select arm (so `ROT[g]` never folded — even at module
//!   scope), and a generate-scope `localparam` was blanket-rejected (E3009).
//!   Now the array element folds and the localparam registers in `self.params`.
//! - **D** (parser): a block-local `automatic <type> <name>;` lifetime override
//!   (IEEE §6.21) in a procedural block or a function/task body (was E2002
//!   "expected statement, found Automatic"). iverilog 13.0 also `sorry`s on the
//!   lifetime override, so the oracle is hand/equivalence (the value is
//!   lifetime-independent for an assign-before-use local).
//! - **E1** (parser): a PACKED-struct typedef as a module port, ANSI or
//!   non-ANSI (mirrors EXT2-C tf-ports — the port net is the struct's flat
//!   vector, `c.field` desugars to a part-select). iverilog-oracled.
//! - **B** (parser): a body-local `enum` typedef stays a v1 cut (LOUD — iverilog
//!   13.0 segfaults on it, so no oracle), but the reject is now CLEAN: the enum
//!   is consumed (emit-and-consume recovery) so the rest of the body, which
//!   references the type, does not cascade into spurious follow-on errors.
//!
//! Pure parser (D/E1/B) + elaborate (G); no AST field added, `.vu`/format
//! unchanged, IR-0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (stdout, stderr, success).
fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r3g_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

/// The payload of the first `R:`-prefixed stdout line.
fn rline(o: &str) -> String {
    o.lines()
        .find_map(|l| l.trim().strip_prefix("R:").map(str::to_owned))
        .unwrap_or_default()
}

// ─────────────────────────────── GAP G ───────────────────────────────

#[test]
fn gap_g_generate_scope_localparam_and_generate_if() {
    // The report's headline repro (Keccak rho/pi shape). Per-lane localparam
    // `R = ROT[g]` folds, the generate-if selects the rotate/pass arm, and the
    // rotations compose. iverilog (array-free equiv) oracle: total rotate
    // 0+1+3+5 = 9 ≡ 1 (mod 8) ⇒ o = rotl(i,1); rotl(8'hA5,1) = 8'h4B.
    let (o, _e, ok) = run("module m (input logic [7:0] i, output logic [7:0] o);\n\
           localparam int unsigned ROT [0:3] = '{0, 1, 3, 5};\n\
           logic [7:0] stage [0:4]; assign stage[0] = i; genvar g;\n\
           generate for (g = 0; g < 4; g++) begin : g_lane\n\
             localparam int unsigned R = ROT[g];\n\
             if (R == 0) begin : g_rot0 assign stage[g+1] = stage[g]; end\n\
             else begin : g_rotn assign stage[g+1] = (stage[g] << R) | (stage[g] >> (8 - R)); end\n\
           end endgenerate\n\
           assign o = stage[4]; endmodule\n\
         module tb; logic [7:0] i, o; m dut(.i(i), .o(o));\n\
           initial begin i = 8'hA5; #1 $display(\"R:%02x\", o); $finish; end endmodule");
    assert!(ok && rline(&o) == "4b", "gap-G rotl(A5,1)=4b:\n{o}");
}

#[test]
fn gap_g_array_param_element_folds_in_const_context() {
    // The foundational half of G, in isolation: an element read of an
    // unpacked-array param in a constant context (`localparam X = ROT[2]`) now
    // folds (was E3009 "not a foldable constant expression"). ROT[2] = 3.
    let (o, _e, ok) = run("module m (output logic [7:0] o);\n\
           localparam int unsigned ROT [0:3] = '{0, 1, 3, 5};\n\
           localparam int X = ROT[2];\n\
           assign o = X[7:0];\n\
           initial begin #1 $display(\"R:%0d\", o); $finish; end endmodule");
    assert!(ok && rline(&o) == "3", "gap-G ROT[2]=3:\n{o}");
}

#[test]
fn gap_g_array_param_oob_index_is_loud() {
    // Correct-or-loud: an out-of-range element read folds None → LOUD, never a
    // silent 0.
    let (_o, _e, ok) = run("module m (output logic [7:0] o);\n\
           localparam int unsigned ROT [0:3] = '{0, 1, 3, 5};\n\
           localparam int X = ROT[9];\n\
           assign o = X[7:0]; endmodule");
    assert!(!ok, "gap-G ROT[9] out-of-range must be loud");
}

#[test]
fn gap_g_descending_array_element_const_is_loud() {
    // Correct-or-loud boundary: only a 0-based ASCENDING array is captured (its
    // positional pattern maps element i → index i). A descending `[3:0]` array
    // is NOT captured, so `ROT[1]` in a const context stays LOUD rather than
    // folding to the wrong element.
    let (_o, _e, ok) = run("module m (output logic [7:0] o);\n\
           localparam int unsigned ROT [3:0] = '{0, 1, 3, 5};\n\
           localparam int X = ROT[1];\n\
           assign o = X[7:0]; endmodule");
    assert!(!ok, "gap-G descending-array const element must be loud");
}

#[test]
fn gap_g_narrow_element_type_truncates_like_runtime() {
    // Adversarial regression: a NARROW element type must truncate its init
    // literal in the const fold, matching BOTH iverilog and vita's own runtime
    // read. `bit[3:0] ROT='{20,17}` ⇒ ROT[0] = 20 & 0xF = 4 (NOT the raw 20).
    // Earlier the const fold stored the un-truncated literal (const=20 while the
    // runtime net read 4) — a silent self-inconsistency.
    let (o, _e, ok) = run(
        "module lane (output logic [31:0] cval, output logic [31:0] rval);\n\
           localparam bit [3:0] ROT[0:1] = '{20, 17};\n\
           genvar g; generate for (g=0; g<1; g=g+1) begin: gl\n\
             localparam int R = ROT[g]; assign cval = R; end endgenerate\n\
           assign rval = ROT[0]; endmodule\n\
         module top; logic [31:0] c, r; lane u(.cval(c), .rval(r));\n\
           initial begin #1 $display(\"R:%0d %0d\", c, r); $finish; end endmodule",
    );
    assert!(
        ok && rline(&o) == "4 4",
        "gap-G narrow-element const fold must truncate (4) and match runtime (4):\n{o}"
    );
}

// ─────────────────────────────── GAP D ───────────────────────────────

#[test]
fn gap_d_block_local_automatic_int_unsigned() {
    // `automatic int unsigned idx;` in an always_comb block (was E2002). The
    // local zero-extends `s`: s=4'd5 ⇒ y=8'h05.
    let (o, _e, ok) = run("module m (input logic [3:0] s, output logic [7:0] y);\n\
           always_comb begin automatic int unsigned idx; idx = s; y = idx[7:0]; end endmodule\n\
         module tb; logic [3:0] s; logic [7:0] y; m dut(.s(s), .y(y));\n\
           initial begin s = 4'd5; #1 $display(\"R:%02x\", y); $finish; end endmodule");
    assert!(ok && rline(&o) == "05", "gap-D block-local automatic:\n{o}");
}

#[test]
fn gap_d_function_body_automatic_int() {
    // The tf_body lifetime override now accepts `automatic int` too (not only the
    // old logic/reg/integer/real kind set). f(41) = 42.
    let (o, _e, ok) = run("module m (output logic [7:0] y);\n\
           function automatic logic [7:0] f(input logic [7:0] a);\n\
             automatic int unsigned t; t = a + 1; return t[7:0];\n\
           endfunction\n\
           assign y = f(8'd41);\n\
           initial begin #1 $display(\"R:%0d\", y); $finish; end endmodule");
    assert!(
        ok && rline(&o) == "42",
        "gap-D function-body automatic:\n{o}"
    );
}

#[test]
fn gap_d_read_before_write_automatic_is_loud() {
    // Correct-or-loud: v1 flattens a procedural block-local to a STATIC net.
    // An automatic accumulator that READS its per-entry-reset value before
    // writing (`acc = acc + i`) is NOT static-equivalent — vita cannot honor
    // the per-entry reset, so it is LOUD (was silently accumulating as static).
    let (_o, _e, ok) = run(
        "module top; integer i;\n\
           initial begin\n\
             for (i=0;i<4;i=i+1) begin automatic int acc; acc = acc + i; $display(\"a=%0d\", acc); end\n\
             $finish; end endmodule",
    );
    assert!(!ok, "read-before-write automatic block-local must be loud");
}

#[test]
fn gap_d_initializer_automatic_is_loud() {
    // Correct-or-loud: an `automatic` block-local WITH an initializer must re-run
    // the init on every entry, which a static flatten (init once at t0) cannot
    // do — LOUD.
    let (_o, _e, ok) = run(
        "module top; integer i;\n\
           initial begin\n\
             for (i=0;i<3;i=i+1) begin automatic integer x = 100; x=x+i; $display(\"x=%0d\", x); end\n\
             $finish; end endmodule",
    );
    assert!(!ok, "initializer automatic block-local must be loud");
}

#[test]
fn gap_d_hidden_read_in_with_clause_is_loud() {
    // Round-2 adversarial: a read of the automatic local hidden in a
    // `q.sum() with (item + t)` clause must still make it non-static-equivalent →
    // LOUD. The equivalence check is sound-by-construction (any expr/stmt form it
    // cannot fully vet blocks the accept), so a shared-walker blind spot
    // (ArrayMethodWith / timing / assign) cannot silently accept a read-before-write.
    let (_o, _e, ok) = run(
        "module top; int q[$]; int x;\n\
           initial begin q.push_back(10); q.push_back(20);\n\
             for (int i=0;i<3;i++) begin automatic int t; x = q.sum() with (item + t); t = 100; end\n\
             $finish; end endmodule",
    );
    assert!(
        !ok,
        "a read hidden in a with-clause must be loud, not silently static"
    );
}

// ─────────────────────────────── GAP E1 ──────────────────────────────

#[test]
fn gap_e1_ansi_packed_struct_port() {
    // A packed-struct ANSI module port. cfg_t{a[MSB], b[LSB]}; y = a & b.
    // i_cfg = 2'b11 ⇒ 1, 2'b10 ⇒ 0.
    let (o, _e, ok) = run(
        "package p; typedef struct packed { logic a; logic b; } cfg_t; endpackage\n\
         module m (input p::cfg_t i_cfg, output logic y); assign y = i_cfg.a & i_cfg.b; endmodule\n\
         module tb; logic [1:0] c; logic y; m d(.i_cfg(c), .y(y));\n\
           initial begin c = 2'b11; #1 $display(\"R:%b\", y); $finish; end endmodule",
    );
    assert!(ok && rline(&o) == "1", "gap-E1 ANSI struct port a&b:\n{o}");
}

#[test]
fn gap_e1_non_ansi_packed_struct_port() {
    // The non-ANSI port path (`input b_t pp;` in the body) also binds the layout.
    // b_t{hi[7:4], lo[3:0]}; s = hi ^ lo; pp=8'hA5 ⇒ A^5 = F.
    let (o, _e, ok) = run(
        "package p; typedef struct packed { logic [3:0] hi; logic [3:0] lo; } b_t; endpackage\n\
         module m(pp, s); import p::*; input b_t pp; output logic [3:0] s; assign s = pp.hi ^ pp.lo; endmodule\n\
         module tb; logic [7:0] c; logic [3:0] s; m d(.pp(c), .s(s));\n\
           initial begin c = 8'hA5; #1 $display(\"R:%0h\", s); $finish; end endmodule",
    );
    assert!(
        ok && rline(&o) == "f",
        "gap-E1 non-ANSI struct port hi^lo:\n{o}"
    );
}

// ─────────────────────────────── GAP B ───────────────────────────────

#[test]
fn gap_b_body_local_enum_is_loud_and_clean() {
    // A body-local enum typedef is a v1 cut → LOUD (iverilog 13.0 segfaults on it,
    // no oracle). The reject must be CLEAN: exactly one enum-reject diagnostic,
    // no cascade from the downstream `e v; v = e'(x);` that references the type.
    let (_o, e, ok) = run("package p;\n\
           function automatic logic [1:0] f(input logic [1:0] x);\n\
             typedef enum logic [1:0] { A = 2'd0, B = 2'd1 } e;\n\
             e v; v = e'(x); return logic'(v);\n\
           endfunction\n\
         endpackage\n\
         module m (input logic [1:0] s, output logic [1:0] y); assign y = s; endmodule");
    assert!(!ok, "body-local enum typedef must be loud");
    let enum_errs = e
        .lines()
        .filter(|l| l.contains("body-local enum typedef is unsupported"))
        .count();
    assert_eq!(enum_errs, 1, "exactly one clean enum-reject error:\n{e}");
    // And no follow-on "e v;" lvalue cascade (the type was consumed cleanly).
    assert!(
        !e.contains("after lvalue"),
        "the rejected enum must not cascade into an lvalue error:\n{e}"
    );
}

#[test]
fn gap_b_ok_enum_port_regression() {
    // Regression guard from the report: header-import + UNQUALIFIED package enum
    // typedef as a module port must PASS (it is not a struct/class cut).
    let (_o, _e, ok) = run(
        "package p; typedef enum logic [1:0] { A, B, C } mode_e; endpackage\n\
         module m import p::*; (input mode_e i_mode, output logic y); assign y = (i_mode == B); endmodule",
    );
    assert!(
        ok,
        "unqualified enum-typedef module port must remain supported"
    );
}
