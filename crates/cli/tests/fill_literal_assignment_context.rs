//! The fill literal (`'0`/`'1`/`'x`/`'z`) takes its ASSIGNMENT'S width — at every
//! assignment form, not just the ones that happened to ask (§4.5.353).
//!
//! IEEE 1800 §5.7.1: an unsized unbased literal fills every bit of its context.
//! §11.6 makes the RHS of an assignment context-determined at the lvalue's width.
//! `Elaborator::resize_fill_rhs` implements exactly that and was already called from
//! the user `assign`, blocking-assign and nonblocking-assign lowerings. THREE
//! assignment forms never called it, lowered the fill self-determined at 1 bit, and
//! then zero-extended the result:
//!
//! - a **net declaration initializer** (`wire [7:0] a = '1;`) — which IS an implicit
//!   continuous assign, so it disagreed with the `assign a = '1;` written beside it;
//! - **`force`** (`force a = '1;`);
//! - the **procedural continuous assign** (`assign a = '1;` inside a process), which
//!   lowers to `Stmt::Force` — an implementation choice that must not change widths.
//!
//! Each read `00000001` at exit 0 where BOTH oracles read `11111111`. A 3-oracle census
//! put the boundary here: the `assign`/`reg`-decl-init/NBA/task-output/port-connect/
//! case-label/assignment-pattern siblings were already correct, so the fix is the same
//! one call at three more places rather than a fourth spelling of the rule.
//!
//! ⚠️ THE OTHER HALF OF THE RULE IS PINNED TOO. A fill in a SELF-determined position
//! must stay 1 bit — a shift amount, a `**` exponent, a `&&`/`||` operand, a bit index.
//! Widening those would trade one silent-wrong for another (`8'd1 << '1` would become
//! `1 << 255` = 0), which this project forbids outright. Those cases are here so a
//! future simplification that pushes context everywhere fails HERE.
//!
//! ORACLES: iverilog 13.0 (every value pinned) + verilator 5.050 (agrees except on
//! `'x`/`'z`, where it is 2-state and prints 0 — there iverilog and §5.7.1 are the
//! authority).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_flac_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "expected success:\n{all}");
    let mut kept = String::new();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        if !l.starts_with("simulation ended") {
            kept.push_str(l);
            kept.push('\n');
        }
    }
    kept
}

// ───────────────────────── (a) net declaration initializer ─────────────────────────

#[test]
fn net_decl_init_fill_takes_the_net_width() {
    // Was: a=00000001 while the `assign` and `reg` spellings of the same value were
    // already 11111111 — the asymmetry that located the site.
    let o = run("module t;\n\
         wire [7:0] a = '1;\n\
         wire [7:0] c; assign c = '1;\n\
         reg  [7:0] b = '1;\n\
         initial begin #1 $display(\"%b %b %b\", a, c, b); end\n\
         endmodule\n");
    assert_eq!(o, "11111111 11111111 11111111\n");
}

#[test]
fn net_decl_init_fill_spans_every_width_and_all_four_fills() {
    // 1, 64, 65 and 128 bits — across the one-word boundary in both directions — plus
    // a signed net, a multi-dimensional packed net, a multi-declarator, and 'x/'z.
    let o = run("module t;\n\
         wire [0:0]   w1  = '1;\n\
         wire [63:0]  w64 = '1;\n\
         wire [64:0]  w65 = '1;\n\
         wire [127:0] w128= '1;\n\
         wire signed [7:0] ws = '1;\n\
         wire [1:0][3:0] md = '1;\n\
         wire [7:0] p = '1, q = '0;\n\
         wire [7:0] xf = 'x;\n\
         wire [7:0] zf = 'z;\n\
         initial begin #1\n\
           $display(\"%b %h %h %h\", w1, w64, w65, w128);\n\
           $display(\"%0d %b %b %b %b %b\", ws, md, p, q, xf, zf);\n\
         end\n\
         endmodule\n");
    // vvp, both lines. `ws` is -1 because a signed net's all-ones IS -1 — the fill has
    // to reach the sign bit for that, which is the whole point.
    assert_eq!(
        o,
        "1 ffffffffffffffff 1ffffffffffffffff ffffffffffffffffffffffffffffffff\n\
         -1 11111111 11111111 00000000 xxxxxxxx zzzzzzzz\n"
    );
}

