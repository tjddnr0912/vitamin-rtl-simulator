//! V34-3 — KEYED assignment patterns: `'{name: value, …}` (IEEE 1800 §10.9.2) for
//! a packed struct, and `'{default: value}` (§10.9.1) for a packed struct or a
//! fixed-size unpacked array.
//!
//! WHY the named form matters, in the reporter's words: a POSITIONAL pattern is
//! position-coupled, so inserting a field silently shifts every later value — and
//! that failure surfaces as a wrong hash, not an error. `'{mode: …, en: …}` makes
//! that class of failure structurally impossible, which is why the values here are
//! asserted with a REORDERED key list wherever it is meaningful.
//!
//! ORACLES. iverilog 13 is NOT an oracle on this axis: measured 2026-08-26 it
//! rejects every keyed pattern AND `'{4{9}}` with a bare "syntax error / Malformed
//! statement", in a procedural assignment and in a declaration initializer alike.
//! verilator 5.050 accepts all of them and agrees with §10.9. So every positive
//! cell below was measured against verilator plus a hand reading of the LRM, and
//! the shapes where verilator would be the SOLE authority for an ordering or
//! priority rule (integer keys, type keys) are left loud on purpose.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_kpat_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// `cfg_t` = 13 bits: `mode` [12:9], `en` [8], `len` [7:0].
const CFG: &str = "typedef struct packed { logic [3:0] mode; logic en; logic [7:0] len; } cfg_t;\n";

// ---------------------------------------------------------------- §10.9.2 named

