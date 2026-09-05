//! §4.5.429 (ROADMAP §2 🆕 N residue): `%m` names a SINGLETON generate scope
//! (`generate if` / `case` / a nested one) without the `[0]` vita stores it under, and a
//! fork's own label (`F: fork … join`) is a scope segment. A generate-for iteration keeps
//! its index. Expected lines are verilator 5.050's (iverilog agrees on the generate lines
//! and rejects a statement label).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pctmgen_{}_{n}", std::process::id()));
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
fn singleton_generate_scopes_print_without_an_index() {
    let src = "module m; initial begin : blk $display(\"D=%m\"); end endmodule\nmodule top;\n  genvar i;\n  generate\n    if (1) begin : gi initial $display(\"D=%m\"); m u(); end\n    for (i = 0; i < 2; i++) begin : gl initial $display(\"D=%m\"); end\n    case (1) 1: begin : gc initial $display(\"D=%m\"); end endcase\n    if (1) begin : go if (1) begin : gn initial $display(\"D=%m\"); end end\n  endgenerate\n  initial begin F: fork $display(\"D=%m\"); join #1 $finish; end\nendmodule\n";
    let mut got = lines(src, "D=");
    got.sort();
    assert_eq!(
        got,
        vec![
            "D=top.F",
            "D=top.gc",
            "D=top.gi",
            "D=top.gi.u.blk",
            "D=top.gl[0]",
            "D=top.gl[1]",
            "D=top.go.gn"
        ]
    );
}

#[test]
fn a_one_iteration_loop_keeps_its_index_and_the_hierarchical_read_is_unchanged() {
    // The storage key stays `gi[0]`: a bare `gi.x` read still resolves; a loop that
    // runs once is still an array (`gl[0]`, IEEE §27.4).
    let src = "module top;\n  genvar i;\n  generate if (1) begin : gi logic [3:0] x = 4'h5; end endgenerate\n  generate for (i = 0; i < 1; i++) begin : gl initial $display(\"D=%m\"); end endgenerate\n  initial begin #1 $display(\"D=%h %h\", gi.x, top.gi.x); #1 $finish; end\nendmodule\n";
    let mut got = lines(src, "D=");
    got.sort();
    assert_eq!(got, vec!["D=5 5", "D=top.gl[0]"]);
}

#[test]
fn an_instance_array_element_keeps_its_index() {
    // Review B B1: `w[0]` is an instance-ARRAY element, not a singleton generate
    // scope — both oracles print `top.w[0]` / `top.w[1]`; a first draft printed `top.w`.
    let src = "module ch (input logic a); initial $display(\"D=%m\"); endmodule\nmodule top;\n  logic [1:0] x = 2'b01;\n  ch w[1:0](.a(x));\n  generate if (1) begin : gi ch u(.a(x[0])); end endgenerate\n  initial #1 $finish;\nendmodule\n";
    let mut got = lines(src, "D=");
    got.sort();
    assert_eq!(got, vec!["D=top.gi.u", "D=top.w[0]", "D=top.w[1]"]);
}