#[test]
fn net_decl_init_fill_covers_every_net_kind_and_the_delay_form() {
    // The initializer is an implicit continuous assign for `tri`/`wand`/`wor` too, and
    // the net-declaration delay (`wire [7:0] #0 x = …`) takes the same path.
    let o = run("module t;\n\
         tri  [7:0] tn = '1;\n\
         wand [7:0] wa = '1;\n\
         wor  [7:0] wo = '1;\n\
         wire [7:0] #0 dl = '1;\n\
         initial begin #1 $display(\"%b %b %b %b\", tn, wa, wo, dl); end\n\
         endmodule\n");
    assert_eq!(o, "11111111 11111111 11111111 11111111\n");
}

#[test]
fn net_decl_init_fill_works_in_every_scope_including_a_parameterised_width() {
    // Interface scope, an instantiated child whose net width comes from a PARAMETER
    // (12 bits -> fff, so the width really is resolved before the fill is sized),
    // generate-if and generate-for.
    let o = run("interface intf; wire [7:0] s = '1; endinterface\n\
         module child #(parameter W = 12) (output wire [W-1:0] o);\n\
           wire [W-1:0] a = '1; assign o = a;\n\
         endmodule\n\
         module t;\n\
           intf ii();\n\
           wire [11:0] co; child c(.o(co));\n\
           generate if (1) begin : g wire [7:0] gw = '1; end endgenerate\n\
           genvar i; generate for (i=0;i<2;i=i+1) begin : gf wire [3:0] fw = '1; end endgenerate\n\
           initial begin #1 $display(\"%b %h %b %b %b\", ii.s, co, g.gw, gf[0].fw, gf[1].fw); end\n\
         endmodule\n");
    assert_eq!(o, "11111111 fff 11111111 1111 1111\n");
}

// ─────────────── (b) force and (c) the procedural continuous assign ───────────────

#[test]
fn force_and_procedural_assign_fills_take_the_target_width() {
    // Both lower to `Stmt::Force`; neither asked for the assignment context. The
    // release/deassign halves are here so the pair is exercised, not just the write.
    let o = run("module t;\n\
         reg [7:0] r; wire [7:0] n; assign n = 8'h00; reg [7:0] pa;\n\
         initial begin\n\
           r = 8'h0F; force r = '1; #1 $display(\"%b\", r);\n\
           release r; #1 $display(\"%b\", r);\n\
           force n = '1; #1 $display(\"%b\", n);\n\
           assign pa = '1; #1 $display(\"%b\", pa);\n\
           deassign pa; pa = 8'h3C; #1 $display(\"%b\", pa);\n\
         end\n\
         endmodule\n");
    // vvp, all five lines. Line 2 stays all-ones because releasing a forced VARIABLE
    // leaves the last forced value in place (IEEE 1364 §9.3.2).
    assert_eq!(o, "11111111\n11111111\n11111111\n11111111\n00111100\n");
}

// ───────────── the other half: a fill in a SELF-determined position stays 1 bit ─────────────

#[test]
fn a_fill_in_a_self_determined_position_must_not_widen() {
    // Shift amount, `**` exponent, `&&`/`||` operands and a bit index are
    // self-determined (IEEE Table 11-21). If the fix leaked the assignment context into
    // them, `8'd1 << '1` would be `1 << 255` = 0 rather than 2 — a NEW silent-wrong
    // traded for the old one, which the accuracy ladder forbids.
    let o = run("module t;\n\
         reg [7:0] m;\n\
         wire [7:0] sh = 8'd1 << '1;\n\
         wire [7:0] s0 = 8'd1 << '0;\n\
         wire [7:0] pw = 8'd2 ** '1;\n\
         wire [7:0] la = '1 && '1;\n\
         wire [7:0] lo = '0 || '0;\n\
         wire       ix = m['1];\n\
         wire [7:0] rc = {8{'1}};\n\
         wire [7:0] cm = ('1 == 1'b1) ? 8'hAA : 8'h55;\n\
         initial begin m = 8'b0000_0010; #1\n\
           $display(\"%0d %0d %0d %0d %0d %b %b %h\", sh, s0, pw, la, lo, ix, rc, cm);\n\
         end\n\
         endmodule\n");
    assert_eq!(o, "2 1 2 1 0 1 11111111 aa\n");
}

