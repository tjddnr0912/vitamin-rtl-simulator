//! `parameter type T = logic [1:0][3:0]` — a MULTI-DIMENSIONAL packed type
//! parameter (ROADMAP §5.2 row 1, §3.a ⑤). Every cell here was E2002 at parse
//! before this slice ("a … multi-dimensional type is unsupported in v1").
//!
//! The engine already ran every one of these shapes once they were on a
//! declaration — `a_control_explicit_multi_dim_packed_declaration_is_unchanged`
//! and its typedef twin are the same designs written out by hand. What was
//! missing is a CARRIER from the type parameter to the declaration, so this slice
//! is the packed mirror of the unpacked-extent carrier (§3 ⑤ⓕ): `T$w` keeps the
//! TOTAL packed width and two more synthesized value parameters per dimension
//! (`T$p<i>a` / `T$p<i>b`, the declared `[msb:lsb]` endpoints) carry the shape, so
//! an override replaces the extents and the element width together.
//!
//! The dimension COUNT is not carried and stays loud in both directions, exactly
//! as the unpacked half does: a count-losing override trips the group's `$fatal`
//! (`T$s` records both counts), a count-ADDING one trips E3002 on the `T$p…` half
//! the module never declared.
//!
//! Every value was measured on iverilog 13.0 AND verilator 5.052, except the
//! pass-through cell (iverilog rejects `.T(T)`) which is verilator's alone and the
//! loud cells, where the comment says what each oracle does instead.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tppk_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// The design's own `$display` lines, in the order the run printed them.
fn said(out: &str, tag: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.starts_with(tag))
        .map(|l| l.to_string())
        .collect()
}

/// The same lines as an order-free SET (a pass-through design prints from two
/// module levels and neither oracle fixes the interleaving).
fn said_sorted(out: &str, tag: &str) -> Vec<String> {
    let mut v = said(out, tag);
    v.sort();
    v
}

fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(said(&out, tag), want, "{out}");
}

fn is_loud(src: &str, needle: &str) -> String {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
    out
}

