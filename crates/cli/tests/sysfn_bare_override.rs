//! A BARE integer-returning system function used as a parameter OVERRIDE is
//! type-determined: 32 bits signed, not the child default literal's width
//! (ROADMAP §5.2 row 1, §2 "Index sealing").
//!
//! ## What was wrong
//!
//! `Elaborator::override_self_meta` (`elaborate/src/param_query.rs`) gated its
//! answer on `sized_by_operator`, a `matches!` that listed only unary `+ - ~`,
//! any binary and the ternary. A BARE `$bits(x)` top failed that single conjunct
//! and the whole meta channel answered `None`, so `bind_one_param` fell through to
//! `param_decl_width_opt`'s DEFAULT-literal arm and bound the CHILD's own
//! initializer width:
//!
//! ```text
//! module leaf #(parameter P = 1'b0) ();
//!   initial $display("RES bits=%0d val=%0h", $bits(P), P);
//! endmodule
//! module top;
//!   logic [7:0] x;
//!   leaf #(.P($bits(x))) u1();      // vita: bits=1 val=0, both oracles: bits=32 val=8
//! endmodule
//! ```
//!
//! "binds 1 bit" is really "binds the DEFAULT literal's width" — with
//! `parameter P = 4'd0` the same override bound 4 (`d_default_literal_width_*`).
//!
//! The class was measured over 52 designs: 17 silent-wrong, and the wrong width
//! reached all FOUR override channels (named, positional, `defparam`, interface
//! instance), CASCADED through a forwarding chain (`g_forwarding_chain_*`) and
//! survived parenthesisation (`e_parenthesised_*`), because the gate peels
//! `Paren` before testing the top.
//!
//! ## What decides which spellings moved
//!
//! The ARGUMENT, not the function name. `bind_one_param` consults the WIDE channel
//! first (`self_meta_binds` requires `ovr_bits.is_none()`), and `override_bits`'
//! `$bits` arm needs its argument folded in the BIT domain, which answers only for
//! parameters / package constants / types. So `$bits(<param>)`, `$bits(<type>)`,
//! `$clog2(<param>)` never reached the broken rung and were already correct
//! (`h_wide_channel_*` controls); only a call over a DATA object did.
//!
//! ## The fix
//!
//! One arm on that `matches!` (renamed `sized_by_type`):
//! `ast::ExprKind::SysCall { name, .. } if sys_fn_is_integer(&name.name)`. It is
//! `param_decl_width_opt`'s arm (`params.rs`, the DECLARATION lane) one lane over;
//! everything downstream was already ready — `ctx_width_names_are_evident` has the
//! same named arm, `const_self_width` answers 32 and `const_signed_env` answers
//! signed.
//!
//! `sys_fn_is_integer` is the NAMED list `$clog2 | $bits | $rtoi`, deliberately not
//! widened here: the `$size`/`$high`/`$low`/`$left`/`$right`/`$increment` family and
//! a call whose VALUE the const domain cannot fold (`{$bits(x)}`, `$bits(x[3])`)
//! stay LOUD — their own loud→value row — which `i_*` pins.
//!
//! ## Oracles
//!
//! Every `RES` line below is printed verbatim by BOTH iverilog 13 (`-g2012` + `vvp
//! -n`) and verilator 5.052 (`--binary --timing`). The oracles did not split on any
//! cell pinned here.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sfbo_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// A clean run (exit 0) whose output contains `want`, verbatim as both oracles print it.
fn res(src: &str, want: &str) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    assert!(o.contains(want), "expected {want:?} in:\n{o}");
}

