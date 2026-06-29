//! `.name` implicit-named port-connection shorthand (IEEE §23.3.2.3). vita
//! parse-rejected a bare `.port` (no parens); now it desugars to `.port(port)`,
//! binding the port to a same-named signal in the instantiating scope. A missing
//! same-named signal stays a loud bind error (correct-or-loud, not a silent
//! skip). The shorthand is parser-only — it flows through the existing
//! named-connection path, so `.a` is byte-identical to writing `.a(a)`. Pinned
//! to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dotname_{}_{n}", std::process::id()));
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

#[test]
fn dotname_basic() {
    let (out, code) = run("module sub(input a, output b); assign b=~a; endmodule\n\
         module top; logic a=1,b; sub u(.a,.b); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("b=0"), ".a,.b shorthand; got:\n{out}");
}

#[test]
fn dotname_multibit() {
    // Widths follow the port/signal declarations, exactly like an explicit conn.
    let (out, _c) = run(
        "module sub(input [7:0] a, output [7:0] b); assign b=a+1; endmodule\n\
         module top; logic [7:0] a=8'd10,b; sub u(.a,.b); \
         initial begin #1 $display(\"b=%0d\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=11"), "multibit shorthand; got:\n{out}");
}

#[test]
fn dotname_mixed_with_explicit() {
    // `.name` shorthand interleaves with an explicit `.c(expr)`.
    let (out, _c) = run(
        "module sub(input a, input c, output b); assign b=a^c; endmodule\n\
         module top; logic a=1,b; sub u(.a,.c(1'b1),.b); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=0"), "mixed shorthand/explicit; got:\n{out}");
}

#[test]
fn dotname_order_independent() {
    // Named connections match by name, so the listing order is free.
    let (out, _c) = run(
        "module sub(input a, input c, output b); assign b=a&c; endmodule\n\
         module top; logic a=1,c=0,b; sub u(.b,.c,.a); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=0"), "reordered shorthand; got:\n{out}");
}

#[test]
fn dotname_alongside_explicit_unconnected() {
    // `.c()` (explicitly unconnected) coexists with `.name` shorthands.
    let (out, _c) = run(
        "module sub(input a, output b, output c); assign b=~a; assign c=a; endmodule\n\
         module top; logic a=1,b,c; sub u(.a,.b,.c()); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=0"), "shorthand + .c(); got:\n{out}");
}

#[test]
fn dotname_missing_signal_is_loud() {
    // `.a` with no same-named signal must be a loud bind error (iverilog: "Unable
    // to bind wire/reg/memory `a'"), never a silent skip.
    let (_o, code) = run("module sub(input a, output b); assign b=~a; endmodule\n\
         module top; logic q=1,b; sub u(.a,.b); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "missing same-named signal must be loud");
}

#[test]
fn dotname_equiv_explicit_byte_identical() {
    // The strongest test: `.a,.b` produces byte-identical stdout to the explicit
    // `.a(a),.b(b)` — proving the shorthand routes through the same path with no
    // new semantics (so a silent divergence is impossible).
    let shorthand = "module sub(input [3:0] a, output [3:0] b); assign b=a<<1; endmodule\n\
         module top; logic [3:0] a=4'd3,b; sub u(.a,.b); \
         initial begin #1 $display(\"b=%0d\",b); $finish; end endmodule\n";
    let explicit = "module sub(input [3:0] a, output [3:0] b); assign b=a<<1; endmodule\n\
         module top; logic [3:0] a=4'd3,b; sub u(.a(a),.b(b)); \
         initial begin #1 $display(\"b=%0d\",b); $finish; end endmodule\n";
    let (so, sc) = run(shorthand);
    let (eo, ec) = run(explicit);
    assert_eq!(sc, Some(0));
    assert_eq!(so, eo, ".name shorthand must equal explicit .name(name)");
    assert_eq!(sc, ec);
}
