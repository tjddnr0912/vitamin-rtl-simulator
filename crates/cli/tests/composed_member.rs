//! `elem_t [hi:0] m` through the real lowering (`hdl-parser/src/structs/member_array.rs`).
//! Expectations are recorded oracle output on this natural text, named on each pin:
//! `vl` verilator, `s2` sv2v + iverilog, `ivh` iverilog on the hand guard, `pre` the hand
//! guard on the accepted binary of the base commit. iverilog refuses a run-time member
//! index, so it is no oracle on natural text. Where verilator's 2-state output stands
//! beside an x, the bits it knows are checked. The matrix is a hand model of IEEE 1800
//! §11.5.1 and §7.4.6: an index read in its own type, out of range reads x, an
//! out-of-range write has no effect. It mentions no guard.
//! Refusals: rc 1, no `A ` line, the class named.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_on(src: &str, backend: Option<&str>) -> (Vec<String>, i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cmem_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg(f.to_str().unwrap()).current_dir(&d);
    if let Some(b) = backend {
        c.args(["--backend", b]);
    }
    let out = c.output().expect("run vita");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines = stdout
        .lines()
        .filter(|l| l.starts_with("A "))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    std::fs::remove_dir_all(&d).ok();
    (lines, out.status.code().unwrap_or(-1), stdout + &stderr)
}

#[track_caller]
fn value(src: &str, expected: &[&str]) {
    let (got, rc, all) = run_on(src, None);
    assert_eq!(rc, 0, "{all}");
    assert_eq!(got, expected, "{all}");
}

#[track_caller]
fn loud(src: &str, code: &str) {
    let (got, rc, all) = run_on(src, None);
    assert_eq!(rc, 1, "{all}");
    assert!(got.is_empty(), "{all}");
    assert!(all.contains(code), "wants {code}: {all}");
}

const PKG: &str = "package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
  typedef e_t [3:0] vec_t;
  parameter vec_t D = 12'h5a3;
endpackage
";

/// The matrix package: a net narrower than the sentinel, and one wider than 2000 bits.
fn matrix_pkg(wide: bool) -> String {
    let pad = if wide { " logic [2047:0] pad;" } else { "" };
    format!(
        "package rp;
  typedef struct packed {{ logic [1:0] q; logic qe; }} m1_t;
  typedef struct packed {{ struct packed {{ logic [3:0] q; logic qe; }} a; struct packed {{ logic d; logic de; }} b; }} m2_t;
  typedef struct packed {{ logic q; }} b1_t;
  typedef struct packed {{ m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;{pad} }} r_t;
endpackage
"
    )
}

/// `(cell, index form, loop value, y, t)`: `s.m[e].q` read into `y`, `.q = 2'b11` into a
/// zeroed `t`, for one iteration of a loop of `int` (`d_int*`) or `int unsigned`.
const MATRIX: &[(&str, &str, &str, &str, &str)] = &[
    ("d_int", "i", "0", "01", "00180000000"),
    ("d_int", "i", "1", "01", "00c00000000"),
    ("d_int", "i", "3", "00", "30000000000"),
    ("d_int", "i", "-2147483647-1", "xx", "00000000000"),
    ("d_int", "i", "2147483647", "xx", "00000000000"),
    ("d_int", "2*i", "-2147483647-1", "01", "00180000000"),
    ("d_int", "2*i", "-2147483647", "00", "06000000000"),
    ("d_int", "2*i", "1", "00", "06000000000"),
    ("d_int", "2*i", "-1073741824", "xx", "00000000000"),
    ("d_int", "2*i", "2147483647", "xx", "00000000000"),
    ("d_int", "2*i+1", "-2147483647-1", "01", "00c00000000"),
    ("d_int", "2*i+1", "-2147483647", "00", "30000000000"),
    ("d_int", "2*i+1", "1", "00", "30000000000"),
    ("d_int", "2*i+1", "-1073741824", "xx", "00000000000"),
    ("d_int", "2*i+1", "2147483647", "xx", "00000000000"),
    ("d_int", "31-i", "-2147483647-1", "xx", "00000000000"),
    ("d_int", "31-i", "2147483647", "xx", "00000000000"),
    ("d_int", "i+8", "-2147483647-1", "xx", "00000000000"),
    ("d_int", "i+8", "2147483647", "xx", "00000000000"),
    ("d_uns", "i", "32'd0", "01", "00180000000"),
    ("d_uns", "i", "32'd1", "01", "00c00000000"),
    ("d_uns", "i", "32'd3", "00", "30000000000"),
    ("d_uns", "i", "32'd4", "xx", "00000000000"),
    ("d_uns", "i", "32'd4294967295", "xx", "00000000000"),
    ("d_uns", "2*i", "32'd0", "01", "00180000000"),
    ("d_uns", "2*i", "32'd1", "00", "06000000000"),
    ("d_uns", "2*i", "32'd2147483649", "00", "06000000000"),
    ("d_uns", "2*i", "32'd3", "xx", "00000000000"),
    ("d_uns", "2*i", "32'd4294967295", "xx", "00000000000"),
    ("d_uns", "2*i+1", "32'd0", "01", "00c00000000"),
    ("d_uns", "2*i+1", "32'd1", "00", "30000000000"),
    ("d_uns", "2*i+1", "32'd2147483649", "00", "30000000000"),
    ("d_uns", "2*i+1", "32'd3", "xx", "00000000000"),
    ("d_uns", "2*i+1", "32'd4294967295", "xx", "00000000000"),
    ("d_uns", "31-i", "32'd0", "xx", "00000000000"),
    ("d_uns", "31-i", "32'd4294967295", "xx", "00000000000"),
    ("d_uns", "i+8", "32'd0", "xx", "00000000000"),
    ("d_uns", "i+8", "32'd4294967295", "xx", "00000000000"),
    ("d_intw", "2*i+1", "-2147483647-1", "00", "00c00000000"),
    ("d_intw", "2*i+1", "-2147483647", "00", "30000000000"),
    ("d_intw", "2*i+1", "-1073741824", "xx", "00000000000"),
    ("d_intw", "2*i+1", "-2", "xx", "00000000000"),
    ("d_intw", "2*i+1", "-1", "xx", "00000000000"),
    ("d_intw", "2*i+1", "0", "00", "00c00000000"),
];

fn matrix_body(rows: &[&(&str, &str, &str, &str, &str)], wide: bool) -> String {
    let w = if wide { 2090 } else { 42 };
    let mut s = matrix_pkg(wide);
    s.push_str(&format!(
        "module top; import rp::*; r_t s;\n  initial s = {w}'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);\n"
    ));
    for (k, (cell, e, v, _, _)) in rows.iter().enumerate() {
        let ty = if cell.starts_with("d_int") {
            "int"
        } else {
            "int unsigned"
        };
        s.push_str(&format!(
            "  r_t t{k}; logic [1:0] y{k};\n  always_comb begin y{k} = 2'b00; t{k} = '0; \
             for ({ty} i = {v}; i == {v}; i = i ^ 1) begin y{k} = s.m[{e}].q; t{k}.m[{e}].q = 2'b11; end end\n"
        ));
    }
    s.push_str("  initial #900 $finish;\n  initial begin #1;\n");
    let t_hi = if wide { "[2089:2048]" } else { "" };
    for k in 0..rows.len() {
        s.push_str(&format!("    $display(\"A y=%b t=%h\",y{k},t{k}{t_hi});\n"));
    }
    s.push_str("    $finish; end\nendmodule\n");
    s
}

#[track_caller]
fn matrix(wide: bool, backend: Option<&str>) {
    let rows: Vec<_> = MATRIX
        .iter()
        .filter(|r| r.0.ends_with('w') == wide)
        .collect();
    let (got, rc, all) = run_on(&matrix_body(&rows, wide), backend);
    assert_eq!(rc, 0, "{all}");
    assert_eq!(got.len(), rows.len(), "{all}");
    for (r, g) in rows.iter().zip(&got) {
        assert_eq!(g, &format!("A y={} t={}", r.3, r.4), "{r:?}");
    }
}

#[test]
fn matrix_native() {
    matrix(false, None);
}

#[test]
fn matrix_vm() {
    matrix(false, Some("vm"));
}

#[test]
fn matrix_interp() {
    matrix(false, Some("interp"));
}

#[test]
fn matrix_wide_net() {
    matrix(true, None);
}