const L1: &str = "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  always_comb v = a;\n  initial begin\n    #1 $display(\"L1 v1=%h v0=%h bits=%0d bv=%0d s1=%0d s2=%0d hi=%h sel=%h\", v[1], v[0], $bits(T), $bits(v), $size(v,1), $size(v,2), v[1][3:2], v[0][1+:2]);\n  end\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n";
const L2: &str = "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L2 v1=%h v0=%h bits=%0d s1=%0d s2=%0d\", v[1], v[0], $bits(T), $size(v,1), $size(v,2));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m #(.T(logic [3:0][1:0])) u(.a(a)); initial #5 $finish; endmodule\n";
const L3: &str = "module m #(parameter type T = logic [1:0][3:0]) (input logic [23:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L3 v2=%h v0=%h bits=%0d s1=%0d s2=%0d\", v[2], v[0], $bits(T), $size(v,1), $size(v,2));\nendmodule\nmodule top; logic [23:0] a = 24'hA5C3F1; m #(.T(logic [2:0][7:0])) u(.a(a)); initial #5 $finish; endmodule\n";
const L4: &str = "typedef logic [1:0][3:0] a_t;\nmodule m #(parameter type T = a_t) (input logic [7:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L4 v1=%h v0=%h bits=%0d s1=%0d s2=%0d\", v[1], v[0], $bits(T), $size(v,1), $size(v,2));\nendmodule\ntypedef logic [3:0][1:0] b_t;\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); m #(.T(b_t)) u2(.a(a)); initial #5 $finish; endmodule\n";
const L5: &str = "module m #(parameter type T = logic [1:0][3:0]) (input T a, output T y);\n  function automatic logic [3:0] f(T x); return x[1] ^ x[0]; endfunction\n  assign y = {a[0], a[1]};\n  initial #1 $display(\"L5 f=%h y=%h a1=%h\", f(a), y, a[1]);\nendmodule\nmodule top; logic [7:0] a = 8'hA5; logic [7:0] y; m u(.a(a), .y(y)); initial #5 $finish; endmodule\n";
const L6: &str = "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v; T w;\n  logic [3:0] e;\n  always_comb begin v = T'(a + 8'd1); w = T'(a); e = w[1]; end\n  initial #1 $display(\"L6 v=%h e=%h bits=%0d left1=%0d right1=%0d left2=%0d dims=%0d\", v, e, $bits(T), $left(v,1), $right(v,1), $left(v,2), $dimensions(v));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); m #(.T(logic [0:3][1:0])) u2(.a(a)); initial #5 $finish; endmodule\n";
const L7: &str = "module m #(parameter type T = logic [0:1][3:0], parameter type S = logic signed [1:0][3:0], parameter type B = bit [1:0][3:0]) (input logic [7:0] a);\n  T v; S s; B b;\n  always_comb begin v = a; s = a; b = a; end\n  initial #1 $display(\"L7 v0=%h v1=%h s=%0d sneg=%0d b=%h bx=%h\", v[0], v[1], s, s < 0, b, b[1]);\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n";
const L8: &str = "module m #(parameter type T = logic [1:0][2:0][3:0]) (input logic [23:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L8 v1=%h v10=%h v01=%h bits=%0d s1=%0d s2=%0d s3=%0d\", v[1], v[1][0], v[0][1], $bits(T), $size(v,1), $size(v,2), $size(v,3));\nendmodule\nmodule top; logic [23:0] a = 24'hA5C3F1; m u(.a(a)); m #(.T(logic [2:0][1:0][3:0])) u2(.a(a)); initial #5 $finish; endmodule\n";
const L11: &str = "typedef logic [1:0][3:0] a_t [0:1];\nmodule m #(parameter type T = a_t) (input logic [7:0] a);\n  T v;\n  always_comb begin v[0] = a; v[1] = ~a; end\n  initial #1 $display(\"L11 v01=%h v10=%h bits=%0d\", v[0][1], v[1][0], $bits(T));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n";
const L13: &str = "module m #(parameter N = 2, parameter type T = logic [N-1:0][3:0]) (input logic [11:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L13 v0=%h bits=%0d s1=%0d\", v[0], $bits(T), $size(v,1));\nendmodule\nmodule top; logic [11:0] a = 12'hA5C; m u(.a(a)); m #(.N(3)) u2(.a(a)); initial #5 $finish; endmodule\n";
const L14: &str = "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  always_comb v = a;\n  initial #1 $display(\"L14 v1=%h s1=%0d\", v[1], $size(v,1));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m #(logic [3:0][1:0]) u2(.a(a)); initial #5 $finish; endmodule\n";
const L10: &str = "module n #(parameter type T = logic [7:0]) (input T a);\n  initial #1 $display(\"L10n a=%h bits=%0d\", a, $bits(T));\nendmodule\nmodule m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  always_comb v = a;\n  n #(.T(T)) c(.a(v));\n  initial #1 $display(\"L10m v1=%h bits=%0d\", v[1], $bits(T));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; m u(.a(a)); m #(.T(logic [3:0][1:0])) u2(.a(a)); initial #5 $finish; endmodule\n";
const CTRL1: &str = "module m #(parameter N = 2, parameter M = 4) (input logic [N*M-1:0] a, input logic [N-1:0][M-1:0] pa, output logic [N-1:0][M-1:0] py);\n  logic [N-1:0][M-1:0] v;\n  typedef logic [N-1:0][M-1:0] t_t;\n  t_t w;\n  function automatic logic [M-1:0] f(logic [N-1:0][M-1:0] x); return x[N-1] ^ x[0]; endfunction\n  always_comb begin v = a; w = a; v[0] = ~v[0]; py = {pa[0], pa[N-1]}; end\n  initial #1 $display(\"C1 v1=%h v0=%h w1=%h f=%h pa1=%h py=%h s1=%0d s2=%0d l1=%0d r2=%0d bits=%0d\", v[N-1], v[0], w[N-1], f(a), pa[N-1], py, $size(v,1), $size(v,2), $left(v,1), $right(v,2), $bits(t_t));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; logic [11:0] b = 12'hA5C; logic [7:0] y1; logic [11:0] y2;\n  m u(.a(a), .pa(a), .py(y1)); m #(.N(3), .M(4)) u2(.a(b), .pa(b), .py(y2)); initial #5 $finish; endmodule\n";
const CTRL2: &str = "package p; typedef logic [1:0][3:0] pt_t; endpackage\nmodule m #(parameter N = 2, parameter M = 4) (input logic [N*M-1:0] a);\n  typedef logic [N-1:0][M-1:0] t_t;\n  function automatic logic [M-1:0] f(t_t x); return x[N-1] ^ x[0]; endfunction\n  function automatic logic [M-1:0] g(p::pt_t x); return x[1]; endfunction\n  t_t w; p::pt_t q;\n  always_comb begin w = a; q = a; end\n  initial #1 $display(\"C2 f=%h g=%h w1=%h q0=%h bits=%0d qs=%0d\", f(a), g(a), w[N-1], q[0], $bits(t_t), $size(q,1));\nendmodule\nmodule n #(parameter N = 2, parameter M = 4) (input logic [N-1:0][M-1:0] pa);\n  typedef logic [N-1:0][M-1:0] t_t;\n  sub #(.N(N), .M(M)) s(.x(pa));\nendmodule\nmodule sub #(parameter N = 2, parameter M = 4) (input logic [N-1:0][M-1:0] x);\n  typedef logic [N-1:0][M-1:0] t_t;\n  t_t y; assign y = x;\n  initial #1 $display(\"C2s y1=%h s1=%0d\", y[N-1], $size(y,1));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; logic [11:0] b = 12'hA5C;\n  m u(.a(a)); m #(.N(3)) u2(.a(b)); n v(.pa(a)); n #(.N(3)) v2(.pa(b)); initial #5 $finish; endmodule\n";
const CTRL3: &str = "typedef logic [1:0][3:0] t_t;\nmodule sub (input t_t x, output t_t y);\n  function automatic logic [3:0] f(t_t z); return z[1]; endfunction\n  assign y = {x[0], x[1]};\n  initial #1 $display(\"C3 x1=%h y=%h f=%h s1=%0d\", x[1], y, f(x), $size(x,1));\nendmodule\nmodule top; logic [7:0] a = 8'hA5; logic [7:0] y; sub s(.x(a), .y(y)); initial #5 $finish; endmodule\n";

