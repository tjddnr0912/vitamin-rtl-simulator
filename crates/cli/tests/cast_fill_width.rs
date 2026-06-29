//! A fill literal (`'1`/`'x`/`'z`) cast to a width takes that width (IEEE §11.6):
//! `8'('1)` is `8'hFF`, `int'('1)` is all-ones. vita lowered the fill operand at
//! one bit and then zero-extended it (`8'('1)` gave `01`, `int'('1)` gave
//! `00000001`) — a silent-wrong across the size cast `N'(e)`, the parameter-width
//! size cast `W'(e)`, and the primitive cast `int'(e)`/`byte'(e)`. Fixed by lowering
//! the operand in the cast's target-width context. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_castfill_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn size_cast_fill_one() {
    let out = run("module top; logic [7:0] x; \
         initial begin x=8'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(out.contains("x=ff"), "8'('1) must be ff; got:\n{out}");
}

#[test]
fn size_cast_fill_x_and_z() {
    let xo = run("module top; logic [7:0] x; \
         initial begin x=8'('x); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(xo.contains("x=xx"), "8'('x) must be xx; got:\n{xo}");
    let zo = run("module top; logic [7:0] x; \
         initial begin x=8'('z); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(zo.contains("x=zz"), "8'('z) must be zz; got:\n{zo}");
}

#[test]
fn size_cast_fill_various_widths() {
    let w4 = run("module top; logic [3:0] x; \
         initial begin x=4'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(w4.contains("x=f"), "4'('1) must be f; got:\n{w4}");
    let w16 = run("module top; logic [15:0] x; \
         initial begin x=16'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(w16.contains("x=ffff"), "16'('1) must be ffff; got:\n{w16}");
}

#[test]
fn param_width_size_cast_fill() {
    let out = run("module top; parameter W=8; logic [7:0] x; \
         initial begin x=W'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(out.contains("x=ff"), "W'('1) must be ff; got:\n{out}");
}

#[test]
fn prim_cast_fill() {
    let i = run("module top; int x; \
         initial begin x=int'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(
        i.contains("x=ffffffff"),
        "int'('1) must be all-ones; got:\n{i}"
    );
    let b = run("module top; byte x; \
         initial begin x=byte'('1); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(b.contains("x=ff"), "byte'('1) must be ff; got:\n{b}");
}

#[test]
fn size_cast_non_fill_unchanged() {
    // A non-fill operand keeps self-determined sizing (byte-identical) — `8'(4'hA)`
    // zero-extends the 4-bit value to `0a`.
    let out = run("module top; logic [7:0] x; \
         initial begin x=8'(4'hA); $display(\"x=%h\",x); #1 $finish; end endmodule\n");
    assert!(out.contains("x=0a"), "8'(4'hA) must stay 0a; got:\n{out}");
}
