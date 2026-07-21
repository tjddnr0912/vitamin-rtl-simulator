//! §4.5.189 SILENT-WRONG fix: a block-local declaration WITH an initializer inside
//! a LOOP body of an automatic task/function must re-run the initializer on EACH
//! block entry (IEEE §6.21 automatic-lifetime), not once at frame entry.
//!
//! The frame body lowering used to emit ALL nested block-local inits at frame ENTRY
//! (once per activation) — a documented "single-entry approximation" that silently
//! ran a `for(..) begin int t = f(k); .. end` init EXACTLY ONCE (with the loop var at
//! its entry value), giving a stale value every later iteration. Now each block's
//! decl-inits are emitted at the block's OWN entry (the `Block` arm of `lower_stmt`,
//! gated on `in_frame_body`); the storage slot still persists (so a no-init
//! read-before-write matches iverilog), only the initializer re-runs.
//!
//! Pinned LIVE against iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fbli_{}_{n}", std::process::id()));
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

#[test]
fn loop_block_local_init_reruns_in_task() {
    // iverilog: 1, 11, 21 (init k*10+1 re-runs each iteration).
    let o = run("module top;\n\
         task automatic t();\n\
           for (int k=0;k<3;k++) begin int x = k*10+1; $display(\"x=%0d\", x); end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    assert!(
        o.contains("x=1") && o.contains("x=11") && o.contains("x=21"),
        "got:\n{o}"
    );
}

#[test]
fn loop_block_local_init_reruns_in_function() {
    // iverilog: 1+11+21 = 33.
    let o = run("module top;\n\
         function automatic int f(int n);\n\
           int total = 0;\n\
           for (int k=0;k<n;k++) begin int step = k*10+1; total += step; end\n\
           return total;\n\
         endfunction\n\
         initial $display(\"f=%0d\", f(3));\n\
         endmodule\n");
    assert!(o.contains("f=33"), "got:\n{o}");
}

#[test]
fn nested_loops_block_local_init() {
    // iverilog: p = i*10+j → 0, 1, 10, 11.
    let o = run("module top;\n\
         task automatic t();\n\
           for (int i=0;i<2;i++)\n\
             for (int j=0;j<2;j++) begin int p = i*10+j; $display(\"p=%0d\", p); end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    for want in ["p=0", "p=1", "p=10", "p=11"] {
        assert!(o.contains(want), "missing {want} in:\n{o}");
    }
}

#[test]
fn sibling_decl_init_ordering() {
    // b reads a's re-initialized value each iteration: (2,3)(4,5)(6,7).
    let o = run("module top;\n\
         task automatic t();\n\
           for (int k=1;k<=3;k++) begin int a = k*2; int b = a + 1; $display(\"a=%0d b=%0d\", a, b); end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    assert!(
        o.contains("a=2 b=3") && o.contains("a=4 b=5") && o.contains("a=6 b=7"),
        "got:\n{o}"
    );
}

#[test]
fn while_loop_block_local_init() {
    // iverilog: sq = k*k → 0, 1, 4.
    let o = run("module top;\n\
         task automatic t();\n\
           int k = 0;\n\
           while (k < 3) begin int sq = k*k; $display(\"sq=%0d\", sq); k++; end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    assert!(
        o.contains("sq=0") && o.contains("sq=1") && o.contains("sq=4"),
        "got:\n{o}"
    );
}

#[test]
fn init_from_function_call_reruns() {
    // v = dbl(k)+1 → 1, 3, 5.
    let o = run("module top;\n\
         function automatic int dbl(int x); return x*2; endfunction\n\
         task automatic t();\n\
           for (int k=0;k<3;k++) begin int v = dbl(k) + 1; $display(\"v=%0d\", v); end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    assert!(
        o.contains("v=1") && o.contains("v=3") && o.contains("v=5"),
        "got:\n{o}"
    );
}

#[test]
fn no_init_read_before_write_persists_like_iverilog() {
    // A block-local WITHOUT an init keeps its slot across iterations (iverilog
    // parity: `int acc; acc=acc+1;` accumulates 1,2,3 — the fix re-runs only
    // INITIALIZERS, it does not reset an uninitialized slot).
    let o = run("module top;\n\
         task automatic t();\n\
           for (int k=0;k<3;k++) begin int acc; acc = acc + 1; $display(\"acc=%0d\", acc); end\n\
         endtask\n\
         initial t();\n\
         endmodule\n");
    assert!(
        o.contains("acc=1") && o.contains("acc=2") && o.contains("acc=3"),
        "got:\n{o}"
    );
}

#[test]
fn block_init_once_per_call_unchanged() {
    // A non-loop block entered once per call still inits once per call (10, 14).
    let o = run("module top;\n\
         task automatic t(int seed);\n\
           begin int a = seed * 2; $display(\"a=%0d\", a); end\n\
         endtask\n\
         initial begin t(5); t(7); end\n\
         endmodule\n");
    assert!(o.contains("a=10") && o.contains("a=14"), "got:\n{o}");
}

#[test]
fn module_initial_block_local_stays_static() {
    // Regression guard: a MODULE-process block-local keeps its static (once-at-t0)
    // init — iverilog also prints 1,1,1 here ("static var init requires explicit
    // lifetime"). The fix is gated on `in_frame_body`, so module context is unchanged.
    let o = run("module top;\n\
         initial begin\n\
           for (int k=0;k<3;k++) begin int x = k*10+1; $display(\"x=%0d\", x); end\n\
         end\n\
         endmodule\n");
    // all three iterations read the single static init value.
    assert_eq!(o.matches("x=1").count(), 3, "got:\n{o}");
}