#[test]
fn hierarchical_names_stay_outside() {
    loud(
        &format!("{PKG}module top; import rp::*; r_t s; logic [1:0] x; initial begin s = '0; x = top.s.m[2].q; $finish; end endmodule\n"),
        "VITA-E3010",
    );
}

#[test]
fn no_index_starts_no_chain() {
    loud(
        &format!("{PKG}module top; import rp::*; r_t s; logic [1:0] x; initial begin x = s.m.q; $finish; end endmodule\n"),
        "VITA-E3010",
    );
    loud(
        &format!("{PKG}module top; import rp::*; r_t s; initial begin s.m.q = 2'b01; $finish; end endmodule\n"),
        "VITA-E3010",
    );
}

/// A force after a plain `=` in one body: the natural text is refused by the parser,
/// and its hand form, the node an admitted site would leave, is refused by the
/// elaborator as a force target. vl: `Force left-hand-side must not have variable
/// bit/part select`; s2 and iverilog refuse both.
#[test]
fn a_force_after_a_plain_assign_is_loud() {
    let m = |stmt: &str| {
        format!(
            "{PKG}module top; import rp::*; r_t s; logic x, o;
  always_comb for (int i = 0; i < 4; i++) begin o = 1'b0; {stmt} end
  initial #1000 $finish;
  initial begin x = 1'b1; #1 $display(\"A s=%h o=%b\", s, o); $finish; end
endmodule
"
        )
    };
    loud(&m("force s.m[i].e = x;"), "on a packed-array struct member");
    loud(
        &m("force s[(((i) >= 0 && (i) < 4) ? (i)*3 + 3 : 2097152) +: 1] = x;"),
        "force target must be a whole net/variable",
    );
}

#[test]
fn a_pattern_stays_loud_on_the_routes_the_parser_cannot_see() {
    loud(
        &format!(
            "{PKG}module sub(input rp::r_t i, output logic o); assign o = i.a; endmodule
module top; logic o; sub u(.i('{{a: 1'b1, m: '0, z: 3'd0}}), .o(o)); endmodule\n"
        ),
        "VITA-E3009",
    );
    loud(
        &format!("{PKG}module top; import rp::*; s_t t; initial begin t.v[1].n = '{{g: 1'b1, h: 2'b01}}; $finish; end endmodule\n"),
        "VITA-E3009",
    );
}

/// What the literal arm inherits from the node it hands over.
#[test]
fn inherited_refusals() {
    let m = |b: &str| {
        format!(
            "{PKG}module top; import rp::*; r_t s; logic x; logic [7:0] v; int n; {b} endmodule\n"
        )
    };
    for b in [
        "always @(posedge s.m[1].e) n = 1;",
        "initial force s.m[1].e = 1'b1;",
        "initial assign s.m[1].e = 1'b1;",
        "initial n = $sscanf(\"1\", \"%d\", s.m[1].q);",
    ] {
        let (got, rc, all) = run_on(&m(b), None);
        assert_eq!(rc, 1, "{b}: {all}");
        assert!(got.is_empty(), "{b}: {all}");
    }
    loud(
        &m("initial begin n = $size(s.m); $finish; end").replace("$size(s.m)", "$size(s.m[1])"),
        "VITA-E",
    );
    loud(
        &m("initial begin n = $left(s.m[1]); $finish; end"),
        "VITA-E",
    );
    loud(
        &format!("{PKG}module top; logic [1:0] x; assign x = rp::D.m; endmodule\n"),
        "VITA-E",
    );
}

/// Expected: vl+s2.
#[test]
fn p_a1() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  always_comb for (int i = 0; i < 4; i++) s.m[i].q = v[1:0] + 2'(i);
  always_comb for (int i = 0; i < 4; i++) s.m[i].qe = v[i];
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.q = v + 4'(i);
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.qe = v[i];
  always_comb for (int i = 0; i < 3; i++) s.n[i].b.d = v[i+1];
  always_comb for (int i = 0; i < 3; i++) s.n[i].b.de = ~v[i];
  always_comb for (int unsigned i = 0; i < 5; i++) s.b[i].q = i[0] ^ v[0];
  assign s.z = v;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h",s); v=4'b1010; #1 $display("A s=%h",s); v=4'b1111; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=078de335955", "A s=18f31dca6aa", "A s=2b3c706fd5f"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a1w() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z; logic [2047:0] pad; } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  always_comb for (int i = 0; i < 4; i++) s.m[i].q = v[1:0] + 2'(i);
  always_comb for (int i = 0; i < 4; i++) s.m[i].qe = v[i];
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.q = v + 4'(i);
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.qe = v[i];
  always_comb for (int i = 0; i < 3; i++) s.n[i].b.d = v[i+1];
  always_comb for (int i = 0; i < 3; i++) s.n[i].b.de = ~v[i];
  always_comb for (int unsigned i = 0; i < 5; i++) s.b[i].q = i[0] ^ v[0];
  assign s.z = v; assign s.pad = '0;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h",s); v=4'b1010; #1 $display("A s=%h",s); v=4'b1111; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=078de33595500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "A s=18f31dca6aa00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "A s=2b3c706fd5f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a3() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; logic k;
  for (genvar i = 0; i < 4; i++) begin : g1 assign s.m[3-i].qe = 1'b0; end
  for (genvar i = 0; i < 4; i++) begin : g2 assign s.m[i].q = '0; end
  always_comb begin
    s.n = '0;
    for (int i = 0; i < 1; i++) begin
      if (k) begin s.n[i].a.q = v; s.n[i+1].a.q = ~v; end
      else begin s.n[2*i].b.d = v[0]; s.n[2*i+1].b.d = v[1]; end
    end
  end
  assign s.b = '1; assign s.z = v;
  initial #900 $finish;
  initial begin v=4'b0101; k=0; #1 $display("A s=%h",s); k=1; #1 $display("A s=%h",s); v=4'b1110; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=000000005f5", "A s=000005051f5", "A s=0000008e1fe"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a3w() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z; logic [2047:0] pad; } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; logic k;
  for (genvar i = 0; i < 4; i++) begin : g1 assign s.m[3-i].qe = 1'b0; end
  for (genvar i = 0; i < 4; i++) begin : g2 assign s.m[i].q = '0; end
  always_comb begin
    s.n = '0;
    for (int i = 0; i < 1; i++) begin
      if (k) begin s.n[i].a.q = v; s.n[i+1].a.q = ~v; end
      else begin s.n[2*i].b.d = v[0]; s.n[2*i+1].b.d = v[1]; end
    end
  end
  assign s.b = '1; assign s.z = v; assign s.pad = '0;
  initial #900 $finish;
  initial begin v=4'b0101; k=0; #1 $display("A s=%h",s); k=1; #1 $display("A s=%h",s); v=4'b1110; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=000000005f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "A s=000005051f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "A s=0000008e1fe00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a5() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; logic [7:0] y; logic [1:0] w [4];
  always_comb for (int i = 0; i < 4; i++) w[i] = s.m[i].q;
  always_comb begin y = 0; for (int i = 0; i < 4; i++) y = y + s.m[i].q; end
  always_comb for (int i = 0; i < 4; i++) s.m[3-i].q = v[1:0] ^ 2'(i);
  assign s.z = v;
  assign s.n = '0; assign s.b = '0;
  always_comb for (int i = 0; i < 4; i++) s.m[i].qe = 1'b1;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A y=%h w=%h %h %h %h",y,w[0],w[1],w[2],w[3]); v=4'b0010; #1 $display("A y=%h w=%h %h %h %h",y,w[0],w[1],w[2],w[3]); $finish; end
endmodule
"#,
        &["A y=06 w=2 3 0 1", "A y=06 w=1 0 3 2"],
    );
}