#[test]
fn named_struct_pattern_procedural() {
    // The report's exact case. verilator: 0707 (mode=3<<9 | en=1<<8 | len=7).
    let (o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{mode: 4'h3, en: 1'b1, len: 8'd7}}; $display(\"%h\", c); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0707"), "got:\n{o}");
}

#[test]
fn named_struct_pattern_is_order_independent() {
    // The whole point of §10.9.2: the DECLARATION fixes the order, the pattern does
    // not. Same three values written back-to-front must produce the same 0707.
    let (o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{len: 8'd7, mode: 4'h3, en: 1'b1}}; $display(\"%h\", c); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0707"), "got:\n{o}");
}

#[test]
fn named_struct_pattern_in_a_task() {
    // Spelling (a) of the request: a procedural assignment filling a config struct
    // inside a testbench task, with a task-local struct variable.
    let (o, code) = run(&format!(
        "module top;\n{CFG}\
         task automatic fill(input int v);\n\
           cfg_t lc;\n\
           lc = '{{mode: 4'(v), en: 1'b1, len: 8'(v + 1)}};\n\
           $display(\"%h\", lc);\n\
         endtask\n\
         initial begin fill(4); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    // mode=4, en=1, len=5 → 4<<9 | 1<<8 | 5 = 0x905.
    assert!(o.contains("0905"), "got:\n{o}");
}

#[test]
fn named_struct_pattern_decl_init() {
    let (o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c = '{{len: 8'd9, mode: 4'h2, en: 1'b0}};\n\
         initial begin $display(\"%h\", c); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0409"), "got:\n{o}");
}

// -------------------------------------------------------------- §10.9.1 default

#[test]
fn default_fills_every_struct_member() {
    // `1'b1` widened into each member independently → mode=1, en=1, len=1 = 0x301.
    // NOT a 13-bit fill: verilator prints 0301, not 1fff.
    let (o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{default: 1'b1}}; $display(\"%h\", c); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0301"), "got:\n{o}");
}

#[test]
fn named_plus_default_mixed() {
    // §10.9.1: `default` covers every member not otherwise given. mode=5, rest 0.
    let (o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{mode: 4'h5, default: 1'b0}}; $display(\"%h\", c); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0a00"), "got:\n{o}");
}

#[test]
fn default_into_two_state_members_squashes_x() {
    // §6.11.3: a 2-state member cannot hold X. The `default:` value rides the SAME
    // `longint'()` squash the positional path already applies per field, so this is
    // 0000, not xxxx — verified against verilator.
    let (o, code) = run("module top;\n\
         typedef struct packed { bit [7:0] a; bit [7:0] b; } two_t;\n\
         two_t t;\n\
         initial begin t = '{default: 1'bx}; $display(\"%h\", t); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("0000"), "got:\n{o}");
}

#[test]
fn default_fills_unpacked_array_bounds_variants() {
    // Spelling (b): `'{default: v}` on a whole fixed-size unpacked array. All three
    // bound shapes the positional path already distinguishes — ascending, descending
    // and offset — must fill identically, because `default` has no position at all.
    let (o, code) = run("module top;\n\
         int a [0:3]; int d [3:0]; int off [2:5];\n\
         initial begin\n\
           a = '{default: 5}; d = '{default: 6}; off = '{default: 8};\n\
           $display(\"%0d%0d%0d%0d %0d%0d%0d%0d %0d%0d%0d%0d\",\n\
             a[0],a[1],a[2],a[3], d[0],d[1],d[2],d[3], off[2],off[3],off[4],off[5]);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("5555 6666 8888"), "got:\n{o}");
}

#[test]
fn default_fills_multidim_unpacked_array() {
    // A multi-dim target needs the NESTED shape `flatten_assign_pattern` validates,
    // so the expansion builds the nest rather than a flat list of six.
    let (o, code) = run("module top;\n\
         int m [0:1][0:2];\n\
         initial begin m = '{default: 9};\n\
           $display(\"%0d%0d%0d%0d%0d%0d\", m[0][0],m[0][1],m[0][2],m[1][0],m[1][1],m[1][2]);\n\
           $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("999999"), "got:\n{o}");
}

#[test]
fn default_unpacked_array_decl_init() {
    let (o, code) = run("module top; int a [0:2] = '{default: 7};\n\
         initial begin $display(\"%0d %0d %0d\", a[0],a[1],a[2]); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("7 7 7"), "got:\n{o}");
}

#[test]
fn named_pattern_rides_every_assignment_form() {
    // `maybe_struct_pattern_rhs` is the shared hook for every `lvalue = rhs` site,
    // so a keyed pattern reaches all of them at once: nonblocking, a 1-D
    // struct-array ELEMENT, `force`, and a continuous assign. Pinned because the
    // hook is easy to widen for one form and forget the others — all four print 12
    // under verilator 5.050.
    let (o, code) = run("module top;\n\
         typedef struct packed { logic [3:0] a; logic [3:0] b; } r_t;\n\
         r_t nb; r_t arr [0:1]; r_t fo; r_t ca;\n\
         assign ca = '{a: 4'h1, b: 4'h2};\n\
         initial begin\n\
           nb <= '{a: 4'h1, b: 4'h2};\n\
           arr[1] = '{a: 4'h1, b: 4'h2};\n\
           force fo = '{a: 4'h1, b: 4'h2};\n\
           #1 $display(\"%h %h %h %h\", nb, arr[1], fo, ca);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("12 12 12 12"), "got:\n{o}");
}

#[test]
fn named_pattern_on_a_packable_unpacked_record() {
    // An UNPACKED record with integral members has an on-demand packed layout
    // (`packable_record_layout`), and the keyed normalization runs off the same
    // `StructLayout::fields`, so it gets the named form for free. verilator: 1 2.
    let (o, code) = run("module top;\n\
         typedef struct { int a; int b; } u_t;\n\
         u_t u;\n\
         initial begin u = '{a: 1, b: 2}; $display(\"%0d %0d\", u.a, u.b); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("1 2"), "got:\n{o}");
}

#[test]
fn named_pattern_as_a_queue_push_actual() {
    // §7.10.2/§10.9.2: `q.push_back('{…})` is how a record is enqueued in a
    // transaction model, and the keyed form resolves against the RECEIVER's type.
    // Keys written out of declaration order, to prove the receiver decides.
    let (o, code) = run("module top;\n\
         typedef struct packed { logic [3:0] a; logic [3:0] b; } r_t;\n\
         r_t q[$];\n\
         initial begin q.push_back('{b: 4'h2, a: 4'h1}); $display(\"%h\", q[0]); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("12"), "got:\n{o}");
}

// ------------------------------------------------------------------- still loud

#[test]
fn unknown_member_key_is_loud() {
    let (_o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{mode: 4'h1, bogus: 1'b1, len: 8'd2}}; $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(code, Some(0), "an unknown member key must be loud");
}

#[test]
fn duplicate_member_key_is_loud() {
    let (_o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{mode: 4'h1, mode: 4'h2, en: 1, len: 3}}; $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(code, Some(0), "a duplicate member key must be loud");
}

#[test]
fn missing_member_without_default_is_loud() {
    // §10.9.1 requires every member to be covered. Filling the rest with 0 would be
    // a guess that reads as a plausible config — exactly the silent-wrong the named
    // form exists to prevent.
    let (_o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{mode: 4'h1}}; $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(code, Some(0), "an uncovered member must be loud");
}

#[test]
fn mixing_positional_and_keyed_is_loud() {
    let (_o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{4'h1, en: 1'b1, len: 8'd2}}; $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(
        code,
        Some(0),
        "a mixed positional/keyed pattern must be loud"
    );
}

#[test]
fn type_key_is_loud() {
    // §10.9.1 type keys are out of scope, and `int` is a keyword: the reject is
    // raised AT the key rather than by letting `expr(0)` run at `int`, which
    // collapses what was a five-diagnostic cascade for one mistake into one line.
    let (_o, code) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         initial begin c = '{{int: 0}}; $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(code, Some(0), "a type key must be loud");
}

#[test]
fn member_key_on_an_array_target_is_loud() {
    let (_o, code) = run("module top; int a [0:3];\n\
         initial begin a = '{mode: 1, default: 0}; $finish; end endmodule\n");
    assert_ne!(code, Some(0), "a member key on an array must be loud");
}

#[test]
fn call_bearing_default_is_loud() {
    // `default:`'s value is CLONED into every slot it fills, so a call there would
    // run once per member/element instead of once. §10.9.1 does not pin that count
    // and iverilog cannot be asked (it rejects the form), so both the struct and the
    // array path refuse rather than silently multiply a side effect.
    let (_o, code) = run("module top; int a [0:3];\n\
         function automatic int f(); return 3; endfunction\n\
         initial begin a = '{default: f()}; $finish; end endmodule\n");
    assert_ne!(code, Some(0), "a call-bearing array default must be loud");
    let (_o2, code2) = run(&format!(
        "module top;\n{CFG}  cfg_t c;\n\
         function automatic int f(); return 3; endfunction\n\
         initial begin c = '{{default: f()}}; $finish; end endmodule\n"
    ));
    assert_ne!(code2, Some(0), "a call-bearing struct default must be loud");
}

#[test]
fn ternary_element_still_parses_as_positional() {
    // The keyed-vs-positional decision is a two-token lookahead (`tok :`). A ternary
    // element must not trip it: in `'{a ? b : c}` the token after `a` is `?`, so the
    // colon is never at element start + 1. Guarding this because getting it wrong
    // would turn a working positional pattern into a parse error.
    let (o, code) = run("module top; int s = 1; int a [0:1];\n\
         initial begin a = '{s ? 11 : 22, s ? 33 : 44};\n\
           $display(\"%0d %0d\", a[0], a[1]); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "got:\n{o}");
    assert!(o.contains("11 33"), "got:\n{o}");
}