// ───────────────────────── the default and its overrides ─────────────────────────

#[test]
fn a_two_dim_packed_default_reads_its_elements_and_its_dimension_queries() {
    // `v[1]`, `v[0]`, a nested part-select and a nested indexed part-select, plus
    // `$bits` of the TYPE and of the variable and both `$size` axes.
    prints(
        L1,
        "L1 ",
        &["L1 v1=a v0=5 bits=8 bv=8 s1=2 s2=4 hi=2 sel=2"],
    );
}

#[test]
fn an_override_replaces_the_packed_extents_at_the_same_total_width() {
    // `.T(logic [3:0][1:0])` — 8 bits either way, four 2-bit elements instead of
    // two 4-bit ones, so `v[1]`/`v[0]` and both `$size` axes all move together.
    prints(L2, "L2 ", &["L2 v1=1 v0=1 bits=8 s1=4 s2=2"]);
}

#[test]
fn an_override_replaces_the_element_width_and_the_extents_together() {
    // `.T(logic [2:0][7:0])` — a wider element AND one more slot: 24 bits.
    prints(L3, "L3 ", &["L3 v2=a5 v0=f1 bits=24 s1=3 s2=8"]);
}

#[test]
fn a_multi_dim_packed_typedef_is_both_a_default_and_an_override() {
    // `parameter type T = a_t` where `typedef logic [1:0][3:0] a_t;`, overridden
    // per instance by a second typedef of a different shape.
    prints(
        L4,
        "L4 ",
        &[
            "L4 v1=a v0=5 bits=8 s1=2 s2=4",
            "L4 v1=1 v0=1 bits=8 s1=4 s2=2",
        ],
    );
}

