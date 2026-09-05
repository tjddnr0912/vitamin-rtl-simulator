//! §4.5.430 (ROADMAP §2 🆕 L ⓝ): `$bits` of a block-local / function-local variable
//! that shadows a wildcard-imported package constant answers the LOCAL's width (it
//! answered the constant's — `bits_of_view` consulted the constant table before the
//! shadow test the value read already applies); an unpacked local array is its element
//! width × count. Expected lines are the output of iverilog 13.0 and verilator 5.050
//! (they agree).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bitsshadow_{}_{n}", std::process::id()));
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

#[test]
fn bits_of_a_shadowing_local() {
    let src = "package p; parameter logic [7:0] P = 8'h5A; parameter logic [3:0] Q = 4'h3; endpackage\nmodule top;\n  import p::*;\n  initial begin : b\n    logic [19:0] P; P = 20'h12345;\n    $display(\"D=%0d %0d %h %0d\", $bits(P), $size(P), P, $left(P));\n  end\n  initial begin : c\n    logic [11:0] Q [2];\n    $display(\"D=%0d %0d %0d\", $bits(Q), $size(Q), $bits(Q[0]));\n  end\n  function automatic int f(); logic [5:0] P; return $bits(P); endfunction\n  initial begin #1 $display(\"D=%0d %0d\", f(), $bits(P)); #1 $finish; end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec!["D=20 20 12345 19", "D=24 2 12", "D=6 8"]
    );
}