/// Expected: pre+ivh.
#[test]
fn p_e_in() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s;
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  logic [1:0] y_V [0:3]; r_t t_V;
  for (genvar i = 0; i <= 3; i++) begin : g_V assign y_V[i] = s.m[i].q; end
  for (genvar i = 0; i <= 3; i++) begin : w_V assign t_V.m[i].qe = 1'b1; end
  logic [1:0] y_2V [0:3]; r_t t_2V;
  for (genvar i = 0; i <= 3; i++) begin : g_2V assign y_2V[i] = s.m[2*i].q; end
  for (genvar i = 0; i <= 3; i++) begin : w_2V assign t_2V.m[2*i].qe = 1'b1; end
  logic [1:0] y_2V1 [0:3]; r_t t_2V1;
  for (genvar i = 0; i <= 3; i++) begin : g_2V1 assign y_2V1[i] = s.m[2*i+1].q; end
  for (genvar i = 0; i <= 3; i++) begin : w_2V1 assign t_2V1.m[2*i+1].qe = 1'b1; end
  logic [1:0] y_31V [0:3]; r_t t_31V;
  for (genvar i = 0; i <= 3; i++) begin : g_31V assign y_31V[i] = s.m[31-i].q; end
  for (genvar i = 0; i <= 3; i++) begin : w_31V assign t_31V.m[31-i].qe = 1'b1; end
  logic [1:0] y_V8 [0:3]; r_t t_V8;
  for (genvar i = 0; i <= 3; i++) begin : g_V8 assign y_V8[i] = s.m[i+8].q; end
  for (genvar i = 0; i <= 3; i++) begin : w_V8 assign t_V8.m[i+8].qe = 1'b1; end
  initial #900 $finish;
  initial begin #1;
    $display("A V 0 y=%b",y_V[0]);
    $display("A V 1 y=%b",y_V[1]);
    $display("A V 2 y=%b",y_V[2]);
    $display("A V 3 y=%b",y_V[3]);
    $display("A V t=%h",t_V);
    $display("A 2V 0 y=%b",y_2V[0]);
    $display("A 2V 1 y=%b",y_2V[1]);
    $display("A 2V 2 y=%b",y_2V[2]);
    $display("A 2V 3 y=%b",y_2V[3]);
    $display("A 2V t=%h",t_2V);
    $display("A 2V1 0 y=%b",y_2V1[0]);
    $display("A 2V1 1 y=%b",y_2V1[1]);
    $display("A 2V1 2 y=%b",y_2V1[2]);
    $display("A 2V1 3 y=%b",y_2V1[3]);
    $display("A 2V1 t=%h",t_2V1);
    $display("A 31V 0 y=%b",y_31V[0]);
    $display("A 31V 1 y=%b",y_31V[1]);
    $display("A 31V 2 y=%b",y_31V[2]);
    $display("A 31V 3 y=%b",y_31V[3]);
    $display("A 31V t=%h",t_31V);
    $display("A V8 0 y=%b",y_V8[0]);
    $display("A V8 1 y=%b",y_V8[1]);
    $display("A V8 2 y=%b",y_V8[2]);
    $display("A V8 3 y=%b",y_V8[3]);
    $display("A V8 t=%h",t_V8);
    $finish; end
endmodule
"#,
        &[
            "A V 0 y=01",
            "A V 1 y=01",
            "A V 2 y=00",
            "A V 3 y=00",
            "A V t=xXXXxxxxxxx",
            "A 2V 0 y=01",
            "A 2V 1 y=00",
            "A 2V 2 y=xx",
            "A 2V 3 y=xx",
            "A 2V t=xXxXxxxxxxx",
            "A 2V1 0 y=01",
            "A 2V1 1 y=00",
            "A 2V1 2 y=xx",
            "A 2V1 3 y=xx",
            "A 2V1 t=xXXxxxxxxxx",
            "A 31V 0 y=xx",
            "A 31V 1 y=xx",
            "A 31V 2 y=xx",
            "A 31V 3 y=xx",
            "A 31V t=xxxxxxxxxxx",
            "A V8 0 y=xx",
            "A V8 1 y=xx",
            "A V8 2 y=xx",
            "A V8 3 y=xx",
            "A V8 t=xxxxxxxxxxx",
        ],
    );
}

/// Expected: pre+ivh.
#[test]
fn p_e_neg() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s;
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  logic [1:0] y_V [-3:3]; r_t t_V;
  for (genvar i = -3; i <= 3; i++) begin : g_V assign y_V[i] = s.m[i].q; end
  for (genvar i = -3; i <= 3; i++) begin : w_V assign t_V.m[i].qe = 1'b1; end
  logic [1:0] y_2V [-3:3]; r_t t_2V;
  for (genvar i = -3; i <= 3; i++) begin : g_2V assign y_2V[i] = s.m[2*i].q; end
  for (genvar i = -3; i <= 3; i++) begin : w_2V assign t_2V.m[2*i].qe = 1'b1; end
  logic [1:0] y_2V1 [-3:3]; r_t t_2V1;
  for (genvar i = -3; i <= 3; i++) begin : g_2V1 assign y_2V1[i] = s.m[2*i+1].q; end
  for (genvar i = -3; i <= 3; i++) begin : w_2V1 assign t_2V1.m[2*i+1].qe = 1'b1; end
  logic [1:0] y_31V [-3:3]; r_t t_31V;
  for (genvar i = -3; i <= 3; i++) begin : g_31V assign y_31V[i] = s.m[31-i].q; end
  for (genvar i = -3; i <= 3; i++) begin : w_31V assign t_31V.m[31-i].qe = 1'b1; end
  logic [1:0] y_V8 [-3:3]; r_t t_V8;
  for (genvar i = -3; i <= 3; i++) begin : g_V8 assign y_V8[i] = s.m[i+8].q; end
  for (genvar i = -3; i <= 3; i++) begin : w_V8 assign t_V8.m[i+8].qe = 1'b1; end
  initial #900 $finish;
  initial begin #1;
    $display("A V -3 y=%b",y_V[-3]);
    $display("A V -2 y=%b",y_V[-2]);
    $display("A V -1 y=%b",y_V[-1]);
    $display("A V 0 y=%b",y_V[0]);
    $display("A V 1 y=%b",y_V[1]);
    $display("A V 2 y=%b",y_V[2]);
    $display("A V 3 y=%b",y_V[3]);
    $display("A V t=%h",t_V);
    $display("A 2V -3 y=%b",y_2V[-3]);
    $display("A 2V -2 y=%b",y_2V[-2]);
    $display("A 2V -1 y=%b",y_2V[-1]);
    $display("A 2V 0 y=%b",y_2V[0]);
    $display("A 2V 1 y=%b",y_2V[1]);
    $display("A 2V 2 y=%b",y_2V[2]);
    $display("A 2V 3 y=%b",y_2V[3]);
    $display("A 2V t=%h",t_2V);
    $display("A 2V1 -3 y=%b",y_2V1[-3]);
    $display("A 2V1 -2 y=%b",y_2V1[-2]);
    $display("A 2V1 -1 y=%b",y_2V1[-1]);
    $display("A 2V1 0 y=%b",y_2V1[0]);
    $display("A 2V1 1 y=%b",y_2V1[1]);
    $display("A 2V1 2 y=%b",y_2V1[2]);
    $display("A 2V1 3 y=%b",y_2V1[3]);
    $display("A 2V1 t=%h",t_2V1);
    $display("A 31V -3 y=%b",y_31V[-3]);
    $display("A 31V -2 y=%b",y_31V[-2]);
    $display("A 31V -1 y=%b",y_31V[-1]);
    $display("A 31V 0 y=%b",y_31V[0]);
    $display("A 31V 1 y=%b",y_31V[1]);
    $display("A 31V 2 y=%b",y_31V[2]);
    $display("A 31V 3 y=%b",y_31V[3]);
    $display("A 31V t=%h",t_31V);
    $display("A V8 -3 y=%b",y_V8[-3]);
    $display("A V8 -2 y=%b",y_V8[-2]);
    $display("A V8 -1 y=%b",y_V8[-1]);
    $display("A V8 0 y=%b",y_V8[0]);
    $display("A V8 1 y=%b",y_V8[1]);
    $display("A V8 2 y=%b",y_V8[2]);
    $display("A V8 3 y=%b",y_V8[3]);
    $display("A V8 t=%h",t_V8);
    $finish; end
endmodule
"#,
        &[
            "A V -3 y=xx",
            "A V -2 y=xx",
            "A V -1 y=xx",
            "A V 0 y=01",
            "A V 1 y=01",
            "A V 2 y=00",
            "A V 3 y=00",
            "A V t=xXXXxxxxxxx",
            "A 2V -3 y=xx",
            "A 2V -2 y=xx",
            "A 2V -1 y=xx",
            "A 2V 0 y=01",
            "A 2V 1 y=00",
            "A 2V 2 y=xx",
            "A 2V 3 y=xx",
            "A 2V t=xXxXxxxxxxx",
            "A 2V1 -3 y=xx",
            "A 2V1 -2 y=xx",
            "A 2V1 -1 y=xx",
            "A 2V1 0 y=01",
            "A 2V1 1 y=00",
            "A 2V1 2 y=xx",
            "A 2V1 3 y=xx",
            "A 2V1 t=xXXxxxxxxxx",
            "A 31V -3 y=xx",
            "A 31V -2 y=xx",
            "A 31V -1 y=xx",
            "A 31V 0 y=xx",
            "A 31V 1 y=xx",
            "A 31V 2 y=xx",
            "A 31V 3 y=xx",
            "A 31V t=xxxxxxxxxxx",
            "A V8 -3 y=xx",
            "A V8 -2 y=xx",
            "A V8 -1 y=xx",
            "A V8 0 y=xx",
            "A V8 1 y=xx",
            "A V8 2 y=xx",
            "A V8 3 y=xx",
            "A V8 t=xxxxxxxxxxx",
        ],
    );
}

