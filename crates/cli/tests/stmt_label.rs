//! §4.5.422: a statement label `L: stmt` (IEEE 1800-2017 §9.3.5) parses as a named
//! block around the statement — `L: begin … end` is `begin : L … end` by the LRM's own
//! equivalence, and a `disable L` ends the labelled statement. lowRISC's
//! `ASSERT_INIT(name, prop)` expands to `initial begin name: assert (prop) else begin …
//! end end`; thirty of them in ibex were `E2002 expected '=' or '<=' after lvalue`.
//! Expected lines are verilator 5.050's (iverilog 13 accepts a label on an immediate
//! assertion only). The `%m` inside a labelled statement is NOT pinned: vita prints the
//! instance scope for every named block (pre-existing, ROADMAP §2 🆕 N).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stlbl_{}_{n}", std::process::id()));
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

#[test]
fn a_label_on_every_statement_kind() {
    let src = "module top;\n  int n = 0;\n  initial begin\n    L1: assert (1) else $display(\"D=fail L1\");\n    L2: assert (0) else $display(\"D=L2 fail\");\n    L3: assert (1) $display(\"D=P\");\n    L4: begin $display(\"D=in L4\"); end\n    L5: $display(\"D=L5\");\n    L6: if (1) $display(\"D=L6\"); else $display(\"D=E\");\n    L7: fork $display(\"D=F1\"); join\n    L8: n = 5; $display(\"D=%0d\", n);\n    L9: case (n) 5: $display(\"D=five\"); default: $display(\"D=d\"); endcase\n    #10 $finish;\n  end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec![
            "D=L2 fail",
            "D=P",
            "D=in L4",
            "D=L5",
            "D=L6",
            "D=F1",
            "D=5",
            "D=five"
        ]
    );
}

#[test]
fn disable_of_a_labelled_loop_ends_it() {
    // verilator: `n=2` — the loop body ran for i = 0 and i = 1.
    let src = "module top;\n  int n = 0; int k = 0;\n  initial begin : blk\n    L2: assert (1) $display(\"D=pass L2\");\n    L4: for (int i = 0; i < 3; i++) begin n++; if (i == 1) disable L4; end\n    $display(\"D=n=%0d\", n);\n    L5: while (k < 10) begin k++; if (k == 4) disable L5; end\n    $display(\"D=k=%0d\", k);\n    L6: begin M: begin disable L6; end $display(\"D=never\"); end $display(\"D=after\");\n    #10 $finish;\n  end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec!["D=pass L2", "D=n=2", "D=k=4", "D=after"]
    );
}

#[test]
fn ibex_assert_init_shape() {
    // The lowRISC standard macro body, with the `$error` replaced by a `$display` so
    // the line is comparable across tools; the failing one is the second.
    let src = "module top;\n  localparam logic [3:0] IbexMuBiOn = 4'b0101;\n  initial begin\n    IbexMuBiSecureOnBottomBitSet: assert (IbexMuBiOn[0] == 1'b1)\n      else begin\n        $display(\"D=[ASSERT FAILED] %0s\", \"IbexMuBiSecureOnBottomBitSet\");\n      end\n  end\n  initial begin\n    IllegalParam: assert (IbexMuBiOn[1] == 1'b1)\n      else begin\n        $display(\"D=[ASSERT FAILED] %0s\", \"IllegalParam\");\n      end\n  end\n  initial #5 $finish;\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=[ASSERT FAILED] IllegalParam"]);
}

#[test]
fn a_statement_label_and_a_block_label_on_one_block_is_loud() {
    let (out, rc) = run(
        "module top;\n  initial begin\n    L1: begin : M $display(\"x\"); end\n  end\nendmodule\n",
    );
    assert_eq!(rc, Some(1), "{out}");
    assert!(
        out.contains("E2002") && out.contains("one label on a block"),
        "{out}"
    );
}

#[test]
fn break_inside_a_labelled_loop() {
    // Review A-1: `break` wraps the loop in a synthetic `$break$` block; the user's
    // label goes around it instead of colliding with it. verilator: 2 / 6 3.
    let src = "module top;\n  int n = 0; int a = 0; int b = 0;\n  initial begin\n    L: for (int i = 0; i < 5; i++) begin if (i == 2) break; n++; end\n    $display(\"D=%0d\", n);\n    M: begin\n      K: while (a < 10) begin a++; if (a == 6) break; end\n      J: for (int i = 0; i < 9; i++) begin b++; if (b == 3) break; end\n    end\n    $display(\"D=%0d %0d\", a, b);\n    #10 $finish;\n  end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=2", "D=6 3"]);
}
