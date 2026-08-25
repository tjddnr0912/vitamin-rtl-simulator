//! IEEE 1800-2017 §27.3 — **`generate` and `endgenerate` are OPTIONAL** — and §27.4's
//! `genvar_initialization`, which lets the loop variable be declared in the header.
//!
//! Both are the dominant modern spelling and what synthesis tools and other simulators
//! are handed. vitamin required the wrapper and required the `genvar` outside, and the
//! error it produced for the first pointed at the `end` / `else` that FOLLOWED rather
//! than at the missing keyword. The second forces every loop variable into module
//! scope, handing name-collision management back to the author.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gok_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
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

const TB: &str = "module tb; logic a; logic [3:0] b;\n  \
                  leaf u1(.o(a)); leaf2 u2(.o(b));\n  \
                  initial begin #1 $display(\"R=%b %b\", a, b); $finish; end\nendmodule\n";

#[test]
fn keyword_less_conditional_and_loop_generate_parse() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\n\
         module leaf(output logic o);\n  \
         if (1) begin : g assign o = 1'b1; end else begin : h assign o = 1'b0; end\n\
         endmodule\n\
         module leaf2(output logic [3:0] o);\n  \
         for (genvar i = 0; i < 4; i = i+1) begin : g assign o[i] = i[0]; end\n\
         endmodule\n{TB}"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=1 1010"), "both oracles: 1 1010\n{out}");
}

/// The wrapped spelling is unchanged — the two must agree, or one source text has two
/// answers.
#[test]
fn the_wrapped_spelling_gives_the_same_answer() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\n\
         module leaf(output logic o);\n  generate\n  \
         if (1) begin : g assign o = 1'b1; end else begin : h assign o = 1'b0; end\n  \
         endgenerate\nendmodule\n\
         module leaf2(output logic [3:0] o);\n  genvar i;\n  generate\n  \
         for (i = 0; i < 4; i = i+1) begin : g assign o[i] = i[0]; end\n  \
         endgenerate\nendmodule\n{TB}"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=1 1010"), "{out}");
}

/// A header `genvar` inside an explicit `generate` block works too, and `i++` — which
/// was already accepted for an outside-declared genvar — still does.
#[test]
fn a_header_genvar_works_inside_an_explicit_generate() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         module leaf2(output logic [3:0] o);\n  generate\n  \
         for (genvar i = 0; i < 4; i++) begin : g assign o[i] = i[1]; end\n  \
         endgenerate\nendmodule\n\
         module tb; logic [3:0] b; leaf2 u(.o(b));\n  \
         initial begin #1 $display(\"R=%b\", b); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=1100"), "both oracles: 1100\n{out}");
}

/// ⭐ IEEE §26.3: a package routine's body resolves in its DECLARING scope. Only the
/// FRAME path carried that scope, so a STATIC package function could not see its own
/// sibling under a selective import — while spelling it `automatic` worked, and so did
/// importing the sibling as well, which is the caller's business and not the callee's.
#[test]
fn a_static_package_function_sees_its_own_sibling() {
    for life in ["", "automatic "] {
        let (out, code) = run(&format!(
            "`timescale 1ns/1ns\npackage p;\n  \
             function {life}logic [7:0] xtime(input logic [7:0] a); xtime = a << 1; endfunction\n  \
             function {life}logic [7:0] gmul (input logic [7:0] a); gmul = xtime(a) ^ 8'h1b; endfunction\n\
             endpackage\n\
             module tb; import p::gmul;\n  logic [7:0] r; assign r = gmul(8'h5a);\n  \
             initial begin #1 $display(\"R=%h\", r); $finish; end\nendmodule\n"
        ));
        assert_eq!(code, Some(0), "lifetime `{life}`:\n{out}");
        assert!(
            out.contains("R=af"),
            "iverilog: af (lifetime `{life}`):\n{out}"
        );
    }
}

/// ⚠️ …and the caller's OWN same-named function still wins in an ACTUAL argument. The
/// package scope is pushed around the callee's BODY only; pushing it earlier would
/// silently hand the module's `xtime(3)` to the package.
#[test]
fn the_callers_own_function_still_wins_in_an_argument() {
    let (out, code) = run("`timescale 1ns/1ns\npackage p;\n  \
         function logic [7:0] xtime(input logic [7:0] a); xtime = a << 1; endfunction\n  \
         function logic [7:0] gmul (input logic [7:0] a); gmul = xtime(a) ^ 8'h1b; endfunction\n\
         endpackage\n\
         module tb; import p::gmul;\n  \
         function logic [7:0] xtime(input logic [7:0] a); xtime = a + 8'd1; endfunction\n  \
         logic [7:0] r; assign r = gmul(xtime(8'h5a));\n  \
         initial begin #1 $display(\"R=%h\", r); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    // iverilog: the module's xtime adds 1 (0x5b), the package's shifts (0xb6), xor 1b
    // => 0xad. If the argument had taken the package's xtime the answer would be 0xa3.
    assert!(
        out.contains("R=ad"),
        "iverilog: ad (a3 = the package won):\n{out}"
    );
}