#[test]
fn a_two_dim_packed_type_parameter_is_an_ansi_port_and_a_tf_formal() {
    // `input T a, output T y` and a function formal `T x` — the carrier reaches
    // the port binder (`try_port_typedef`) and the tf-port one, not only `T v;`.
    prints(L5, "L5 ", &["L5 f=f y=5a a1=a"]);
}

#[test]
fn a_cast_to_a_two_dim_packed_type_parameter_is_flat_and_the_queries_follow() {
    // `T'(e)` stays the FLAT size cast at `T$w` (both oracles: `T'(a+1)` is `a6`,
    // not a per-element operation), while `$left`/`$right`/`$dimensions` read the
    // carried extents — including the ASCENDING override `.T(logic [0:3][1:0])`.
    prints(
        L6,
        "L6 ",
        &[
            "L6 v=a6 e=a bits=8 left1=1 right1=0 left2=3 dims=2",
            "L6 v=a6 e=2 bits=8 left1=0 right1=3 left2=1 dims=2",
        ],
    );
}

#[test]
fn an_ascending_a_signed_and_a_two_state_packed_default_keep_their_shape() {
    // three type parameters in one module: `logic [0:1][3:0]` (ascending outer
    // dim), `logic signed [1:0][3:0]` (the whole vector is signed — 8'hA5 is −91)
    // and `bit [1:0][3:0]`.
    prints(L7, "L7 ", &["L7 v0=a v1=5 s=-91 sneg=1 b=a5 bx=a"]);
}

#[test]
fn a_three_dim_packed_type_parameter_carries_every_extent() {
    // three dimensions, and an override that permutes the outer two.
    prints(
        L8,
        "L8 ",
        &[
            "L8 v1=a5c v10=c v01=f bits=24 s1=2 s2=3 s3=4",
            "L8 v1=c3 v10=3 v01=f bits=24 s1=3 s2=2 s3=4",
        ],
    );
}

#[test]
fn a_typedef_with_packed_and_unpacked_dimensions_keeps_both_lists() {
    // `typedef logic [1:0][3:0] a_t [0:1];` — the packed dims ride `T$p…`, the
    // unpacked ones `T$d…`, and `$bits(T)` multiplies all three (8 × 2 = 16).
    prints(L11, "L11 ", &["L11 v01=a v10=a bits=16"]);
}

#[test]
fn a_packed_extent_that_names_a_header_parameter_follows_its_override() {
    // `parameter N = 2, parameter type T = logic [N-1:0][3:0]` under `#(.N(3))`:
    // the extent is an EXPRESSION elaborate folds per instance, so `$bits(T)` and
    // `$size(v,1)` both move.
    prints(
        L13,
        "L13 ",
        &["L13 v0=c bits=8 s1=2", "L13 v0=c bits=12 s1=3"],
    );
}

#[test]
fn a_positional_type_override_carries_the_packed_extents() {
    // `m #(logic [3:0][1:0])` — the `T$p…` slots are pushed in the same order the
    // group declares them.
    prints(L14, "L14 ", &["L14 v1=1 s1=4"]);
}

// ───────────────────────── writes, aliases, localparam ─────────────────────────

#[test]
fn a_write_to_a_whole_element_and_to_a_part_of_one_lands_per_instance() {
    // `v[1] = 4'h3; v[0][1:0] = 2'b11;` under the default and under an override
    // that makes every element 2 bits wide (both oracles `37` then `af`).
    prints(
        "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  \
         always_comb begin v = a; v[1] = 4'h3; v[0][1:0] = 2'b11; end\n  \
         initial #1 $display(\"L12 v=%h\", v);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); \
         m #(.T(logic [3:0][1:0])) u2(.a(a)); initial #5 $finish; endmodule\n",
        "L12 ",
        &["L12 v=37", "L12 v=af"],
    );
}

