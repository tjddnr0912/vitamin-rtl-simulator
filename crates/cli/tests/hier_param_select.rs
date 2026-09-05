//! §4.5.422 (ROADMAP §2 🆕 M ⓒ / ⓓ): a bit / part-select and `$bits` of a
//! HIERARCHICAL parameter (`u.K[7:0]`, `u.K[0]`, `u.K[b+:w]`, `$bits(u.K)`), `$bits`
//! of a hierarchical net, and a bitwise `& | ^` expression override of a >64-bit
//! parameter. All were loud at every width (`a bit / part-select of the hierarchical
//! parameter … is unsupported`, `$bits argument shape unsupported`, `W3056 override …
//! is not a constant`). Expected lines are the output of iverilog 13.0 and verilator
//! 5.050 (they agree).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hpsel_{}_{n}", std::process::id()));
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

const M: &str = "module m #(parameter logic [63:0] K = 64'hdead_beef_0000_1111, parameter logic [11:4] N = 8'h3C, parameter logic [127:0] W = 128'hdead_beef_0000_1111_2222_3333_4444_5555) ();\n  logic [15:0] n = 16'habcd;\nendmodule\n";

#[test]
fn select_and_bits_of_a_hierarchical_parameter() {
    let src = format!("{M}module top;\n  m u();\n  int i = 1;\n  initial begin\n    $display(\"D=%0d %h %h\", $bits(u.n), u.n[3:0], u.n[i]);\n    $display(\"D=%h %h %h\", u.N[7:4], u.N[11], u.N[5+:2]);\n    $display(\"D=%h %h %h\", u.W[71:64], u.W[127], u.W[7+:8]);\n    $display(\"D=%h %h %0d\", u.K[7:0], u.K[i], $bits(u.K));\n    $display(\"D=%h %0d %0d\", u.W[7:0], $bits(u.W), $bits(u.N));\n    #1 $finish;\n  end\nendmodule\n");
    assert_eq!(
        lines(&src, "D="),
        vec![
            "D=16 d 0",
            "D=c 0 2",
            "D=11 1 aa",
            "D=11 0 64",
            "D=55 128 8"
        ]
    );
}

#[test]
fn bitwise_expression_override_of_a_wide_parameter() {
    let src = "module m #(parameter logic [127:0] W = 128'h1) (); endmodule\nmodule top;\n  localparam logic [127:0] PK = 128'hdead_beef_0000_1111_2222_3333_4444_5555;\n  m #(.W(128'hdead_beef_0000_1111_2222_3333_4444_5555 ^ 128'd3)) u2();\n  m #(.W(PK & 128'hffff_ffff)) u3();\n  initial begin\n    $display(\"D=%h %0d\", u2.W, $bits(u2.W));\n    $display(\"D=%h\", u3.W);\n    #1 $finish;\n  end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec![
            "D=deadbeef000011112222333344445556 128",
            "D=00000000000000000000000044445555"
        ]
    );
}

#[test]
fn a_carrying_wide_override_stays_loud() {
    // `~` and `+` carry past the operands' top bit; folding them in the parent at the
    // operands' width would be a different value at the child's — loud, as before.
    let (out, rc) = run("module m #(parameter logic [255:0] W = 256'h1) (); endmodule\nmodule top;\n  m #(.W(~128'hdead_beef_0000_1111_2222_3333_4444_5555)) u();\n  initial begin $display(\"%h\", u.W); #1 $finish; end\nendmodule\n");
    assert_eq!(rc, Some(1), "{out}");
    assert!(out.contains("E3009"), "{out}");
}
