//! r18 (family D): an `automatic` block-local WITH AN INITIALIZER (or read-before-first-
//! write) inside a MODULE process is now correct-support for the non-concurrent case — the
//! initializer RE-RUNS on each block entry (IEEE §6.21 automatic lifetime) instead of once
//! at t0. Was E3009 "…per-entry lifetime differs from static…".
//!
//! SOUND on the single flattened net because a module process's loops are sequential (one
//! activation live at a time). ONLY a `fork` ancestor spawns concurrent copies — those stay
//! loud (a shared net would alias). A read-before-write WITHOUT an initializer also stays
//! loud (no init to reset the leftover).
//!
//! ORACLE: iverilog 13.0 rejects the automatic-lifetime override ("Overriding the default
//! variable lifetime is not yet supported"), so this is hand-IEEE (§6.21).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blai_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

// ── the report's D repro: a constant-init automatic block-local in a loop ──
#[test]
fn const_init_in_loop() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 2; i++) begin\n\
            automatic int lim = 20;\n\
            if (lim == 20) $display(\"PASS\");\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    // Two iterations, each re-inits lim=20 → two PASS lines.
    assert!(
        !is_loud(&o) && o.matches("PASS").count() == 2,
        "D repro (2× PASS):\n{o}"
    );
}

// ── per-entry re-init: modify then re-enter — each iteration starts fresh ──
#[test]
fn per_entry_reinit_not_carried() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 3; i++) begin\n\
            automatic int acc = 100;\n\
            acc = acc + i;\n\
            $display(\"acc=%0d\", acc);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    // §6.21: acc re-inits to 100 each entry → 100+i = 100, 101, 102 (NOT 100,101,103).
    assert!(
        !is_loud(&o)
            && o.contains("acc=100")
            && o.contains("acc=101")
            && o.contains("acc=102")
            && !o.contains("acc=103"),
        "per-entry re-init:\n{o}"
    );
}

// ── a NON-constant init reading the loop variable ──
#[test]
fn nonconst_init_reads_loop_var() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 3; i++) begin\n\
            automatic int x = i * 10;\n\
            $display(\"x=%0d\", x);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("x=0") && o.contains("x=10") && o.contains("x=20"),
        "non-const init:\n{o}"
    );
}

// ── multiple automatic-with-init locals in one block ──
#[test]
fn multiple_inits_in_block() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 2; i++) begin\n\
            automatic int a = 5;\n\
            automatic int b = 7;\n\
            $display(\"sum=%0d\", a + b + i);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("sum=12") && o.contains("sum=13"),
        "multiple inits:\n{o}"
    );
}

// ── an automatic-with-init in an always block (re-triggered) ──
#[test]
fn init_in_always_block() {
    let o = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        int seen = 0;\n\
        always @(posedge clk) begin\n\
          automatic int base = 42;\n\
          seen = base;\n\
        end\n\
        initial begin #21; if (seen == 42) $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "always block init:\n{o}"
    );
}

// ── correct-or-loud: a fork-spawned automatic-with-init stays LOUD (concurrency) ──
#[test]
fn fork_spawned_init_stays_loud() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 3; i++) begin\n\
            fork begin automatic int lim = i; #5; $display(\"lim=%0d\", lim); end join_none\n\
          end\n\
          #100 $finish;\n\
        end\n\
        endmodule\n");
    assert!(is_loud(&o), "fork-spawned init must stay loud:\n{o}");
}

// ── correct-or-loud: an automatic read-before-write WITHOUT an init stays LOUD ──
#[test]
fn read_before_write_no_init_stays_loud() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 2; i++) begin\n\
            automatic int y;\n\
            $display(\"y=%0d\", y);\n\
            y = i;\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "read-before-write (no init) must stay loud:\n{o}"
    );
}

// ── regression: an automatic local ASSIGNED before use (no init) still works ──
#[test]
fn definitely_assigned_unchanged() {
    let o = run("module t;\n\
        initial begin\n\
          for (int i = 0; i < 2; i++) begin\n\
            automatic int z;\n\
            z = i + 1;\n\
            $display(\"z=%0d\", z);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("z=1") && o.contains("z=2"),
        "definitely-assigned automatic:\n{o}"
    );
}
