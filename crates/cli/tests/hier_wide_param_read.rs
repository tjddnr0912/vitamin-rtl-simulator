//! §4.5.421 (ROADMAP §2 🆕 G / row 28): a parameter wider than 64 bits read through a
//! hierarchical path (`u.C`) is the same `Const` a bare read lowers to. It was
//! `E3010 undeclared hierarchical name` — a false message on a valid design — because
//! the hierarchical resolver consulted the i64 table only. Expected lines are the output
//! of iverilog 13.0 and verilator 5.050 (they agree).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hwpr_{}_{n}", std::process::id()));
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

const DESIGN: &str = "module sub; localparam logic [127:0] K = 128'h1 - 128'h3; localparam logic [63:0] K64 = 64'h1 - 64'h3; endmodule\nmodule m #(parameter logic [127:0] C = ~32'd0, parameter logic [95:0] D = 96'hffff_0000_1234_5678_9abc_def0)(); endmodule\nmodule w; localparam logic [65:0] ONLY = 66'h3_ffff_ffff_ffff_ffff; endmodule\nmodule top;\n  sub s(); m u(); m #(.C(128'h5555_aaaa_5555_aaaa_5555_aaaa_5555_aaaa)) u2(); w o();\n  logic [127:0] x;\n  initial begin\n    $display(\"D=%h\", s.K); $display(\"D=%h\", s.K64); $display(\"D=%h\", u.C); $display(\"D=%h\", u.D);\n    $display(\"D=%h\", u2.C); $display(\"D=%h\", o.ONLY);\n    x = u.C + 1; $display(\"D=%h\", x);\n    $display(\"D=%0d\", u.C == 128'hffffffffffffffffffffffffffffffff);\n    #1 $finish;\n  end\nendmodule\n";

#[test]
fn wide_parameter_reads_hierarchically() {
    assert_eq!(
        lines(DESIGN, "D="),
        vec![
            "D=fffffffffffffffffffffffffffffffe",
            "D=fffffffffffffffe",
            "D=ffffffffffffffffffffffffffffffff",
            "D=ffff0000123456789abcdef0",
            "D=5555aaaa5555aaaa5555aaaa5555aaaa",
            "D=3ffffffffffffffff",
            "D=00000000000000000000000000000000",
            "D=1",
        ]
    );
}

#[test]
fn wide_parameter_in_a_continuous_assign_and_nested_path() {
    let src = "module n; localparam logic [95:0] K = 96'hdead_beef_0000_1111_2222_3333; endmodule\nmodule m; n v(); endmodule\nmodule top;\n  m u();\n  logic [95:0] y; assign y = u.v.K;\n  initial begin #1 $display(\"D=%h\", y); #1 $finish; end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=deadbeef0000111122223333"]);
}

#[test]
fn declared_wide_value_narrow_reads_at_declared_width() {
    // both oracles: the bare read and the hierarchical read agree at 128 bits
    let src = "module sub; localparam logic [127:0] SMALL = 128'h7; localparam logic signed [95:0] NEG = -96'sd2; endmodule\nmodule top;\n  sub u();\n  initial begin $display(\"D=%h\", u.SMALL); $display(\"D=%h\", u.NEG); #1 $finish; end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec![
            "D=00000000000000000000000000000007",
            "D=fffffffffffffffffffffffe"
        ]
    );
}

#[test]
fn select_of_a_hierarchical_parameter_is_loud_with_an_honest_message() {
    let src = "module sub; localparam logic [127:0] K = 128'h1; endmodule\nmodule top;\n  sub u();\n  initial begin $display(\"%h\", u.K[7:0]); #1 $finish; end\nendmodule\n";
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("hierarchical parameter `u.K`"), "{out}");
    assert!(!out.contains("undeclared"), "{out}");
}
