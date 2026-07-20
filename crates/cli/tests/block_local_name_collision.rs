//! §4.5.180 (SILENT-WRONG → loud): two same-named STATIC block-locals in DISJOINT
//! procedural blocks (sibling `begin…end`, named blocks, or separate processes) are
//! flattened by v1 to ONE module net. That coalesce was assumed "safe — the net is just
//! reused in time," but it is byte-identical to iverilog's distinct-per-scope variables
//! ONLY when the two decls have the SAME type AND the second block assigns before it reads.
//! Otherwise:
//!   - a DIFFERENT type (sign/width) makes the second block read/write the shared net with
//!     the wrong sign or width — wrong `%d` sign, wrong `>>>` arithmetic, wrong `%h` width
//!     (`begin logic signed [7:0] y; … end  begin logic [7:0] y; y=x; $display("%0d",y); end`
//!     printed a negative value where iverilog printed the unsigned 253);
//!   - a READ-BEFORE-WRITE observes the FIRST block's leftover value instead of the X a
//!     fresh variable holds (`begin y=5; end  begin logic [7:0] y; $display(y); end` printed
//!     5 where iverilog printed x).
//! Both are silent-wrong. v1 has no per-block scope to give the second local distinct typed
//! storage, so it is now loud (E3009). The SAFE same-type + definitely-assigned coalesce
//! (the common `for`/`tmp` name reuse) is unaffected. A legitimate SHADOW of a module-scope
//! var (handled by the struct/enum/typedef scoping or the scope-leak guard) is unaffected.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first KEY= line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blnc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let key = text
        .lines()
        .find(|l| l.starts_with("K="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (key, out.status.success())
}

fn loud(src: &str) -> bool {
    !run(src).1
}

// ── silent-wrong → loud ─────────────────────────────────────────────────────

#[test]
fn sibling_blocks_different_sign_loud() {
    // signed y (block A) then unsigned y (block B): the shared net's signedness would be
    // wrong for B (%0d of 0b11111101 = -3 instead of 253).
    assert!(loud(
        "module top; initial begin\n\
         begin logic signed [3:0] xa; logic signed [7:0] y; xa=-3; y=xa; $display(\"K=%0d\",y); end\n\
         begin logic signed [3:0] xb; logic [7:0] y; xb=-3; y=xb; $display(\"K=%0d\",y); end\n\
         end endmodule"
    ));
}

#[test]
fn sibling_blocks_different_width_loud() {
    assert!(loud(
        "module top; initial begin\n\
         begin logic [7:0] y; y=8'hAB; $display(\"K=%h\",y); end\n\
         begin logic [3:0] y; y=4'hC; $display(\"K=%h\",y); end\n\
         end endmodule"
    ));
}

#[test]
fn sibling_blocks_wrong_shift_sign_loud() {
    // >>> on the shared net uses block A's SIGNED type → arithmetic shift instead of
    // logical (a live semantic wrong, not just display).
    assert!(loud(
        "module top; initial begin\n\
         begin logic signed [7:0] y; y=-1; $display(\"K=%0d\", y>>>1); end\n\
         begin logic [7:0] y; y=8'hFE; $display(\"K=%0d\", y>>>1); end\n\
         end endmodule"
    ));
}

#[test]
fn read_before_write_stale_value_loud() {
    // block B reads its own y before assigning it — on the shared net it observes block
    // A's leftover 5 instead of the X a fresh variable would hold.
    assert!(loud(
        "module top; initial begin\n\
         begin logic [7:0] y; y=8'd5; end\n\
         begin logic [7:0] y; $display(\"K=%0d\", y); end\n\
         end endmodule"
    ));
}

#[test]
fn named_blocks_collision_loud() {
    assert!(loud(
        "module top; initial begin\n\
         begin : ba logic signed [7:0] y; y=-3; $display(\"K=%0d\",y); end\n\
         begin : bb logic [7:0] y; y=8'hFD; $display(\"K=%0d\",y); end\n\
         end endmodule"
    ));
}

#[test]
fn cross_process_collision_loud() {
    // Two SEPARATE initial blocks: the first process's y is flattened to `top.y`, the
    // second collides with it — different signedness → loud.
    assert!(loud(
        "module top;\n\
         initial begin logic signed [7:0] y; y=-3; #1 $display(\"K=%0d\",y); end\n\
         initial begin logic [7:0] y; y=8'hFD; #2 $display(\"K=%0d\",y); end\n\
         endmodule"
    ));
}

// ── safe cases — must STILL run (no over-rejection) ─────────────────────────

#[test]
fn same_type_assigned_first_reuse_works() {
    // The common pattern: reuse `i`/`s` (same type, assigned before read) in two
    // sequential blocks — a legitimate flatten coalesce, unaffected.
    let (k, ok) = run("module top; initial begin\n\
         begin int i; int s; s=0; for(i=0;i<4;i++) s+=i; $display(\"K=%0d\", s); end\n\
         begin int i; int s; s=0; for(i=0;i<3;i++) s+=i*2; $display(\"K=%0d\", s); end\n\
         $finish; end endmodule");
    assert!(
        ok && k == "K=6",
        "same-type reuse must still work; got ({k}, {ok})"
    );
}

#[test]
fn unique_names_unaffected() {
    // Distinct names in sibling blocks never collide — plainly fine.
    let (k, ok) = run("module top; initial begin\n\
         begin logic signed [7:0] ya; ya=-3; end\n\
         begin logic [7:0] yb; yb=8'hFD; $display(\"K=%0d\", yb); end\n\
         $finish; end endmodule");
    assert!(ok && k == "K=253", "unique names must run; got ({k}, {ok})");
}

#[test]
fn single_block_local_unaffected() {
    // A lone block-local (no collision) is byte-identical to before.
    let (k, ok) = run(
        "module top; initial begin logic signed [7:0] y; y=-5; $display(\"K=%0d\", y); #1 $finish; end endmodule",
    );
    assert!(
        ok && k == "K=-5",
        "single block-local must run; got ({k}, {ok})"
    );
}
