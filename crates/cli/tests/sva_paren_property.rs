//! §4.5.419: a property wrapped whole in parentheses — `assert property (@(posedge c)
//! disable iff (r) (a |-> b))`, the shape every lowRISC `ASSERT` macro expands to —
//! parses and lowers exactly as the unwrapped body. Pinned against verilator 5.050
//! (`--assert`; iverilog 13.0 rejects `|->`): the same failure times on both
//! spellings, double parentheses, an `else begin … end` action block, a named
//! property's `;` form, `cover property`, and the tree-path shapes that were already
//! parsing. A group that is NOT the whole property stays a parse error.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svapp_{}_{n}", std::process::id()));
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
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    out.lines()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.to_string())
        .collect()
}

const TB_HEAD: &str = "module top;\n  logic clk = 0, rst_n = 0, a = 0, b = 0;\n  always #5 clk = ~clk;\n  initial begin #12 rst_n = 1; #10 a = 1; b = 1; #10 a = 1; b = 0; #10 a = 0; #20 $finish; end\n";

fn tb(items: &str) -> String {
    format!("{TB_HEAD}{items}\nendmodule\n")
}

fn fails(items: &str) -> Vec<String> {
    let mut v = lines(&tb(items), "P");
    v.sort();
    v
}

#[test]
fn paren_and_flat_agree_with_verilator() {
    // verilator: P1 35 · P2 35,45 · P4 35 · P5 35 · P6 35 · P7 none
    for (flat, par, want) in [
        ("a |-> b", "(a |-> b)", vec!["P fail t=35"]),
        ("a |=> b", "(a |=> b)", vec!["P fail t=35", "P fail t=45"]),
        (
            "a |-> b inside {1'b1}",
            "(a |-> b inside {1'b1})",
            vec!["P fail t=35"],
        ),
        ("(a) |-> (b)", "((a) |-> (b))", vec!["P fail t=35"]),
        ("a & b |-> b", "(a & b |-> b)", vec![]),
        ("a |-> b", "((a |-> b))", vec!["P fail t=35"]),
    ] {
        let want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
        let f = format!("  P: assert property (@(posedge clk) disable iff ((!rst_n) !== '0) {flat}) else $display(\"P fail t=%0t\", $time);");
        let p = format!("  P: assert property (@(posedge clk) disable iff ((!rst_n) !== '0) {par}) else begin $display(\"P fail t=%0t\", $time); end");
        assert_eq!(fails(&f), want, "flat {flat}");
        assert_eq!(fails(&p), want, "paren {par}");
    }
}

#[test]
fn paren_without_disable_iff_and_named_property() {
    // verilator: 35 for every spelling
    let want = vec!["P fail t=35".to_string()];
    assert_eq!(fails("  P: assert property (@(posedge clk) (a |-> b)) else $display(\"P fail t=%0t\", $time);"), want);
    assert_eq!(fails("  property pp; @(posedge clk) disable iff (!rst_n) (a |-> b); endproperty\n  P: assert property (pp) else $display(\"P fail t=%0t\", $time);"), want);
    assert_eq!(fails("  P: assume property (@(posedge clk) disable iff (!rst_n) (a |-> b)) else $display(\"P fail t=%0t\", $time);"), want);
}

#[test]
fn tree_path_shapes_unchanged() {
    // property-level `or` without `disable iff`: parsed by the tree path before and after
    let (out, rc) = run(&tb("  P: assert property (@(posedge clk) (a |-> b) or (b |-> a)) else $display(\"P fail t=%0t\", $time);"));
    assert_eq!(rc, Some(0), "{out}");
    assert!(!out.contains("P fail"), "{out}");
}

#[test]
fn ibex_assert_macro_shape() {
    let src = "`define ASSERT(__name, __prop, __clk = clk, __rst = !rst_n) \\\n  __name: assert property (@(posedge __clk) disable iff ((__rst) !== '0) (__prop)) \\\n    else begin \\\n      $display(\"P fail t=%0t\", $time); \\\n    end\n";
    let items = "  `ASSERT(AImpliesB, a |-> b)";
    let mut v = lines(&format!("{src}{TB_HEAD}{items}\nendmodule\n"), "P fail");
    v.sort();
    assert_eq!(v, vec!["P fail t=35".to_string()]);
}

#[test]
fn group_that_is_not_the_whole_property_stays_loud() {
    for body in ["(a |-> b) ##1 b", "(a |-> b) |-> b"] {
        let (out, rc) = run(&tb(&format!("  P: assert property (@(posedge clk) disable iff (!rst_n) {body}) else $display(\"x\");")));
        assert_ne!(rc, Some(0), "{body}:\n{out}");
        assert!(out.contains("VITA-E2002"), "{body}:\n{out}");
    }
}
