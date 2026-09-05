//! §4.5.427 (ROADMAP §2 🆕 L ⓞ): a part-select on a NON-innermost dimension of a partially
//! indexed multi-dim packed variable selects whole sub-elements (`logic [1:0][2:0][1:0] v;
//! v[1][2:1]` is four bits, `1011`); vita read two flat bits of the element. Read, `+:`/`-:`,
//! `$bits`, non-zero-LSB and ascending dims, a runtime outer index, and the write twin.
//! Expected lines are the output of iverilog 13.0 and verilator 5.050 (they agree).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkin_{}_{n}", std::process::id()));
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

const DECL: &str = "logic [1:0][2:0][1:0] v = 12'b10_11_01_00_11_10; logic [3:1][2:0][1:0] z = 18'b10_11_01_00_11_10_01_10_11; int i = 1;";

#[test]
fn inner_dimension_part_select_reads() {
    let src = format!("module top;\n  {DECL}\n  initial begin\n    $display(\"D=%b %b %b %b\", v[1][2:1], v[1][2], v[1][2][1:0], v[1:0]);\n    $display(\"D=%b %b %0d\", v[1][1+:2], v[1][2-:2], $bits(v[1][2:1]));\n    $display(\"D=%b %b %b\", z[2][2:1], z[3][1:0], v[i][2:1]);\n    #1 $finish;\n  end\nendmodule\n");
    assert_eq!(
        lines(&src, "D="),
        vec![
            "D=1011 10 10 101101001110",
            "D=1011 1011 4",
            "D=0011 1101 1011"
        ]
    );
}

#[test]
fn inner_dimension_part_select_write() {
    let src = "module top;\n  logic [1:0][2:0][1:0] w;\n  initial begin w = 0; w[1][2:1] = 4'b1001; $display(\"D=%b\", w); #1 $finish; end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=100100000000"]);
}