/// Expected: pre+ivh.
#[test]
fn p_e_oob() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s;
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  logic [1:0] y_V [0:9]; r_t t_V;
  for (genvar i = 0; i <= 9; i++) begin : g_V assign y_V[i] = s.m[i].q; end
  for (genvar i = 0; i <= 9; i++) begin : w_V assign t_V.m[i].qe = 1'b1; end
  logic [1:0] y_2V [0:9]; r_t t_2V;
  for (genvar i = 0; i <= 9; i++) begin : g_2V assign y_2V[i] = s.m[2*i].q; end
  for (genvar i = 0; i <= 9; i++) begin : w_2V assign t_2V.m[2*i].qe = 1'b1; end
  logic [1:0] y_2V1 [0:9]; r_t t_2V1;
  for (genvar i = 0; i <= 9; i++) begin : g_2V1 assign y_2V1[i] = s.m[2*i+1].q; end
  for (genvar i = 0; i <= 9; i++) begin : w_2V1 assign t_2V1.m[2*i+1].qe = 1'b1; end
  logic [1:0] y_31V [0:9]; r_t t_31V;
  for (genvar i = 0; i <= 9; i++) begin : g_31V assign y_31V[i] = s.m[31-i].q; end
  for (genvar i = 0; i <= 9; i++) begin : w_31V assign t_31V.m[31-i].qe = 1'b1; end
  logic [1:0] y_V8 [0:9]; r_t t_V8;
  for (genvar i = 0; i <= 9; i++) begin : g_V8 assign y_V8[i] = s.m[i+8].q; end
  for (genvar i = 0; i <= 9; i++) begin : w_V8 assign t_V8.m[i+8].qe = 1'b1; end
  initial #900 $finish;
  initial begin #1;
    $display("A V 0 y=%b",y_V[0]);
    $display("A V 1 y=%b",y_V[1]);
    $display("A V 2 y=%b",y_V[2]);
    $display("A V 3 y=%b",y_V[3]);
    $display("A V 4 y=%b",y_V[4]);
    $display("A V 5 y=%b",y_V[5]);
    $display("A V 6 y=%b",y_V[6]);
    $display("A V 7 y=%b",y_V[7]);
    $display("A V 8 y=%b",y_V[8]);
    $display("A V 9 y=%b",y_V[9]);
    $display("A V t=%h",t_V);
    $display("A 2V 0 y=%b",y_2V[0]);
    $display("A 2V 1 y=%b",y_2V[1]);
    $display("A 2V 2 y=%b",y_2V[2]);
    $display("A 2V 3 y=%b",y_2V[3]);
    $display("A 2V 4 y=%b",y_2V[4]);
    $display("A 2V 5 y=%b",y_2V[5]);
    $display("A 2V 6 y=%b",y_2V[6]);
    $display("A 2V 7 y=%b",y_2V[7]);
    $display("A 2V 8 y=%b",y_2V[8]);
    $display("A 2V 9 y=%b",y_2V[9]);
    $display("A 2V t=%h",t_2V);
    $display("A 2V1 0 y=%b",y_2V1[0]);
    $display("A 2V1 1 y=%b",y_2V1[1]);
    $display("A 2V1 2 y=%b",y_2V1[2]);
    $display("A 2V1 3 y=%b",y_2V1[3]);
    $display("A 2V1 4 y=%b",y_2V1[4]);
    $display("A 2V1 5 y=%b",y_2V1[5]);
    $display("A 2V1 6 y=%b",y_2V1[6]);
    $display("A 2V1 7 y=%b",y_2V1[7]);
    $display("A 2V1 8 y=%b",y_2V1[8]);
    $display("A 2V1 9 y=%b",y_2V1[9]);
    $display("A 2V1 t=%h",t_2V1);
    $display("A 31V 0 y=%b",y_31V[0]);
    $display("A 31V 1 y=%b",y_31V[1]);
    $display("A 31V 2 y=%b",y_31V[2]);
    $display("A 31V 3 y=%b",y_31V[3]);
    $display("A 31V 4 y=%b",y_31V[4]);
    $display("A 31V 5 y=%b",y_31V[5]);
    $display("A 31V 6 y=%b",y_31V[6]);
    $display("A 31V 7 y=%b",y_31V[7]);
    $display("A 31V 8 y=%b",y_31V[8]);
    $display("A 31V 9 y=%b",y_31V[9]);
    $display("A 31V t=%h",t_31V);
    $display("A V8 0 y=%b",y_V8[0]);
    $display("A V8 1 y=%b",y_V8[1]);
    $display("A V8 2 y=%b",y_V8[2]);
    $display("A V8 3 y=%b",y_V8[3]);
    $display("A V8 4 y=%b",y_V8[4]);
    $display("A V8 5 y=%b",y_V8[5]);
    $display("A V8 6 y=%b",y_V8[6]);
    $display("A V8 7 y=%b",y_V8[7]);
    $display("A V8 8 y=%b",y_V8[8]);
    $display("A V8 9 y=%b",y_V8[9]);
    $display("A V8 t=%h",t_V8);
    $finish; end
endmodule
"#,
        &[
            "A V 0 y=01",
            "A V 1 y=01",
            "A V 2 y=00",
            "A V 3 y=00",
            "A V 4 y=xx",
            "A V 5 y=xx",
            "A V 6 y=xx",
            "A V 7 y=xx",
            "A V 8 y=xx",
            "A V 9 y=xx",
            "A V t=xXXXxxxxxxx",
            "A 2V 0 y=01",
            "A 2V 1 y=00",
            "A 2V 2 y=xx",
            "A 2V 3 y=xx",
            "A 2V 4 y=xx",
            "A 2V 5 y=xx",
            "A 2V 6 y=xx",
            "A 2V 7 y=xx",
            "A 2V 8 y=xx",
            "A 2V 9 y=xx",
            "A 2V t=xXxXxxxxxxx",
            "A 2V1 0 y=01",
            "A 2V1 1 y=00",
            "A 2V1 2 y=xx",
            "A 2V1 3 y=xx",
            "A 2V1 4 y=xx",
            "A 2V1 5 y=xx",
            "A 2V1 6 y=xx",
            "A 2V1 7 y=xx",
            "A 2V1 8 y=xx",
            "A 2V1 9 y=xx",
            "A 2V1 t=xXXxxxxxxxx",
            "A 31V 0 y=xx",
            "A 31V 1 y=xx",
            "A 31V 2 y=xx",
            "A 31V 3 y=xx",
            "A 31V 4 y=xx",
            "A 31V 5 y=xx",
            "A 31V 6 y=xx",
            "A 31V 7 y=xx",
            "A 31V 8 y=xx",
            "A 31V 9 y=xx",
            "A 31V t=xxxxxxxxxxx",
            "A V8 0 y=xx",
            "A V8 1 y=xx",
            "A V8 2 y=xx",
            "A V8 3 y=xx",
            "A V8 4 y=xx",
            "A V8 5 y=xx",
            "A V8 6 y=xx",
            "A V8 7 y=xx",
            "A V8 8 y=xx",
            "A V8 9 y=xx",
            "A V8 t=xxxxxxxxxxx",
        ],
    );
}

/// Expected: vl+s2.
#[test]
fn p_f_fn() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic k; logic [7:0] y; logic [7:0] y2; typedef enum logic [1:0] {A0,A1,A2,A3} e_t; e_t en;
  typedef struct packed { logic [1:0] a; logic b; } o_t; o_t o [4]; int arr [4];
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); k = 1; arr[0]=1;arr[1]=2;arr[2]=3;arr[3]=4; end
  always_comb begin y = 0; for (int i = 0; i < 4; i++) y[2*i+:2] = sw(s.m[i].q, k); end
  initial #900 $finish;
  initial begin #1 $display("A y=%h",y); s[0]=~s[0]; #1 $display("A y=%h",y); $finish; end