#[test]
fn a_part_select_wider_than_the_element_is_the_same_loud_it_is_when_written_out() {
    // ⚠️ NOT a type-parameter limit. `v[0][2:1]` on 2-bit elements is E3009 in vita
    // and a partial write in both oracles (`af`) — the EXPLICIT spelling below is
    // the control: same refusal, same message, no type parameter in sight.
    let a = is_loud(
        "module m #(parameter type T = logic [3:0][1:0]) (input logic [7:0] a);\n  T v;\n  \
         always_comb begin v = a; v[0][2:1] = 2'b11; end\n  \
         initial #1 $display(\"X v=%h\", v);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n",
        "part-select range exceeds the packed sub-element width",
    );
    let b = is_loud(
        "module m (input logic [7:0] a);\n  logic [3:0][1:0] v;\n  \
         always_comb begin v = a; v[0][2:1] = 2'b11; end\n  \
         initial #1 $display(\"X v=%h\", v);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n",
        "part-select range exceeds the packed sub-element width",
    );
    assert_eq!(
        a.contains("VITA-E3009"),
        b.contains("VITA-E3009"),
        "the type parameter must not change the verdict\n{a}\n{b}"
    );
}

#[test]
fn a_localparam_type_carries_its_packed_dimensions() {
    // `localparam type LT = logic [3:0][1:0];` — not overridable, so the extents
    // stay literal, and every consumer folds exactly as the explicit spelling does.
    prints(
        "module m (input logic [7:0] a);\n  localparam type LT = logic [3:0][1:0];\n  LT v;\n  \
         always_comb v = a;\n  \
         initial #1 $display(\"L9 v3=%h v0=%h bits=%0d\", v[3], v[0], $bits(LT));\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n",
        "L9 ",
        &["L9 v3=2 v0=1 bits=8"],
    );
}

#[test]
fn a_package_type_parameter_read_as_pkg_pt_is_still_loud() {
    // ⚠️ PRE-EXISTING and NOT a packed-dimension limit: a package `parameter type`
    // registers no `pkg::PT` twin, so `p::PT w;` is a parse cascade. Measured the
    // same for a ONE-dimensional `parameter type PT = logic [7:0]` (both oracles
    // run either spelling), so the gap is the package export, not this slice. The
    // rc is pinned, not the message — the text is a cascade, not a diagnosis.
    let (out, rc) = run(
        "package p; parameter type PT = logic [1:0][3:0]; endpackage\n\
         module m (input logic [7:0] a);\n  p::PT w;\n  always_comb w = a;\n  \
         initial #1 $display(\"L9 w1=%h\", w[1]);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n",
    );
    assert_ne!(rc, Some(0), "{out}");
    let (out1, rc1) = run("package p; parameter type PT = logic [7:0]; endpackage\n\
         module m (input logic [7:0] a);\n  p::PT w;\n  always_comb w = a;\n  \
         initial #1 $display(\"L9 w=%h\", w);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); initial #5 $finish; endmodule\n");
    assert_ne!(
        rc1,
        Some(0),
        "the ONE-dimensional twin is the control\n{out1}"
    );
}

#[test]
fn an_alias_type_parameter_inherits_the_packed_dimensions_and_follows_an_override() {
    // `localparam type U = T;` — `U` names `T`'s CARRIED dimensions, so an
    // override of `T` reaches `U v;` too (both oracles, both instances).
    prints(
        "module m #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  \
         localparam type U = T;\n  U v; T w;\n  \
         always_comb begin v = a; w = a; end\n  \
         initial #1 $display(\"AL v1=%h v0=%h ub=%0d s1=%0d s2=%0d w1=%h\", v[1], v[0], \
         $bits(U), $size(v,1), $size(v,2), w[1]);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m u(.a(a)); \
         m #(.T(logic [3:0][1:0])) u2(.a(a)); initial #5 $finish; endmodule\n",
        "AL ",
        &[
            "AL v1=a v0=5 ub=8 s1=2 s2=4 w1=a",
            "AL v1=1 v0=1 ub=8 s1=4 s2=2 w1=1",
        ],
    );
}