/// A LOUD run (exit 1) carrying the given diagnostic CODE.
///
/// Pinned on the code, never on the message TEXT.
fn loud_code(src: &str, code_str: &str) {
    let (o, code) = run(src);
    assert_eq!(code, Some(1), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
}

/// `module leaf #(parameter P = <default>)` printing `$bits(P)` and `P`, plus a
/// `top` whose body is `body`.
fn design(default: &str, body: &str) -> String {
    format!(
        "module leaf #(parameter {default}) ();\n  initial $display(\"RES bits=%0d val=%0h\", \
         $bits(P), P);\nendmodule\nmodule top;\n{body}\n  initial #1 $finish;\nendmodule\n"
    )
}

/// The named-connection cell over a packed vector — the class's canonical shape.
#[test]
fn a_bare_bits_of_a_vector_binds_32() {
    res(
        &design("P = 1'b0", "  logic [7:0] x;\n  leaf #(.P($bits(x))) u1();"),
        "RES bits=32 val=8",
    );
}

/// A 1-bit operand: the VALUE was right by accident (1 fits in 1 bit), the width was not.
#[test]
fn a_bare_bits_of_a_one_bit_variable_binds_32() {
    res(
        &design("P = 1'b0", "  logic x;\n  leaf #(.P($bits(x))) u1();"),
        "RES bits=32 val=1",
    );
}

/// `integer` — a 32-bit 4-state variable, whose `$bits` is 32 (`20` hex).
#[test]
fn a_bare_bits_of_an_integer_binds_32() {
    res(
        &design("P = 1'b0", "  integer x;\n  leaf #(.P($bits(x))) u1();"),
        "RES bits=32 val=20",
    );
}

/// `int` — the 2-state twin of the cell above.
#[test]
fn a_bare_bits_of_an_int_binds_32() {
    res(
        &design("P = 1'b0", "  int x;\n  leaf #(.P($bits(x))) u1();"),
        "RES bits=32 val=20",
    );
}

/// A NET, not a variable: the operand kind is not what the wide channel declines on.
#[test]
fn a_bare_bits_of_a_wire_binds_32() {
    res(
        &design("P = 1'b0", "  wire [4:0] w;\n  leaf #(.P($bits(w))) u1();"),
        "RES bits=32 val=5",
    );
}

/// A typedef'd VARIABLE (not the TYPE — that one already bound through the wide channel).
#[test]
fn b_bare_bits_of_a_typedef_variable_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  typedef logic [11:0] t_t; t_t x;\n  leaf #(.P($bits(x))) u1();",
        ),
        "RES bits=32 val=c",
    );
}

/// A packed-struct variable: 4 + 3 = 7 bits.
#[test]
fn b_bare_bits_of_a_packed_struct_variable_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  typedef struct packed {logic [3:0] a; logic [2:0] b;} s_t; s_t x;\n  \
             leaf #(.P($bits(x))) u1();",
        ),
        "RES bits=32 val=7",
    );
}

/// A CONCAT argument — the call itself is still bare, so this is the fixed class,
/// unlike `{$bits(x)}` (a concat AROUND the call), which stays loud below.
#[test]
fn b_bare_bits_of_a_concat_argument_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf #(.P($bits({x,x}))) u1();",
        ),
        "RES bits=32 val=10",
    );
}

/// `$clog2` over a non-foldable argument. The same spelling over a PARAMETER was
/// always correct (wide channel) — the discriminator is the argument.
#[test]
fn c_bare_clog2_over_a_variable_width_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf #(.P($clog2($bits(x)))) u1();",
        ),
        "RES bits=32 val=3",
    );
}

/// `$rtoi` — the third name in `sys_fn_is_integer`. Its argument is a REAL literal,
/// so `override_self_value`'s re-fold at `(32, true)` is what must reproduce `2`
/// (both oracles truncate 2.9 toward zero).
#[test]
fn c_bare_rtoi_of_a_real_literal_binds_32_and_truncates() {
    res(
        &design("P = 1'b0", "  leaf #(.P($rtoi(2.9))) u1();"),
        "RES bits=32 val=2",
    );
}

/// The child default is `4'd0`, so the pre-fix answer was 4, not 1: the broken
/// fallback bound the DEFAULT LITERAL's width, whatever it was.
#[test]
fn d_default_literal_width_four_is_also_replaced_by_32() {
    res(
        &design("P = 4'd0", "  logic [7:0] x;\n  leaf #(.P($bits(x))) u1();"),
        "RES bits=32 val=8",
    );
}

/// `override_self_meta` peels `Paren` before testing the top, so parenthesising the
/// bare call did NOT rescue it — the operator-wrapped twin was fine only for a REAL
/// operator.
#[test]
fn e_parenthesised_bare_call_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf #(.P((($bits(x))))) u1();",
        ),
        "RES bits=32 val=8",
    );
}

/// Channel 2 of 4: positional override.
#[test]
fn f_positional_override_binds_32() {
    res(
        &design("P = 1'b0", "  logic [7:0] x;\n  leaf #($bits(x)) u1();"),
        "RES bits=32 val=8",
    );
}