endmodule
"#,
        &["A y=0a", "A y=0a"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_f_ter() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic k; logic [7:0] y; logic [7:0] y2; typedef enum logic [1:0] {A0,A1,A2,A3} e_t; e_t en;
  typedef struct packed { logic [1:0] a; logic b; } o_t; o_t o [4]; int arr [4];
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); k = 1; arr[0]=1;arr[1]=2;arr[2]=3;arr[3]=4; end
  always_comb begin y = 0; for (int i = 0; i < 4; i++) y = (y << 2) | 8'(k ? s.m[i].q : s.m[2*i].q); end
  initial #900 $finish;
  initial begin #1 $display("A y=%h",y); s[0]=~s[0]; #1 $display("A y=%h",y); $finish; end
endmodule
"#,
        &["A y=50", "A y=50"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_g_lv() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic k; logic [7:0] y; logic [7:0] y2; typedef enum logic [1:0] {A0,A1,A2,A3} e_t; e_t en;
  typedef struct packed { logic [1:0] a; logic b; } o_t; o_t o [4]; int arr [4];
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); k = 1; arr[0]=1;arr[1]=2;arr[2]=3;arr[3]=4; end
  logic [15:0] p; for (genvar i = 0; i < 4; i++) begin : g assign p[4*i+:4] = s.m[i]; end
 assign y = p[7:0] ^ p[15:8];
  initial #900 $finish;
  initial begin #1 $display("A y=%h",y); s[0]=~s[0]; #1 $display("A y=%h",y); $finish; end
endmodule
"#,
        &["A y=22", "A y=22"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_h_narrow() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s,z; logic [3:0] v; logic k; logic [7:0] y;
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin z = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); v = 4'b0110; k = 1; end
  always_comb begin s = '0; for (int i = 0; i < 4; i++) s.m[i].q = 1'b1; end
  initial #900 $finish;
  initial begin #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=12480000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_h_read() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s,z; logic [3:0] v; logic k; logic [7:0] y;
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin z = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); v = 4'b0110; k = 1; end
  always_comb begin s = '0; for (int i = 0; i < 4; i++) s.m[i].q = z.m[3-i].q; end
  initial #900 $finish;
  initial begin #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=12000000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_h_wide() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s,z; logic [3:0] v; logic k; logic [7:0] y;
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin z = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); v = 4'b0110; k = 1; end
  always_comb begin s = '0; for (int i = 0; i < 4; i++) s.m[i].q = 8'hA5; end
  initial #900 $finish;
  initial begin #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=12480000000"],
    );
}

/// Expected: pre+vl.
#[test]
fn p_g_rhs() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic k; logic [7:0] y; logic [7:0] y2; typedef enum logic [1:0] {A0,A1,A2,A3} e_t; e_t en;
  typedef struct packed { logic [1:0] a; logic b; } o_t; o_t o [4]; int arr [4];
  function automatic logic [1:0] sw(logic [1:0] x, logic f); return f ? {x[0],x[1]} : x; endfunction
  initial begin s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3); k = 1; arr[0]=1;arr[1]=2;arr[2]=3;arr[3]=4; end
  for (genvar i = 0; i < 4; i++) begin : g assign y[i] = s.m[i].qe; end
  initial #900 $finish;
  initial begin #1 $display("A y=%h",y); s[0]=~s[0]; #1 $display("A y=%h",y); $finish; end
endmodule
"#,
        &["A y=xf", "A y=xf"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n1_out() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  function automatic void gw(output logic [1:0] o, input logic [1:0] a); o = a; endfunction
  always_comb begin s = '0; for (int i = 0; i < 4; i++) gw(s.m[i].q, 2'(i)+v[1:0]); end
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h",s); v=4'b1010; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=06880000000", "A s=10d00000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n6_whole() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s, t; logic [2:0] e [4]; logic b [5]; logic [6:0] n [3];
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  always_comb for (int i = 0; i < 4; i++) e[i] = s.m[i];
  always_comb for (int unsigned i = 0; i < 5; i++) b[i] = s.b[i];
  always_comb for (int i = 0; i < 3; i++) n[i] = s.n[i];
  always_comb begin t = '0; for (int i = 0; i < 5; i++) t.b[i].q = s.b[i].q; end
  initial #900 $finish;
  initial begin #1 $display("A e=%b %b %b %b b=%b%b%b%b%b n=%b %b %b t=%h",e[0],e[1],e[2],e[3],b[0],b[1],b[2],b[3],b[4],n[0],n[1],n[2],t); $finish; end
endmodule
"#,
        &["A e=011 011 001 001 b=11010 n=1101010 1100010 1100011 t=000000000b0"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n7_ctx() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [7:0] y; logic [7:0] y2; logic c; int w; int j;
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  always_comb begin y = 0; y2 = 0; c = 0; w = 0; j = 0; for (int i = 0; i < 4; i++) begin
    y = y + 8'(~s.m[i].q);
    y2 = y2 ^ (~s.m[i].q);
    c = c | ((s.m[i].q - 1) < 0);
    w = w + $bits(s.m[i].q);
    j = j + int'($signed(s.m[i].q));
  end end
  initial #900 $finish;
  initial begin #1 $display("A y=%h y2=%h c=%b w=%0d j=%0d",y,y2,c,w,j); $finish; end
endmodule
"#,
        &["A y=fa y2=00 c=0 w=8 j=2"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n8_settle() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; logic [1:0] y [4];
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.q = v + 4'(i);
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.qe = v[i];
  always_comb for (int i = 0; i < 3; i++) begin s.n[i].b.d = s.n[i].a.qe; s.n[i].b.de = s.n[i].b.d; end
  always_comb for (int i = 0; i < 3; i++) s.m[i].q = 2'(s.n[i].a.q) ^ 2'(s.n[i].b.de);
  always_comb s.m[3].q = '0;
  always_comb for (int i = 0; i < 4; i++) s.m[i].qe = ^s.m[i].q;
  always_comb for (int i = 0; i < 4; i++) y[i] = s.m[i].q;
  assign s.z = v; assign s.b = '0;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h y=%b %b %b %b",s,y[0],y[1],y[2],y[3]); v=4'b1010; #1 $display("A s=%h",s); v=4'b1111; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &[
            "A s=05a1fb05e05 y=00 10 10 00",
            "A s=00b705fa00a",
            "A s=0074787fe0f",
        ],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n8w_settle() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z; logic [2047:0] pad; } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; logic [1:0] y [4];
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.q = v + 4'(i);
  always_comb for (int i = 0; i < 3; i++) s.n[i].a.qe = v[i];
  always_comb for (int i = 0; i < 3; i++) begin s.n[i].b.d = s.n[i].a.qe; s.n[i].b.de = s.n[i].b.d; end
  always_comb for (int i = 0; i < 3; i++) s.m[i].q = 2'(s.n[i].a.q) ^ 2'(s.n[i].b.de);
  always_comb s.m[3].q = '0;
  always_comb for (int i = 0; i < 4; i++) s.m[i].qe = ^s.m[i].q;
  always_comb for (int i = 0; i < 4; i++) y[i] = s.m[i].q;
  assign s.z = v; assign s.b = '0; assign s.pad = '0;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h y=%b %b %b %b",s,y[0],y[1],y[2],y[3]); v=4'b1010; #1 $display("A s=%h",s); v=4'b1111; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=05a1fb05e0500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 y=00 10 10 00", "A s=00b705fa00a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", "A s=0074787fe0f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_n9_auto() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic clk = 0; logic [3:0] v; logic [1:0] y;
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  always @(posedge clk) y <= 2'b00;
  always_comb begin v = 0; for (int i = 0; i < 4; i++) begin automatic int k = i + 1; v[i] = |s.m[i].q; end end
  initial #900 $finish;
  initial begin #1 $display("A v=%b",v); $finish; end
endmodule
"#,
        &["A v=0011"],
    );
}

/// Expected: vl+pre.
#[test]
fn p_w_out() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; int n;
  function automatic logic [1:0] f(output int k); k = 2; return 2'b11; endfunction
  always_comb begin s = '0; n = 0; for (int i = 0; i < 1; i++) begin s.m[i].q = f(i); n = i; end end
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h n=%0d",s,n); v=4'b1010; #1 $display("A s=%h n=%0d",s,n); $finish; end
endmodule
"#,
        &["A s=06000000000 n=2", "A s=06000000000 n=2"],
    );
}