// ───────────────────── the dimension COUNT stays loud, both ways ─────────────────

#[test]
fn a_packed_count_adding_override_is_the_missing_carrier_reject() {
    // A 2-D override on a 1-D default: the `T$p0a` half names a parameter the
    // module never declared. E3002, reported against `T` and not the synthesized
    // carrier. Both oracles RUN this (`L15 v=a5`) — vita's declarators are stamped
    // with the default's dimension LIST at parse, so the count cannot be followed.
    let out = is_loud(
        "module m #(parameter type T = logic [7:0]) (input logic [7:0] a);\n  T v;\n  \
         always_comb v = a;\n  initial #1 $display(\"L15 v=%h\", v);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m #(.T(logic [1:0][3:0])) u(.a(a)); \
         initial #5 $finish; endmodule\n",
        "the override has more packed dimensions than the default",
    );
    assert!(out.contains("VITA-E3002"), "{out}");
    assert!(out.contains("type parameter `T`"), "{out}");
    assert!(
        !out.contains("$p0"),
        "the carrier name must not leak\n{out}"
    );
    assert_eq!(out.matches("VITA-E3002").count(), 1, "one report\n{out}");
}

#[test]
fn a_packed_count_losing_override_is_the_shape_guards_fatal() {
    // The mirror: a 1-D override on a 2-D default. `T$s` carries both dimension
    // counts above the sign / 2-state bits, so the narrowed compare still sees it.
    // Both oracles RUN this (`L15b v1=0`).
    let out = is_loud(
        "module m2 #(parameter type T = logic [1:0][3:0]) (input logic [7:0] a);\n  T v;\n  \
         always_comb v = a;\n  initial #1 $display(\"L15b v1=%h\", v[1]);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m2 #(.T(logic [7:0])) u2(.a(a)); \
         initial #5 $finish; endmodule\n",
        "type parameter `T`: the override changes the type's dimension COUNT (packed or unpacked)",
    );
    assert!(out.contains("fatal[VITA-F4004]"), "{out}");
    assert!(
        !out.contains("L15b v1="),
        "the wrong value must never print\n{out}"
    );
}

#[test]
fn a_positional_packed_count_adding_override_is_loud_without_naming_t() {
    // ⚠️ The positional channel carries no NAME, so the extra `T$p…` slots land on
    // the generic arity report rather than the named one. Measured identical for
    // the UNPACKED twin (`m #(a_t)` on a scalar default) before and after this
    // slice — the message is the pre-existing one, not a regression. Both oracles
    // run the design (`L15p v=a5`).
    let out = is_loud(
        "module m #(parameter type T = logic [7:0]) (input logic [7:0] a);\n  T v;\n  \
         always_comb v = a;\n  initial #1 $display(\"L15p v=%h\", v);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; m #(logic [1:0][3:0]) u(.a(a)); \
         initial #5 $finish; endmodule\n",
        "more positional parameter overrides than module parameters",
    );
    assert!(out.contains("VITA-E3002"), "{out}");
    assert!(!out.contains("L15p v="), "{out}");
}

#[test]
fn a_pass_through_of_a_two_dim_t_into_a_one_dim_child_is_loud() {
    // ⚠️ DIFFERS from verilator, which runs it (iverilog rejects `.T(T)` outright):
    // `L10m v1=a bits=8` / `L10n a=a5 bits=8`. The child's declarations of `T` were
    // stamped with ONE dimension at parse, so the outer type's two cannot be
    // followed — the same rule that makes an explicit count-changing override
    // loud. Honest-loud, once per instance, named against `T`.
    let out = is_loud(
        L10,
        "the override has more packed dimensions than the default",
    );
    assert!(out.contains("type parameter `T`"), "{out}");
    assert_eq!(
        out.matches("VITA-E3002").count(),
        2,
        "one per instance\n{out}"
    );
    assert!(!out.contains("L10n a="), "{out}");
}

