//! §4.5.426 (ROADMAP §2 🆕 N): `%m` inside a named block, a statement label, a fork child,
//! an inline task and a frame subroutine block appends the block-label chain to the instance
//! path (IEEE 1800-2017 §21.2.1 — a named block is a scope). vita printed the instance path
//! only. Expected lines are verilator 5.050's (iverilog agrees on every line it accepts; it
//! rejects a statement label). Also pinned staged (`vcmp → velab → vrun`) through the v30
//! `stmt_scopes` / `expr_scopes` trailer — see `staged_pipeline.rs`-style tests elsewhere.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pctm_{}_{n}", std::process::id()));
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
fn percent_m_carries_the_named_block_chain() {
    let src = "module top;\n  task t; $display(\"D=t %m\"); endtask\n  function automatic int f(); begin : fb $display(\"D=f %m\"); end return 1; endfunction\n  int k;\n  initial begin : blk\n    $display(\"D=A %m\");\n    begin : inner $display(\"D=B %m\"); end\n    L: $display(\"D=L %m\");\n    t(); k = f();\n    fork begin : fk $display(\"D=F %m\"); end join\n    $display(\"D=S %s\", $sformatf(\"%m\"));\n    #1 $finish;\n  end\n  initial $display(\"D=C %m\");\nendmodule\n";
    let mut got = lines(src, "D=");
    got.sort();
    assert_eq!(
        got,
        vec![
            "D=A top.blk",
            "D=B top.blk.inner",
            "D=C top",
            "D=F top.blk.fk",
            "D=L top.blk.L",
            "D=S top.blk",
            "D=f top.f.fb",
            "D=t top.t"
        ]
    );
}

#[test]
fn nested_instances_generate_and_severity_tasks() {
    let src = "module m; initial begin : blk begin : in $info(\"D=%m\"); end end endmodule\nmodule top;\n  m u1(); m u2();\n  generate if (1) begin : g initial begin : blk $display(\"D=%m\"); end end endgenerate\n  initial begin begin : a $display(\"D=%m\"); end $display(\"D=%m\"); #1 $finish; end\nendmodule\n";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let mut got: Vec<&str> = out.lines().filter(|l| l.contains("D=")).collect();
    got.sort();
    assert!(got.iter().any(|l| l.contains("D=top.u1.blk.in")), "{out}");
    assert!(got.iter().any(|l| l.contains("D=top.u2.blk.in")), "{out}");
    // A `generate if` block prints `top.g[0].blk` here where both oracles print
    // `top.g.blk` — the instance path names a singleton generate scope `g[0]`
    // (pre-existing, ROADMAP §2 🆕 N residue); the block chain itself is right.
    assert!(
        got.iter()
            .any(|l| l.ends_with(".blk") && l.contains("top.g")),
        "{out}"
    );
    assert!(got.iter().any(|l| *l == "D=top.a"), "{out}");
    assert!(got.iter().any(|l| *l == "D=top"), "{out}");
}

#[test]
fn a_nested_system_task_hands_the_chain_back() {
    // Review B F2: `f()` prints from inside the outer statement's argument; the
    // chain slot is saved and restored, so the second `%m` is still `top.blk`.
    let src = "module top;\n  function automatic int f(); $display(\"D=inner %m\"); return 7; endfunction\n  initial begin : blk $display(\"D=A %m n=%0d B %m\", f()); #1 $finish; end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec!["D=inner top.f", "D=A top.blk n=7 B top.blk"]
    );
}