/// Expected: vl+pre.
#[test]
fn p_w_ino() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v; int n;
  function automatic logic [1:0] f(inout int k); k = k + 2; return 2'b10; endfunction
  always_comb begin s = '0; n = 0; for (int i = 0; i < 1; i++) begin s.m[i].q = f(i); n = i; end end
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h n=%0d",s,n); v=4'b1010; #1 $display("A s=%h n=%0d",s,n); $finish; end
endmodule
"#,
        &["A s=04000000000 n=2", "A s=04000000000 n=2"],
    );
}

/// Expected: vl+pre.
#[test]
fn p_r_out() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [7:0] y; int n;
  function automatic logic [1:0] f(output int k); k = 2; return 2'b11; endfunction
  initial s = 42'(112'h3A5C96F1E2D4B3AA5C96F1E2D4B3);
  always_comb begin y = 0; n = 0; for (int i = 0; i < 1; i++) begin y = {f(i), s.m[i].q}; n = i; end end
  initial #900 $finish;
  initial begin #1 $display("A y=%h n=%0d",y,n); $finish; end
endmodule
"#,
        &["A y=0c n=2"],
    );
}

/// Expected: pre hand+vl.
#[test]
fn p_a2x() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  for (genvar i = 0; i < 4; i++) begin : g1 assign s.m[i].qe = v[i]; end
  for (genvar i = 0; i < 4; i++) begin : g2 assign s.m[3-i].q = v[1:0]; end
  assign s.z = v;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h",s); v=4'b1010; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=134Xxxxxxx5", "A s=2cbXxxxxxxa"],
    );
}

/// Expected: pre hand+vl.
#[test]
fn p_a4x() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s, r; logic [3:0] v;
  assign r = {42{1'b0}} | 42'(v) << 30;
  always_comb begin : p_rdy for (int i = 0; i < 3; i++) begin s.n[i].a.q = 4'(v[i]); s.n[i].a.qe = 1'b0; end end
  always_comb begin : p_pend for (int i = 0; i < 3; i++) begin s.n[i].b.de = v[i]; s.n[i].b.d = r.m[i].q == 2'd1 | r.m[i].qe; end end
  assign s.z = 4'h9;
  initial #900 $finish;
  initial begin v=4'b0110; #1 $display("A s=%h",s); v=4'b1001; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=xxxX4890Xx9", "A s=xxxX0021Xx9"],
    );
}

/// Expected: pre hand+vl.
#[test]
fn p_a7x() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { struct packed { logic [3:0] q; logic qe; } a; struct packed { logic d; logic de; } b; } m2_t;
  typedef struct packed { logic q; } b1_t;
  typedef struct packed { m1_t [3:0] m; m2_t [2:0] n; b1_t [4:0] b; logic [3:0] z;  } r_t;