#[test]
fn a_synthesized_packed_carrier_cannot_be_named_by_the_user() {
    // `.T$p0a(3)` — both oracles reject the spelling ("parameter `T$p0a` not found
    // in `top.u`"); so does vita, at the parse, as it already did for `T$d0a`.
    is_loud(
        "module m #(parameter type T = logic [1:0][3:0]) ();\n  T v;\n  \
         initial $display(\"CA %0d\", $bits(v));\nendmodule\n\
         module top; m #(.T$p0a(3)) u(); initial #5 $finish; endmodule\n",
        "is an internal type-parameter carrier, not a user parameter",
    );
}

// ───────────────── the positions with no slot for a dimension list ───────────────

#[test]
fn the_containers_that_cannot_hold_a_dimension_list_stay_loud() {
    // Four positions whose AST container carries ONE range and no packed list, so
    // a multi-dimensional `T` there would silently flatten to its outer dimension.
    // Each keeps its own diagnosis; none of them is silent.
    //
    // (a) a packed struct/union MEMBER of type `T` (iverilog runs it: `PS a=a5`).
    is_loud(
        "module m #(parameter type T = logic [1:0][3:0]) ();\n  \
         typedef struct packed { T a; logic [3:0] b; } s_t;\n  s_t s;\n  \
         initial begin s = 12'hA5C; #1 $display(\"PS a=%h\", s.a); end\nendmodule\n\
         module top; m u(); initial #5 $finish; endmodule\n",
        "a simple type for a struct/union member",
    );
    // (b) a function RETURN type (iverilog runs it: `FR f=a5`). `FunctionDef` is a
    // frozen SchemaHash type with one `range`; widening it is a format bump.
    is_loud(
        "module m #(parameter type T = logic [1:0][3:0]) ();\n  \
         function automatic T f(logic [7:0] x); return x; endfunction\n  \
         initial #1 $display(\"FR f=%h\", f(8'hA5));\nendmodule\n\
         module top; m u(); initial #5 $finish; endmodule\n",
        "as a function return type",
    );
    // (c) a NON-ANSI port (iverilog runs it: `NA x1=a`). `PortDecl` has no packed
    // list; the ANSI twin below is the supported spelling.
    is_loud(
        "typedef logic [1:0][3:0] t_t;\nmodule sub (x);\n  input t_t x;\n  \
         initial #1 $display(\"NA x1=%h\", x[1]);\nendmodule\n\
         module top; logic [7:0] a = 8'hA5; sub s(.x(a)); initial #5 $finish; endmodule\n",
        "as a non-ANSI port type is unsupported in v1",
    );
    // (d) an ENUM base — iverilog refuses this one too ("Enum type must not have
    // more than 1 packed dimension"), so vita and the oracle agree on the verdict.
    is_loud(
        "module m #(parameter type T = logic [1:0][3:0]) ();\n  \
         typedef enum T { A = 1, B = 2 } e_t;\n  e_t e;\n  \
         initial begin e = B; #1 $display(\"EB e=%h\", e); end\nendmodule\n\
         module top; m u(); initial #5 $finish; endmodule\n",
        "as an enum base",
    );
}

// ─────────────────────────────── the controls ───────────────────────────────────

#[test]
fn an_explicit_multi_dim_packed_typedef_is_an_ansi_port() {
    // c3: a plain `typedef logic [1:0][3:0] t_t;` as an ANSI port type — a
    // PRE-EXISTING gap this slice's port change closes (`try_port_typedef` used to
    // error on any typedef carrying packed dims). No type parameter involved.
    prints(CTRL3, "C3 ", &["C3 x1=a y=5a f=a s1=2"]);
}