/// Channel 3 of 4: `defparam`.
#[test]
fn f_defparam_override_binds_32() {
    res(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf u1();\n  defparam u1.P = $bits(x);",
        ),
        "RES bits=32 val=8",
    );
}

/// Channel 4 of 4: an INTERFACE instance override (`iface_inst.rs`, its own call site).
#[test]
fn f_interface_instance_override_binds_32() {
    res(
        "interface ifc #(parameter P = 1'b0) ();\n  initial $display(\"RES bits=%0d val=%0h\", \
         $bits(P), P);\nendinterface\nmodule top;\n  logic [7:0] x;\n  \
         ifc #(.P($bits(x))) u1();\n  initial #1 $finish;\nendmodule\n",
        "RES bits=32 val=8",
    );
}

/// The wrong width CASCADED: the grandchild's `P` is bound from the middle module's
/// already-mis-sized `Q`, so the whole forwarding chain answers 1 bit pre-fix.
#[test]
fn g_forwarding_chain_carries_the_width_to_the_grandchild() {
    res(
        "module leaf #(parameter P = 1'b0) ();\n  initial $display(\"RES bits=%0d val=%0h\", \
         $bits(P), P);\nendmodule\nmodule mid #(parameter Q = 1'b0)();\n  \
         leaf #(.P(Q)) u2();\nendmodule\nmodule top;\n  logic [7:0] x;\n  \
         mid #(.Q($bits(x))) u1();\n  initial #1 $finish;\nendmodule\n",
        "RES bits=32 val=8",
    );
}

/// CONTROL — the wide channel. `$bits` of a PARAMETER folds in the bit domain, so
/// `ovr_bits` is `Some` and `self_meta_binds` is false: this cell never touched the
/// fixed rung and must stay byte-identical.
#[test]
fn h_wide_channel_bits_of_a_parameter_is_unchanged() {
    res(
        &design(
            "P = 1'b0",
            "  parameter [7:0] xp = 8'd3;\n  leaf #(.P($bits(xp))) u1();",
        ),
        "RES bits=32 val=8",
    );
}

/// CONTROL — a DECLARED-type target. §6.20.2 keeps the declaration's type through
/// the override, and `self_meta_binds` requires `ParamType::Implicit`, so `int P`
/// is answered by the declaration, not by this predicate.
#[test]
fn h_declared_type_target_is_unchanged() {
    res(
        "module leaf #(parameter int P = 0) ();\n  initial $display(\"RES bits=%0d val=%0h\", \
         $bits(P), P);\nendmodule\nmodule top;\n  logic [7:0] x;\n  \
         leaf #(.P($bits(x))) u1();\n  initial #1 $finish;\nendmodule\n",
        "RES bits=32 val=8",
    );
}

/// CONTROL — a DECLARED RANGE target keeps its 4 bits (the value truncates to 8,
/// as both oracles print).
#[test]
fn h_declared_range_target_is_unchanged() {
    res(
        &design(
            "[3:0] P = 0",
            "  logic [7:0] x;\n  leaf #(.P($bits(x))) u1();",
        ),
        "RES bits=4 val=8",
    );
}

/// CONTROL — a concat AROUND the call. The VALUE half (`const_eval_in_scope`)
/// cannot fold it, so it stays loud; the width fix does not manufacture a value.
#[test]
fn i_concat_around_the_call_stays_loud() {
    loud_code(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf #(.P({$bits(x)})) u1();",
        ),
        "VITA-E3009",
    );
}

/// CONTROL — the `$size` dimension-query family is deliberately outside
/// `sys_fn_is_integer` and stays loud (its own loud→value row).
#[test]
fn i_size_dimension_query_stays_loud() {
    loud_code(
        &design(
            "P = 1'b0",
            "  logic [7:0] a [0:5];\n  leaf #(.P($size(a))) u1();",
        ),
        "VITA-E3009",
    );
}

/// CONTROL — `$bits` of a SELECT: in the named list, but the value does not fold,
/// so it stays loud rather than silently binding a width with no value.
#[test]
fn i_bits_of_a_bit_select_stays_loud() {
    loud_code(
        &design(
            "P = 1'b0",
            "  logic [7:0] x;\n  leaf #(.P($bits(x[3]))) u1();",
        ),
        "VITA-E3009",
    );
}