endpackage
module top; import rp::*; r_t s; logic [3:0] v;
  assign s.m[0].qe = v[0]; assign s.m[1].qe = v[1]; assign s.m[2].qe = v[2]; assign s.m[3].qe = v[3];
  always_comb for (int i = 0; i < 4; i++) s.m[i].q = 2'(i) ^ v[1:0];
  assign s.z = v;
  initial #900 $finish;
  initial begin v=4'b0101; #1 $display("A s=%h",s); v=4'b0010; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=270Xxxxxxx5", "A s=10fXxxxxxx2"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_fresh() {
    value(
        r#"package pa;
  typedef struct packed { logic [1:0] q; logic r; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
package pb;
  typedef struct packed { logic r; logic [1:0] q; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
module top; pa::r_t a; pb::r_t b; logic [1:0] x, y;
  initial begin a = 6'b101100; b = 6'b101100; x = a.m[1].q; y = b.m[1].q; #1 $display("A x=%b y=%b",x,y); $finish; end
endmodule
"#,
        &["A x=10 y=01"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_fresh2() {
    value(
        r#"package pa;
  typedef struct packed { logic [1:0] q; logic r; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
package pb;
  typedef struct packed { logic [2:0][1:0] m; } r_t;
endpackage
module top; import pa::*; r_t a; pb::r_t b; logic [1:0] x; logic [1:0] y;
  initial begin a = 6'b101100; b = 6'b101100; x = a.m[1].q; y = b.m[1]; #1 $display("A x=%b y=%b",x,y); $finish; end
endmodule
"#,
        &["A x=10 y=11"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_bits() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
endpackage
module top; import rp::*; r_t s; s_t t;
  initial begin s = 16'h9a5c; #1 $display("A b=%0d c=%0d d=%0d",$bits(s),$bits(s.m),$bits(t.v)); $finish; end
endmodule
"#,
        &["A b=16 c=12 d=8"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_whole() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
endpackage
module top; import rp::*; r_t s; logic [11:0] x; logic [2:0] y;
  initial begin s = 16'h9a5c; x = s.m; y = s.m[1]; #1 $display("A x=%h y=%b",x,y); s.m = 12'h123; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A x=34b y=001", "A s=891c"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_patcast() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
endpackage
module top; import rp::*; r_t s;
  initial begin s = r_t'(16'h9a5c); #1 $display("A e=%b",s.m[1].e); $finish; end
endmodule
"#,
        &["A e=1"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_lit2() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
endpackage
module top; import rp::*; s_t t; logic [1:0] x; logic g; logic k;
  initial begin t = 16'ha5c3; x = t.v[1].n.h; g = t.v[1].n.g; k = t.v[0].k;
    #1 $display("A x=%b g=%b k=%b",x,g,k);
    t.v[0].n.h = 2'b01; #1 $display("A t=%h",t); $finish; end
endmodule
"#,
        &["A x=01 g=1 k=1", "A t=a3c3"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_alp() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
  typedef e_t [3:0] vec_t;
  parameter vec_t D = 12'h5a3;
endpackage
module sub(input rp::vec_t p, output logic [1:0] o); assign o = p[2].q; endmodule
module top; logic [1:0] o; rp::e_t [3:0] a; sub u(.p(a), .o(o));
  initial begin a = 12'h5a3; #1 $display("A o=%b b=%0d d=%h",o,$bits(rp::D),rp::D[1]); $finish; end
endmodule
"#,
        &["A o=11 b=12 d=4"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_z_rtwhole() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { struct packed { logic g; logic [1:0] h; } n; logic k; } d_t;
  typedef struct packed { logic a; e_t [3:0] m; logic [2:0] z; } r_t;
  typedef struct packed { d_t [1:0] v; logic [7:0] y; } s_t;
endpackage
module top; import rp::*; r_t s; logic [2:0] y; logic [7:0] u;
  always_comb begin u = 0; for (int i = 0; i < 4; i++) begin y = s.m[i]; u = u + 8'(y); end end
  initial begin s = 16'h9a5c; #1 $display("A u=%h",u); $finish; end
endmodule
"#,
        &["A u=0a"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_c_racl() {
    value(
        r#"package rp;
  parameter int unsigned NrP = 1;
  typedef logic [0:0] sel_t;
  typedef logic [1:0] vec_t;
  typedef struct packed { vec_t wr; vec_t rd; } pol_t;
  typedef struct packed { logic [3:0] base; sel_t policy_sel; logic en; } rng_t;
endpackage
module sub #(parameter int N = 2, parameter rp::sel_t Sel [N] = '{default:'0}, parameter rp::sel_t S1 = 1'b0, parameter rp::rng_t R = '{base:4'h1, policy_sel:1'b0, en:1'b1})
 (input rp::pol_t [rp::NrP-1:0] pols, input logic [1:0] role, output logic a, output logic b, output logic [3:0] c);
  for (genvar k = 0; k < N; k++) begin : gen assign a = |(pols[Sel[k]].rd & role); end
  assign b = |(pols[S1].wr & role);
  rp::pol_t p; assign p = pols[R.policy_sel]; assign c = p;
endmodule
module top; import rp::*; pol_t [NrP-1:0] pp; logic [1:0] r; logic a, b; logic [3:0] c;
  sub #(.N(1)) u(.pols(pp), .role(r), .a(a), .b(b), .c(c));
  initial begin pp = 4'b1001; r = 2'b01; #1 $display("A a=%b b=%b c=%b",a,b,c); r = 2'b10; #1 $display("A a=%b b=%b c=%b",a,b,c); $finish; end
endmodule
"#,
        &["A a=1 b=0 c=1001", "A a=0 b=1 c=1001"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_c_aparam() {
    value(
        r#"package rp;
  typedef logic [0:0][1:0] v_t;
  parameter v_t D = '0;
  parameter v_t E = 2'b10;
endpackage
module top; logic [1:0] y; initial begin y = rp::E[0]; $display("A d=%b e=%b y=%b",rp::D,rp::E,y); $finish; end endmodule
"#,
        &["A d=00 e=10 y=10"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a5_mreg_cell() {
    value(
        r#"// R04/R12 OpenTitan multireg shape: a struct member of a struct type with extra packed dims, element then member
package rp;
  typedef struct packed {logic [1:0] q; logic qe;} mreg_t;
  typedef struct packed {mreg_t [3:0] m; logic [3:0] z;} reg2hw_t;
endpackage
module m;
  rp::reg2hw_t r;
  initial begin r = '0; r.m[2].q = 2'b11; r.z = 4'h9; #1 $display("A mreg r=%h q=%b b=%0d", r, r.m[2].q, $bits(r)); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
        &["A mreg r=1809 q=11 b=16"],
    );
}

/// Expected: vl+s2.
#[test]
fn p_a6_ret_cell() {
    value(
        r#"// R04 prim_sha2_pkg shape: a package function returning a typedef with extra packed dims
package sp;
  typedef logic [7:0] w_t;
  function automatic w_t [1:0] f(logic [15:0] x); return x; endfunction
endpackage
module m;
  initial #1 $display("A ret f=%h b=%0d", sp::f(16'hA55A), $bits(sp::f(16'h0)));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
        &["A ret f=a55a b=16"],
    );
}

/// Expected: s2+vl.
#[test]
fn p_b_w1n() {
    value(
        r#"package rp;
  typedef struct packed { logic [1:0] q; logic qe; } m1_t;
  typedef struct packed { logic v; } b1_t;
  typedef struct packed { m1_t [3:0] m; b1_t [3:0] n; logic [3:0] z; } r_t;
endpackage
module top; import rp::*; r_t s; logic x; 
  always_ff @(posedge x) s.n[0] <= 1'b1;
  initial begin s = 20'h0; x=0; #1 x=1; #1 $display("A s=%h",s); $finish; end
endmodule
"#,
        &["A s=00010"],
    );
}

/// An element no page declares, each a cell the r1 lenses measured wrong on the row's
/// first build (row 41 r1c): a signed struct at any depth, an enum of a 2-state base at any
/// depth, a struct two levels down. Each is the member refusal now; the oracle text the
/// element would have to match is named beside it.
#[test]
fn an_element_no_page_declares_is_refused() {
    // c01_sgnest: sv2v, verilator `A x=-8 y=-7 xs0=-7 xs1=-8 lt=1`; r1 read 8.
    loud(
        r#"package p;
  typedef struct packed signed { logic [3:0] a; } sg_t;
  typedef struct packed { sg_t sub; logic q; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; int x, y; int xs [2];
  always_comb for (int i = 0; i < 2; i++) xs[i] = s.m[i].sub;
  initial #100 $finish;
  initial begin s = 10'b1000_0_1001_0; #1 x = s.m[1].sub; y = s.m[0].sub;
    $display("A x=%0d y=%0d xs0=%0d xs1=%0d lt=%0d", x, y, xs[0], xs[1], s.m[1].sub < 0); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // c03_sganon: sv2v, verilator `A x=-8 xs0=-7 xs1=-8`; r1 read 8.
    loud(
        r#"package p;
  typedef struct packed { struct packed signed { logic [3:0] a; } sub; logic q; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; int x; int xs [2];
  always_comb for (int i = 0; i < 2; i++) xs[i] = s.m[i].sub;
  initial #100 $finish;
  initial begin s = 10'b1000_0_1001_0; #1 x = s.m[1].sub; $display("A x=%0d xs0=%0d xs1=%0d", x, xs[0], xs[1]); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // d12_deepsign: sv2v, verilator `A x=-4 y=-3 xs=-3 -4`; r1 read 4.
    loud(
        r#"package p;
  typedef struct packed signed { logic [2:0] a; } sg_t;
  typedef struct packed { struct packed { sg_t b; logic c; } n; logic q; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; int x, y; int xs [2];
  always_comb for (int i = 0; i < 2; i++) xs[i] = s.m[i].n.b;
  initial #100 $finish;
  initial begin s = 10'b100_0_0_101_1_1; #1 x = s.m[1].n.b; y = s.m[0].n.b; $display("A x=%0d y=%0d xs=%0d %0d", x, y, xs[0], xs[1]); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // i01_enumbit: verilator `A lit=00 rt=00 ctl=00 lt=0 ctlt=0`, IEEE §7.2.1 2-state read; r1 read xx.
    loud(
        r#"package p;
  typedef enum bit [1:0] {A, B, C} be_e;
  typedef struct packed { be_e k; logic e; } e_t;
  typedef struct packed { logic a; e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; e_t t; logic [$bits(be_e)-1:0] x [2];
  always_comb for (int i = 0; i < 2; i++) x[i] = s.m[i].k;
  initial #100 $finish;
  initial begin  #1 $display("A lit=%b rt=%b ctl=%b lt=%b ctlt=%b", s.m[1].k, x[1], t.k, s.m[1].k < 0, t.k < 0); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // i02_enumbit_x: verilator `lit=10 rt=10`; r1 read 1x.
    loud(
        r#"package p;
  typedef enum bit [1:0] {A, B, C} be_e;
  typedef struct packed { be_e k; logic e; } e_t;
  typedef struct packed { logic a; e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; e_t t; logic [$bits(be_e)-1:0] x [2];
  always_comb for (int i = 0; i < 2; i++) x[i] = s.m[i].k;
  initial #100 $finish;
  initial begin s = 7'b0_1x1_x01; t = 3'b1x1; #1 $display("A lit=%b rt=%b ctl=%b lt=%b ctlt=%b", s.m[1].k, x[1], t.k, s.m[1].k < 0, t.k < 0); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // ns2: sv2v, verilator `A v0=-2 v2=-6 ..`; r1 read 14.
    loud(
        r#"package p;
  typedef struct packed signed { logic [3:0] a; } ss_t;
  typedef struct packed { ss_t s; logic b; } e5_t;
  typedef struct packed { logic [1:0] pre; e5_t [2:0] m; } r_t;
  typedef struct packed { logic [1:0] pre; e5_t m2; e5_t m1; e5_t m0; } t_t;
endpackage
module top; import p::*; r_t s; t_t t; int v0, v2, t0, t2; logic c0, c2, d0, d2; int k [3]; int g [3]; logic [7:0] w0, x0;
  assign v0 = s.m[0].s; assign v2 = s.m[2].s; assign t0 = t.m0.s; assign t2 = t.m2.s;
  assign c0 = s.m[0].s < 0; assign c2 = s.m[2].s < 0; assign d0 = t.m0.s < 0; assign d2 = t.m2.s < 0;
  assign w0 = s.m[0].s; assign x0 = t.m0.s;
  always_comb for (int i = 0; i < 3; i++) k[i] = s.m[i].s;
  for (genvar q = 0; q < 3; q++) begin : gl assign g[q] = s.m[q].s; end
  initial #2000 $finish;
  initial begin s = 17'b10_1010_1_0011_0_1110_1; t = s; #1
    $display("A v0=%0d v2=%0d t0=%0d t2=%0d c0=%b c2=%b d0=%b d2=%b w0=%h x0=%h k=%0d %0d %0d g=%0d %0d %0d", v0, v2, t0, t2, c0, c2, d0, d2, w0, x0, k[0], k[1], k[2], g[0], g[1], g[2]);
    $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // b4_nested_signed_lw: sv2v, verilator, iverilog `A v=-3 ..`; r1 read 13.
    loud(
        r#"package p;
  typedef struct packed signed { logic [3:0] a; } ss_t;
  typedef struct packed { ss_t s; logic b; } e5_t;
  typedef struct packed { logic [1:0] pre; e5_t [2:0] m; } r_t;
endpackage
module top; import p::*; r_t s; int v; logic [16:0] q;
  always_comb begin q = '0; for (int i = 0; i < 3; i++) begin q[5*i +: 4] = 4'hf; end end
  assign v = s.m[1].s + 0;
  initial #2000 $finish;
  initial begin s = '0; s.m[1].s = -3; #1 $display("A v=%0d s=%b", v, s); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // b5_nested_signed_deep: sv2v, verilator, iverilog `A v0=-2 c0=1`; r1 read 14.
    loud(
        r#"package p;
  typedef struct packed signed { logic [3:0] a; } ss_t;
  typedef struct packed { ss_t s; } mid_t;
  typedef struct packed { mid_t md; logic b; } e5_t;
  typedef struct packed { logic [1:0] pre; e5_t [2:0] m; } r_t;
endpackage
module top; import p::*; r_t s; int v0; logic c0;
  assign v0 = s.m[0].md.s; assign c0 = s.m[0].md.s < 0;
  initial #2000 $finish;
  initial begin s = 17'b10_1010_1_0011_0_1110_1; #1 $display("A v0=%0d c0=%b", v0, c0); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // b3_enum2_x_lit: verilator `A y1=00 ..`; r1 read xx.
    loud(
        r#"package p;
  typedef enum bit [1:0] {A0, A1, A2, A3} k2_t;
  typedef struct packed { k2_t q; logic qe; } e3_t;
  typedef struct packed { logic [3:0] pre; e3_t [3:0] m; logic [2:0] post; } r_t;
  typedef struct packed { logic [3:0] pre; e3_t m3; e3_t m2; e3_t m1; e3_t m0; logic [2:0] post; } t_t;
endpackage
module top; import p::*; r_t s; t_t t; e3_t [3:0] a; logic [1:0] y1, y2, y3; logic e1, e2;
  assign y1 = s.m[1].q; assign y2 = t.m1.q; assign y3 = a[1].q; assign e1 = s.m[1].qe; assign e2 = t.m1.qe;
  initial #2000 $finish;
  initial begin  #1
    $display("A y1=%b y2=%b y3=%b e1=%b e2=%b", y1, y2, y3, e1, e2);
    $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // b5_nested_enum2: verilator `A y=00`; r1 read xx.
    loud(
        r#"package p;
  typedef enum bit [1:0] {A0, A1, A2, A3} k2_t;
  typedef struct packed { k2_t k; } in_t;
  typedef struct packed { in_t i; logic qe; } e3_t;
  typedef struct packed { logic [3:0] pre; e3_t [3:0] m; logic [2:0] post; } r_t;
endpackage
module top; import p::*; r_t s; logic [1:0] y;
  assign y = s.m[1].i.k;
  initial #2000 $finish;
  initial begin #1 $display("A y=%b", y); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // row 41 PLAN2 (lens r2, plan probes): an element no page declares, each one r1c
    // admitted. f01_tp_cast, a type parameter leaf: verilator `A u_lit=00 u_rt=00
    // u_ctl=00 b=2` / `A v_lit=10 v_rt=10 v_ctl=00`; r1c read xx / 1x.
    loud(
        r#"package p;
  typedef enum bit [1:0] { A = 2'(1), B } be_e;
  localparam type T = be_e;
  typedef struct packed { T k; logic e; } e_t;
  typedef struct packed { logic a; e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; e_t t; logic [1:0] x [2];
  always_comb for (int i = 0; i < 2; i++) x[i] = s.m[i].k;
  initial #100 $finish;
  initial begin #1 $display("A u_lit=%b u_rt=%b u_ctl=%b b=%0d", s.m[1].k, x[1], t.k, $bits(T));
    s = 7'b0_1x1_x01; t = 3'bx01; #1 $display("A v_lit=%b v_rt=%b v_ctl=%b", s.m[1].k, x[1], t.k); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // k09_leaf_kw_md10, a leaf of two written dimensions: s2, verilator, iverilog
    // `A b=6 s=000000` (r1c the same).
    loud(
        r#"package p;
  typedef struct packed { logic [1:0][0:0] a; logic e; } e_t;
  typedef struct packed { e_t [1:0] m; } r_t;
endpackage
module top; import p::*; r_t s; logic x;
  initial #100 $finish;
  initial begin s = '0; #1 $display("A b=%0d s=%b", $bits(r_t), s); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // cal/d2, a `$unit` vector of a `[W-1:0]` bound read in the package: iverilog, sv2v,
    // verilator `A bh=17 be=5 v1=0 v0=f s=1fc3f`; r1c read `A bh=29 be=9 v1=00 v0=ff
    // s=1ffc03ff`.
    loud(
        r#"localparam int W = 4;
typedef logic [W-1:0] w_t;
package q;
  localparam int W = 8;
  typedef struct packed { w_t a; logic b; } e_t;
  typedef struct packed { logic [1:0] pre; e_t [2:0] m; } h_t;
endpackage
module top; import q::*; h_t s;
  initial #2000 $finish;
  initial begin s = '1; s.m[1].a = 0; #1 $display("A bh=%0d be=%0d v1=%h v0=%h s=%h", $bits(h_t), $bits(e_t), s.m[1].a, s.m[0].a, s); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // qs6, the element a package vector over a `$unit` struct of its name: s2, verilator,
    // iverilog `A b=13 bm=12 s=1000000000000 x=000000`; r1c read `A b=7 bm=6 ..`.
    loud(
        r#"typedef struct packed { logic [1:0] a; logic b; } e_t;
package p;
  typedef logic [5:0] e_t;
  typedef struct packed { logic pre; e_t [1:0] m; } h_t;
endpackage
module top; import p::*; h_t s; logic [5:0] x;
  initial #100 $finish;
  initial begin s = '0; s.pre = 1'b1; x = s.m[1]; #1 $display("A b=%0d bm=%0d s=%b x=%b", $bits(h_t), $bits(s.m), s, x); $finish; end
endmodule
"#,
        "packed array of a packed struct as a struct member",
    );
    // qs10, the same as an alias base: s2, verilator, iverilog `A o=1100101010111100`;
    // r1c read `A o=0110000000111100`.
    loud(
        r#"typedef struct packed { logic [1:0] a; logic b; } e_t;
package p;
  typedef logic [5:0] e_t;
  typedef e_t [1:0] a_t;
endpackage
module sub(input p::a_t pv, output logic [15:0] o);
  assign o = {4'($bits(pv)), 12'(pv)};
endmodule
module top; logic [11:0] v; logic [15:0] o;
  sub u(.pv(v), .o(o));
  initial #100 $finish;
  initial begin v = 12'hABC; #1 $display("A o=%b", o); $finish; end
endmodule
"#,
        "this packed dimension on a named type",
    );
    // qs5, a body vector over a package struct as the element, in a function body: the
    // holder says what it said on the base (verilator, iverilog `A f=1101111111111111`;
    // r1c read `A f=0111000001111111`).
    loud(
        r#"package p;
  typedef struct packed { logic [1:0] a; logic b; } e_t;
  function automatic logic [15:0] f();
    typedef logic [5:0] e_t;
    typedef struct packed { logic pre; e_t [1:0] m; } h_t;
    h_t s;
    s = '1;
    return {4'($bits(h_t)), 12'(s)};
  endfunction
endpackage
module top; import p::*;
  initial #100 $finish;
  initial begin #1 $display("A f=%b", f()); $finish; end
endmodule
"#,
        "expected identifier, found '['",
    );
}

/// A return base no page declares (lens r2 f08_tp_ret): a type parameter of an enum of a
/// 2-state base. The return refuses by the base's written form (PLAN2 V1'), not by the
/// parameter's recorded kind.
#[test]
fn a_return_base_no_page_declares_is_refused() {
    loud(
        r#"package p;
  typedef enum bit [1:0] { A = 2'(1), B } be_e;
  localparam type T = be_e;
  function automatic T [1:0] f(input logic [3:0] v); return v; endfunction
  function automatic T [1:0] g(); endfunction
endpackage
module top;
  initial #100 $finish;
  initial begin #1 $display("A f=%b g=%b", p::f(4'b1x01), p::g()); $finish; end
endmodule
"#,
        "this packed dimension on a named type",
    );
}

/// A member after a holder in the same struct is its own: the holder's element marks no
/// later member. Expected: s2+vl+iv (lens r1 h21_after_holder).
#[test]
fn a_member_after_a_holder_is_its_own() {
    value(
        r#"package p;
  typedef struct packed { logic [1:0] q; logic e; } e_t;
  typedef struct packed { e_t [1:0] m; logic [3:0] z; } r_t;
endpackage
module top; import p::*; r_t s;
  initial #100 $finish;
  initial begin s = 10'b101_011_1001; #1 $display("A z1=%b z=%b zs=%b m1=%b", s.z[1], s.z, s.z[2:1], s.m[1]); $finish; end
endmodule
"#,
        &["A z1=0 z=1001 zs=00 m1=101"],
    );
}
