//! `defparam inst.param = value;` (IEEE §23.10.1) — a hierarchical parameter
//! override. vita parse-rejected it; now the common DIRECT-child case is supported.
//! The override is collected when the parent's FQ scope is current and applied to
//! the child instance before its params bind (so it folds widths and beats a
//! `#()` override per the LRM). A deeper `a.b.c` path or a non-constant value stays
//! loud (correct-or-loud, not a silent skip). Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dfp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

const SUB: &str = "module sub#(parameter N=0)(); initial $display(\"N=%0d\",N); endmodule\n";

#[test]
fn defparam_direct_child() {
    let (out, code) = run(&format!(
        "{SUB}module top; sub u(); defparam u.N=7; initial #1 $finish; endmodule\n"
    ));
    assert_eq!(code, Some(0));
    assert!(out.contains("N=7"), "defparam u.N=7; got:\n{out}");
}

#[test]
fn defparam_affects_width() {
    // The override must apply BEFORE the child binds, so a `[W-1:0]` width folds.
    let (out, code) = run(
        "module sub#(parameter W=4)(); reg [W-1:0] r; initial begin r='1; \
         $display(\"W=%0d r=%h\",W,r); end endmodule\n\
         module top; sub u(); defparam u.W=8; initial #1 $finish; endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("W=8 r=ff"), "defparam width; got:\n{out}");
}

#[test]
fn defparam_order_independent() {
    // A defparam written BEFORE the instance still applies (collected per module).
    let (out, _c) = run(&format!(
        "{SUB}module top; defparam u.N=7; sub u(); initial #1 $finish; endmodule\n"
    ));
    assert!(out.contains("N=7"), "defparam before inst; got:\n{out}");
}

#[test]
fn defparam_wins_over_hash_paren() {
    // IEEE §23.10.1: a defparam overrides a `#()` value.
    let (out, _c) = run(&format!(
        "{SUB}module top; sub#(.N(3)) u(); defparam u.N=7; initial #1 $finish; endmodule\n"
    ));
    assert!(out.contains("N=7"), "defparam wins over #(); got:\n{out}");
}

#[test]
fn defparam_value_is_parent_param_expr() {
    // The value is const-evaluated in the parent scope (sees parent params).
    let (out, _c) = run(&format!(
        "{SUB}module top; localparam P=12; sub u(); defparam u.N=P+1; \
         initial #1 $finish; endmodule\n"
    ));
    assert!(out.contains("N=13"), "defparam value=P+1; got:\n{out}");
}

#[test]
fn defparam_last_write_wins() {
    let (out, _c) = run(&format!(
        "{SUB}module top; sub u(); defparam u.N=1; defparam u.N=9; \
         initial #1 $finish; endmodule\n"
    ));
    assert!(out.contains("N=9"), "last write wins; got:\n{out}");
}

#[test]
fn defparam_per_instance() {
    let (out, _c) = run(&format!(
        "{SUB}module top; sub a(); sub b(); defparam a.N=1; defparam b.N=2; \
         initial #1 $finish; endmodule\n"
    ));
    assert!(
        out.contains("N=1") && out.contains("N=2"),
        "per-instance; got:\n{out}"
    );
}

#[test]
fn defparam_unmatched_target_warns_keeps_default() {
    // A defparam whose instance does not exist (a typo, or an array `u.N` with no
    // index) must NOT silently apply nor crash: iverilog warns ("Scope not found")
    // and the parameter keeps its default. vita matches — a warning (W3056) + the
    // default value + a successful run (an adversarial review caught the original
    // silent no-op of the unconsumed override).
    let (out, code) = run(&format!(
        "{SUB}module top; sub u(); defparam typo.N=7; initial #1 $finish; endmodule\n"
    ));
    assert_eq!(code, Some(0), "unmatched defparam must not be fatal");
    assert!(out.contains("N=0"), "target keeps its default; got:\n{out}");
}

#[test]
fn defparam_multilevel_is_loud() {
    // A multi-level `m.l.N` target is not yet supported — must be loud, not silent.
    let (_o, code) = run(
        "module leaf#(parameter N=0)(); initial $display(\"N=%0d\",N); endmodule\n\
         module mid(); leaf l(); endmodule\n\
         module top; mid m(); defparam m.l.N=9; initial #1 $finish; endmodule\n",
    );
    assert_ne!(code, Some(0), "multi-level defparam must be loud");
}
