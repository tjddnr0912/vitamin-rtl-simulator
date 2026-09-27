//! A select of a parameter WIDER than 64 bits reads its DECLARED range where the scope
//! of the parameter is named — a package constant through `pkg::`, and a hierarchical
//! `u.P[…]` — and an override past 64 bits keeps its value on a `[w-1:0]` declaration.
//!
//! The value of a >64-bit parameter lives in `wide_param_bits` with no record of its
//! declaration, so `u.P[15:8]` on a child's `logic [79:8] P` read the stored `[71:0]`
//! bits positionally (`68` for `69`, `P[79:72]` `xx` for `61`), and so did the constant
//! lanes of `pk::P[15:8]`. A bare-name select of such a parameter still does (ROADMAP §2:
//! the select resolvers must not see a >64-bit binding until code folded outside its
//! declaring scope resolves there — §4.5.560's reverted axis). Values: iverilog 13.0 and
//! verilator 5.052 agree on every line. `PRE` is vita before §4.5.560.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, args: &[&str]) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_wpsdr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    v.sort();
    v
}

fn lines(src: &str) -> Vec<String> {
    run(src, &[])
}

/// A package constant through `pkg::`: an untyped `localparam` (value and `$bits`), a
/// range bound and a generate condition over a 72-bit `[79:8]` constant, and a
/// `[135:8]` constant whose value fits 64 bits. PRE: `68`, 104 bits, the `else` branch,
/// and E3009 on the fitting one.
#[test]
fn a_package_constant_select_reads_the_declared_bits() {
    let src = "package pk;\n  localparam logic [79:8] P = 72'h616263646566676869;\n  localparam \
               [135:8] K = 128'hDD_0000;\nendpackage\nmodule t;\n  localparam X = \
               pk::P[15:8];\n  localparam Y = pk::K[31:24];\n  logic [pk::P[15:8]-1:0] v;\n  if \
               (pk::P[15:8] == 8'h69) begin : yes\n    initial #1 $display(\"T yes\");\n  end \
               else begin : no\n    initial #1 $display(\"T no\");\n  end\n  initial begin #2 \
               $display(\"T %h %0d %0d %h %0d\", X, $bits(X), $bits(v), Y, $bits(Y)); $finish; \
               end\nendmodule\n";
    assert_eq!(lines(src), vec!["T 69 8 105 dd 8", "T yes"], "{src}");
}

/// A hierarchical select of a child's >64-bit parameter, on its default, a numeric
/// override and `defparam`. PRE: `xx 68 xx` on each.
#[test]
fn a_hierarchical_select_reads_the_declared_bits() {
    let src = "module s #(parameter logic [79:8] P = 72'h616263646566676869) ();\nendmodule\n\
               module t;\n  s u();\n  s #(.P(72'h717273747576777879)) v();\n  s w();\n  \
               defparam w.P = 72'h414243444546474849;\n  initial begin #1\n    $display(\"T u \
               %h %h %h\", u.P[79:72], u.P[15:8], u.P[23:16]);\n    $display(\"T v %h %h %h\", \
               v.P[79:72], v.P[15:8], v.P[23:16]);\n    $display(\"T w %h %h %h\", w.P[79:72], \
               w.P[15:8], w.P[23:16]);\n    $finish; end\nendmodule\n";
    assert_eq!(
        lines(src),
        vec!["T u 61 69 68", "T v 71 79 78", "T w 41 49 48"],
        "{src}"
    );
}

/// An instance array folds its child's port widths with the child's header parameters
/// bound at the PARENT's prefix; nothing of that binding may outlive it. PRE: the
/// child's `[23:16] P = 8'hc3` answered the parent's `dut.P` (`c3 xx x`).
#[test]
fn an_instance_array_leaves_the_parents_parameters_alone() {
    let src = "module s #(parameter logic [23:16] P = 8'hc3) ();\nendmodule\nmodule d;\n  \
               localparam logic [15:8] P = 8'h5a;\n  s u[1:0] ();\nendmodule\nmodule t;\n  d \
               dut();\n  initial begin #1 $display(\"T %h %h %h\", dut.P, dut.P[15:8], \
               dut.P[11:8]); $finish; end\nendmodule\n";
    assert_eq!(lines(src), vec!["T 5a 5a a"], "{src}");
}

/// An override past 64 bits whose i64 low word would sign-extend differently from the
/// value — bit 63 set in a positive value of a signed `[127:0]` declaration — keeps the
/// value. PRE: `ffffffffffffffff8000000000000000` and `ff` for each.
#[test]
fn a_signed_wide_override_keeps_its_value() {
    let src = "module s #(parameter signed [127:0] K = 0) ();\n  localparam X = K[127:120];\n  \
               initial #1 $display(\"T %m %h %h %h\", K, K[127:120], X);\nendmodule\nmodule t;\n  \
               s #(.K(128'h0000_0000_0000_0000_8000_0000_0000_0000)) u();\n  s d();\n  defparam \
               d.K = 128'h0000_0000_0000_0000_8000_0000_0000_0001;\n  initial #3 \
               $finish;\nendmodule\n";
    assert_eq!(
        lines(src),
        vec![
            "T t.d 00000000000000008000000000000001 00 00",
            "T t.u 00000000000000008000000000000000 00 00"
        ],
        "{src}"
    );
    let g = "module t #(parameter signed [127:0] K = 0);\n  initial begin #1 $display(\"T %h \
             %h\", K, K[127:120]); $finish; end\nendmodule\n";
    assert_eq!(
        run(g, &["-G", "K=128'h00000000000000008000000000000000"]),
        vec!["T 00000000000000008000000000000000 00"],
        "{g}"
    );
}