#[test]
fn the_explicit_spellings_are_unchanged() {
    // c1 / c2: the same shapes written out by hand — variables, typedefs, package
    // typedefs, tf-port formals, ANSI ports with per-instance extents. These ran
    // identically BEFORE this slice; they are here so a later change to the carrier
    // cannot move them without a failure.
    prints(
        CTRL1,
        "C1 ",
        &[
            "C1 v1=a v0=a w1=a f=f pa1=a py=5a s1=2 s2=4 l1=1 r2=0 bits=8",
            "C1 v1=a v0=3 w1=a f=6 pa1=a py=0ca s1=3 s2=4 l1=2 r2=0 bits=12",
        ],
    );
    let (out, rc) = run(CTRL2);
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(
        said_sorted(&out, "C2"),
        [
            "C2 f=6 g=5 w1=a q0=c bits=12 qs=2",
            "C2 f=f g=a w1=a q0=5 bits=8 qs=2",
            "C2s y1=a s1=2",
            "C2s y1=a s1=3",
        ],
        "{out}"
    );
}

// ───────────────────── `defparam` cannot name a synthesized carrier ─────────────────────

/// `T$w` / `T$s` / `T$d<i>a` / `T$p<i>a` are built by the desugar, never written
/// by a user, and no legal elaboration of the module has a parameter by those
/// names — both oracles say so ("parameter `T$w` not found in `top.u`",
/// `%Error-PINNOTFOUND: Parameter not found: 'T$p0a'`). A NAMED `#(.T$w(…))`
/// override was already refused at the parser; `defparam` binds by NAME in
/// elaborate and reached the carriers untouched, so `defparam u.T$w = 16;`
/// silently reshaped the type parameter (measured on the 1-D `T$w` carrier
/// PRE-slice: `SUB bits=16 v=90`, exit 0) and `defparam u.T$p0a = 3;` silently
/// reshaped the packed EXTENTS (`SUB bits=16 v1=5 v0=10`, exit 0).
#[test]
fn a_defparam_naming_a_type_parameter_carrier_is_refused() {
    const NEEDLE: &str = "internal type-parameter carrier";
    let carriers = ["T$w", "T$s", "T$p0a", "T$p0b", "T$d0a"];
    for c in carriers {
        is_loud(
            &format!(
                "module sub #(parameter type T = logic [1:0][3:0]) (output logic [31:0] o);\n  T v;\n  initial begin v = 8'h5A; #1 $display(\"SUB bits=%0d v=%0d\", $bits(v), v); end\n  assign o = {{24'd0, v}};\nendmodule\nmodule top;\n  logic [31:0] o;\n  sub u(.o(o));\n  defparam u.{c} = 3;\n  initial #5 $finish;\nendmodule\n"
            ),
            NEEDLE,
        );
    }
    // the 1-D twin, which is where the hole was measured on PRE
    is_loud(
        "module sub #(parameter type T = logic [7:0]) (output logic [31:0] o);\n  T v;\n  initial begin v = 8'h5A; #1 $display(\"SUB bits=%0d v=%0d\", $bits(v), v); end\n  assign o = {24'd0, v};\nendmodule\nmodule top;\n  logic [31:0] o;\n  sub u(.o(o));\n  defparam u.T$w = 16;\n  initial #5 $finish;\nendmodule\n",
        NEEDLE,
    );
}

/// The control: an ordinary VALUE parameter beside a type parameter is still a
/// legal `defparam` target. 3-way identical — iverilog and verilator both accept
/// `defparam u.N = 3` and print the same two lines.
#[test]
fn a_defparam_of_a_value_parameter_beside_a_type_parameter_still_runs() {
    let (out, rc) = run(
        "module sub #(parameter type T = logic [1:0][3:0], parameter int N = 1) (output logic [31:0] o);\n  T v;\n  initial begin v = 8'h5A; #1 $display(\"SUB bits=%0d N=%0d\", $bits(v), N); end\n  assign o = {24'd0, v};\nendmodule\nmodule top;\n  logic [31:0] o;\n  sub u(.o(o));\n  defparam u.N = 3;\n  initial #2 $display(\"O=%h\", o);\n  initial #5 $finish;\nendmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
    assert_eq!(said(&out, "SUB "), ["SUB bits=8 N=3"], "{out}");
    assert_eq!(said(&out, "O="), ["O=0000005a"], "{out}");
}