#[test]
fn the_same_self_determined_rule_holds_at_the_three_new_sites() {
    // The shift-amount case again, but reached through `force` and through the
    // procedural continuous assign — the two sites that newly call the context lowering.
    let o = run("module t;\n\
         reg [7:0] a, b;\n\
         initial begin\n\
           force a = 8'd1 << '1; #1 $display(\"%0d\", a);\n\
           assign b = 8'd1 << '1; #1 $display(\"%0d\", b);\n\
         end\n\
         endmodule\n");
    assert_eq!(o, "2\n2\n");
}

// ───────────── the siblings that were already right and must stay byte-identical ─────────────

#[test]
fn the_already_correct_assignment_forms_are_unchanged() {
    // Every one of these was correct BEFORE the fix. They are pinned because the fix
    // adds a call that re-lowers the RHS: a form that started routing through the
    // context lowering when it did not before would show up here.
    let o = run("module t;\n\
         reg [7:0] nb, tk, ae [0:1]; reg [15:0] ps; reg [7:0] cx, cy;\n\
         function [7:0] f(input [7:0] x); f = x; endfunction\n\
         task tsk(output [7:0] o); o = '1; endtask\n\
         wire [7:0] fa; assign fa = f('1);\n\
         initial begin\n\
           nb <= '1; ps = 0; ps[11:4] = '1; ae[0] = '1; ae[1] = '0;\n\
           tsk(tk); {cx, cy} = '1;\n\
           #1 $display(\"%b %h %b %b %b %b %b %b\", nb, ps, ae[0], ae[1], tk, cx, cy, fa);\n\
         end\n\
         endmodule\n");
    assert_eq!(
        o,
        "11111111 0ff0 11111111 00000000 11111111 11111111 11111111 11111111\n"
    );
}

// ───────────── a `real` target has no bit context (adversarial review, BLOCKING) ─────────────

#[test]
fn a_fill_assigned_to_a_real_stays_one_bit() {
    // ⚠️ THE FIRST DRAFT OF THIS SLICE REGRESSED THIS. `resize_fill_rhs` sizes the fill
    // with `ir_lvalue_width`, which answers 64 for a real net — its STORAGE width, not
    // an assignment bit-context. A real has no width to propagate (IEEE 1800 §6.12,
    // §11.6), so `'1` is its own 1-bit value converted to real: 1.0, per both oracles.
    // Taking 64 instead makes it 2^64-1 → 1.84467e+19.
    //
    // Adding the call at `force`/procedural-`assign` dragged their (correct) real
    // targets in with it — correct → silent-wrong, which the accuracy ladder forbids.
    // The guard therefore lives INSIDE `resize_fill_rhs`, which also repairs the FOUR
    // pre-existing spellings below (blocking, decl-init, array element) that were
    // already 1.84467e+19 before this slice.
    let o = run("module t;\n\
         real ra, rb, rc, rd, dinit = '1; real arr [0:1];\n\
         initial begin\n\
           assign ra = '1; rb = '1; force rc = '1; rd = '0; arr[0] = '1;\n\
           #1 $display(\"%g %g %g %g %g %g\", ra, rb, rc, rd, dinit, arr[0]);\n\
         end\n\
         endmodule\n");
    // vvp: all ones-valued reals are 1, the '0 is 0.
    assert_eq!(o, "1 1 1 0 1 1\n");
}

// ───────────── the short-circuit must SEE the fill (adversarial review, MAJOR) ─────────────

#[test]
fn a_fill_inside_a_min_typ_max_is_still_a_fill() {
    // `lower_expr` lowers `(min:typ:max)` as a transparent pass-through to `typ`, so the
    // assignment context does reach the chosen branch — but `expr_contains_fill` did not
    // look inside, and that walk is the gate on the whole context lowering. So
    // `(1:'1:2)` kept a 1-bit fill at EVERY assignment site, the pre-existing ones
    // included. Both halves are pinned: the net-decl init (this slice's site) and the
    // blocking assign (a site that was already calling the helper).
    let o = run("module t;\n\
         wire [7:0] mt = (1:'1:2);\n\
         reg  [7:0] rmt;\n\
         initial begin rmt = (1:'1:2); #1 $display(\"%b %b\", mt, rmt); end\n\
         endmodule\n");
    assert_eq!(o, "11111111 11111111\n");
}
