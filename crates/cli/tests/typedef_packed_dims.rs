//! §4.5.425: packed dimensions after a typedef name (`cfg_t [N-1:0] p`, `pkg::t [pkg::C-1:0] p`
//! — the ibex_top_tracing port shape) in ANSI ports and variable declarations, with member
//! access on a packed element; and a struct-typed port ARRAY whose element member is read
//! with a non-constant index (`c[r].mode` in a generate loop — the ibex_pmp shape — was
//! parsed as a generate-array hierarchical reference). Expected lines are the output of
//! iverilog 13.0 and verilator 5.050 (they agree).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tddims_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
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

fn lines(src: &str, prefix: &str) -> Vec<String> {
    let (out, rc) = run(src);
    assert_eq!(
        rc,
        Some(0),
        "expected exit 0, got {rc:?}:
{out}"
    );
    out.lines()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.to_string())
        .collect()
}

const PK: &str = "package pk; typedef struct packed { logic [1:0] mode; logic x; } cfg_t; localparam int N = 2; endpackage\n";

#[test]
fn scoped_typedef_with_packed_dims_on_a_port_and_a_declaration() {
    let src = format!("{PK}module m (\n  input  pk::cfg_t [pk::N-1:0] r,\n  output logic [pk::N-1:0] o\n);\n  assign o = {{r[1].x, r[0].x}};\nendmodule\nmodule top;\n  pk::cfg_t [pk::N-1:0] r; logic [pk::N-1:0] o;\n  m u(.r(r), .o(o));\n  initial begin r = 6'b101_010; #1 $display(\"D=%b %h %0d %b\", o, r, $bits(r), r[1].mode); #1 $finish; end\nendmodule\n");
    assert_eq!(lines(&src, "D="), vec!["D=10 2a 6 10"]);
}

#[test]
fn every_spelling_of_a_struct_array_element_member() {
    let src = format!("{PK}module a1 (input pk::cfg_t c [2]); initial begin #1 $display(\"D1=%b\", c[0].mode); end endmodule\nmodule a2 import pk::*; (input cfg_t c [2]); initial begin #1 $display(\"D2=%b\", c[1].mode); end endmodule\nmodule a3 (input pk::cfg_t c [2]); for (genvar r = 0; r < 2; r++) begin : g initial begin #1 $display(\"D3=%b\", c[r].mode); end end endmodule\nmodule a4; import pk::*; cfg_t c [2]; initial begin c[0] = 3'b011; c[1] = 3'b100; #1 $display(\"D4=%b %b\", c[0].mode, c[1].x); end endmodule\nmodule a5; pk::cfg_t c [2]; initial begin c[0] = 3'b011; #1 $display(\"D5=%b\", c[0].mode); end endmodule\nmodule a6; typedef struct packed {{ logic [1:0] mode; logic x; }} l_t; l_t [1:0] r; l_t q [2]; initial begin r = 6'b011100; q[1] = 3'b110; #1 $display(\"D6=%b %b\", r[1].mode, q[1].mode); end endmodule\nmodule top;\n  pk::cfg_t c [2]; a1 u1(.c(c)); a2 u2(.c(c)); a3 u3(.c(c)); a4 u4(); a5 u5(); a6 u6();\n  initial begin c[0] = 3'b011; c[1] = 3'b100; #2 $finish; end\nendmodule\n");
    let mut got = lines(&src, "D");
    got.sort();
    assert_eq!(
        got,
        vec!["D1=01", "D2=10", "D3=01", "D3=10", "D4=01 0", "D5=01", "D6=01 11"]
    );
}

#[test]
fn runtime_and_genvar_indexed_member_in_assign_and_initial() {
    let src = format!("{PK}module b4 (input pk::cfg_t c [2]); for (genvar r = 0; r < 2; r++) begin : g logic m; assign m = (c[r].mode == 2'd1); initial begin #1 $display(\"D4=%b\", m); end end endmodule\nmodule b5 (input pk::cfg_t c [2]); int i = 1; initial begin #1 $display(\"D5=%b\", c[i].mode); end endmodule\nmodule top;\n  pk::cfg_t c [2]; b4 u4(.c(c)); b5 u5(.c(c));\n  initial begin c[0] = 3'b011; c[1] = 3'b100; #2 $finish; end\nendmodule\n");
    let mut got = lines(&src, "D");
    got.sort();
    assert_eq!(got, vec!["D4=0", "D4=1", "D5=10"]);
}

#[test]
fn two_packed_dims_after_a_struct_typedef_is_loud_and_a_continuation_port_binds() {
    // Review B F1: with TWO dims `r[1]` is a sub-array, not the struct — both oracles
    // refuse `r[1].mode`; PRE refused the declaration. Loud, not a value.
    let (out, rc) = run(&format!("{PK}module top;\n  pk::cfg_t [1:0][2:0] r;\n  initial begin r = '0; $display(\"D=%b\", r[1].mode); #1 $finish; end\nendmodule\n"));
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("E3010") || out.contains("E2002"), "{out}");
    // Review B F3: `input pk::cfg_t [1:0] a, b` — `b` is the same packed struct array.
    let src = format!("{PK}module m (input pk::cfg_t [1:0] a, b, output logic [2:0] o);\n  assign o = {{b[0].mode, a[1].x}};\nendmodule\nmodule top;\n  pk::cfg_t [1:0] a, b; logic [2:0] o;\n  m u(.a(a), .b(b), .o(o));\n  initial begin a = 6'b101_011; b = 6'b110_001; #1 $display(\"D=%b %0d %0d\", o, $bits(a), $bits(b)); #1 $finish; end\nendmodule\n");
    assert_eq!(lines(&src, "D="), vec!["D=001 6 6"]);
}

#[test]
fn a_signed_typedef_element_with_packed_dims_is_loud() {
    // Review A F1: `typedef logic signed [3:0] s_t; s_t [1:0] v;` — the flat net is
    // unsigned and each ELEMENT is signed (both oracles: `v[1]` = −2, a signed
    // accumulator over `samp_t [3:0]` sums 0); the flat representation cannot carry a
    // per-element sign, so the shape is refused (PRE refused it too).
    let (out, rc) = run("module top;\n  typedef logic signed [3:0] s_t; s_t [1:0] v;\n  initial begin v = 8'b1110_0001; $display(\"D=%0d\", v[1]); #1 $finish; end\nendmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("SIGNED typedef as the element"), "{out}");
}
