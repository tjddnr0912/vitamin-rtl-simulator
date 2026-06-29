//! `.*` implicit (wildcard) port connection (IEEE §23.3.2.5). vita parse-rejected
//! `.*` ("not yet supported; ignored"); it now connects every port the explicit
//! list does not name to a same-named signal in the instantiating scope. A port
//! whose same-named signal is missing stays a loud bind error (correct-or-loud,
//! not a silent float). `.*` on an instance array is a loud v1 limitation. Pinned
//! to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dotstar_{}_{n}", std::process::id()));
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
fn star_basic() {
    // `.*` connects a, c, b all by name → 1 ^ 0 = 1.
    let (out, code) = run(
        "module sub(input a, input c, output b); assign b=a^c; endmodule\n\
         module top; logic a=1,c=0,b; sub u(.*); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(out.contains("b=1"), ".* basic; got:\n{out}");
}

#[test]
fn star_mixed_with_explicit() {
    // An explicit `.c(1'b0)` takes precedence; `.*` fills a and b. 1 ^ 0 = 1.
    let (out, _c) = run(
        "module sub(input a, input c, output b); assign b=a^c; endmodule\n\
         module top; logic a=1,c=1,b; sub u(.*, .c(1'b0)); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=1"), ".* + explicit override; got:\n{out}");
}

#[test]
fn star_multibit() {
    let (out, _c) = run(
        "module sub(input [7:0] a, output [7:0] b); assign b=a+1; endmodule\n\
         module top; logic [7:0] a=8'd10,b; sub u(.*); \
         initial begin #1 $display(\"b=%0d\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=11"), ".* multibit; got:\n{out}");
}

#[test]
fn star_alongside_explicit_unconnected() {
    // `.c()` is explicitly unconnected, so `.*` does not reconnect it.
    let (out, _c) = run(
        "module sub(input a, output b, output c); assign b=~a; assign c=a; endmodule\n\
         module top; logic a=1,b; sub u(.*, .c()); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert!(out.contains("b=0"), ".* + .c(); got:\n{out}");
}

#[test]
fn star_explicit_port_excluded_from_wildcard() {
    // `.a(1'b0)` names port a, so `.*` must not also connect a → b = ~0 = 1.
    let (out, _c) = run("module sub(input a, output b); assign b=~a; endmodule\n\
         module top; logic a=1,b; sub u(.*, .a(1'b0)); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n");
    assert!(
        out.contains("b=1"),
        "explicit excluded from wildcard; got:\n{out}"
    );
}

#[test]
fn star_missing_signal_is_loud() {
    // `.*` for a port whose same-named signal is missing → loud (iverilog:
    // "Wildcard ... did not find a matching identifier"), never a silent float.
    let (_o, code) = run(
        "module sub(input a, input c, output b); assign b=a^c; endmodule\n\
         module top; logic a=1,b; sub u(.*); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n",
    );
    assert_ne!(code, Some(0), "missing wildcard signal must be loud");
}

#[test]
fn star_same_name_constant_is_loud() {
    // IEEE §23.3.2.5: `.*` matches a NET/VARIABLE, not a constant. A `parameter`
    // sharing an input port's name must NOT silently feed its value into the port
    // (iverilog: "did not find a matching identifier") — an adversarial review
    // caught vita silently connecting the constant. A localparam/enum label is
    // the same case.
    let (_o, code) = run(
        "module buf1(input [7:0] d, output [7:0] q); assign q=d; endmodule\n\
         module top; parameter d=8'hAA; logic [7:0] q; buf1 u(.*); \
         initial begin #1 $display(\"q=%h\",q); #10 $finish; end endmodule\n",
    );
    assert_ne!(code, Some(0), "`.*` to a same-named parameter must be loud");
}

#[test]
fn star_portless_module_ok() {
    // `.*` on a port-less module matches zero ports → no error (iverilog parity).
    let (out, code) = run("module sub(); initial $display(\"hi\"); endmodule\n\
         module top; sub u(.*); initial begin #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("hi"), "portless .* ok; got:\n{out}");
}

#[test]
fn star_instance_array_is_loud() {
    // `.*` across an instance array is a loud v1 limitation, not a silent
    // mis-wire (iverilog supports it; vita refuses cleanly).
    let (_o, code) = run("module sub(input a, output b); assign b=~a; endmodule\n\
         module top; logic [1:0] a=2'b10,b; sub u[1:0](.*); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n");
    assert_ne!(code, Some(0), ".* on instance array must be loud");
}

#[test]
fn star_equiv_all_named_byte_identical() {
    // The strongest test: `.*` produces byte-identical stdout to spelling out
    // every connection `.a,.c,.b` — proving the wildcard expands to exactly the
    // same named connections (so a silent divergence is impossible).
    let star = "module sub(input a, input c, output [3:0] b); assign b={a,c,a,c}; endmodule\n\
         module top; logic a=1,c=0; logic [3:0] b; sub u(.*); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n";
    let named = "module sub(input a, input c, output [3:0] b); assign b={a,c,a,c}; endmodule\n\
         module top; logic a=1,c=0; logic [3:0] b; sub u(.a,.c,.b); \
         initial begin #1 $display(\"b=%b\",b); $finish; end endmodule\n";
    let (so, sc) = run(star);
    let (no, nc) = run(named);
    assert_eq!(sc, Some(0));
    assert_eq!(so, no, ".* must equal the fully-named connection list");
    assert_eq!(sc, nc);
}
