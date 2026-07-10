//! Round-7 external report — two narrow gaps that (with the round-6 UARR fix) were
//! the last blockers to a full `hash_top` elaborate:
//!
//!   PKG   — a package-scoped subroutine CALL `pkg::name(args)` (IEEE §26.3). Was a
//!           parser loud (E2002, a v1 cut). Now the parser lowers it to a 2-segment
//!           `Call` and elaborate resolves the callee in the named package's scope.
//!           SUPPORTED for a SELF-CONTAINED, straight-line function (its body reads
//!           only its own formals/locals + literals + `pkg::CONST` + `$sys` calls, no
//!           control flow) — such a function inlines as a pure function of its args,
//!           correct with or without importing the package. A stateful / control-flow
//!           / bare-external-referencing package function is loud (workaround:
//!           `import pkg::*` and call by the bare name).
//!
//!   UARR2 — forwarding a function's OWN unpacked-array formal as the ACTUAL of
//!           another call (`f(a,i)` passing `f`'s formal `a` on to `g`). Round-6
//!           accepted only a bare MODULE-array actual; a sibling formal (a single
//!           md-packed frame net, not a static array) was loud. Now, when the caller
//!           and callee formals have the SAME shape (count × elem_w) and direction,
//!           the caller formal's whole md-packed value is passed through directly.
//!           A shape / direction mismatch is loud (§7.6 positional copy).
//!
//! Oracle = iverilog 13.0. It runs `pkg::f(x)` directly (PKG cases diff against it);
//! it does NOT run unpacked tf-ports ("sorry"), so UARR2 diffs against the equivalent
//! PACKED-vector twin (`a[0:3]` of `[7:0]` ⟷ `logic [31:0]`, `a[i]` ⟷ `a[(i*8)+:8]`,
//! element 0 at the LSB) — both give 10/20/30/40.
//!
//! No AST change (the call reuses `ExprKind::Call`); sim-ir / format_version 19
//! UNCHANGED (IR-0).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (first `o=`/`y=` stdout line, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r7_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("o=") || l.starts_with("y="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

/// Does `src` elaborate + simulate successfully (exit 0)?
fn ok(src: &str) -> bool {
    run(src).1
}

// ════════════════════════ PKG — supported ════════════════════════

#[test]
fn pkg_call_is_512_family_diff() {
    // The report's mirror (hash_top.sv:289). vita's explicit `cp::` form matches
    // iverilog running the same file: m==3 → 1, else 0.
    for (m, exp) in [(0, "0"), (2, "0"), (3, "1")] {
        let src = format!(
            "package cp; function automatic logic is_512_family(input logic [1:0] x); return (x==2'd3); endfunction endpackage\n\
             module m(output logic o); logic [1:0] s; assign o = cp::is_512_family(s);\n\
             initial begin s=2'd{m}; #1; $display(\"o=%0d\",o); end endmodule"
        );
        assert_eq!(run(&src).0, format!("o={exp}"), "m={m}");
    }
}

#[test]
fn pkg_call_xor_literal() {
    // `bp::f(2'b10)` with f(x)=x^2'b01 → 2'b11 = 3.
    let src = "package bp; function automatic logic [1:0] f(input logic [1:0] x); return x ^ 2'b01; endfunction endpackage\n\
        module m(output logic [1:0] o); assign o = bp::f(2'b10); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=3");
}

#[test]
fn pkg_call_multi_arg() {
    // A self-contained two-formal function.
    let src = "package p; function automatic logic [7:0] mac(input logic [7:0] a, input logic [7:0] b); return (a*b) + 8'd1; endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = p::mac(8'd3, 8'd4); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=13"); // 3*4+1
}

#[test]
fn pkg_call_reads_pkg_scoped_const() {
    // A body referencing a package constant by its SCOPED name (`p::OFF`) is still
    // self-contained — `p::OFF` resolves to a single global net (collision-free).
    let src = "package p; localparam logic [7:0] OFF = 8'd5;\n\
        function automatic logic [7:0] addoff(input logic [7:0] m); return (m + p::OFF); endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = p::addoff(8'd10); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=15");
}

#[test]
fn pkg_call_and_import_bare_agree() {
    // The explicit `p::f(s)` and the import+bare `f(s)` forms give the same result.
    let expl = "package p; function automatic logic [3:0] inv(input logic [3:0] x); return ~x; endfunction endpackage\n\
        module m(output logic [3:0] o); assign o = p::inv(4'hA); initial begin #1; $display(\"o=%h\",o); end endmodule";
    let bare = "package p; function automatic logic [3:0] inv(input logic [3:0] x); return ~x; endfunction endpackage\n\
        module m import p::*; (output logic [3:0] o); assign o = inv(4'hA); initial begin #1; $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(expl).0, "o=5");
    assert_eq!(run(expl).0, run(bare).0);
}

#[test]
fn pkg_call_return_carries_context_width() {
    // The context-width silent-wrong (round-7 R1): an overflowing top-level operator in
    // the return expression must keep its IEEE §11.6.1 assignment-context width (the
    // 16-bit return), NOT fold in the 8-bit operand width. `255+255` = 510, not 254;
    // `200*200` = 40000, not 64. This is why the feature routes through the frame path,
    // not the inline fold.
    for (op, aa, bb, exp) in [
        ("+", "255", "255", "510"),
        ("*", "200", "200", "40000"),
        ("<<", "255", "4", "4080"),
    ] {
        let src = format!(
            "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); return a {op} b; endfunction endpackage\n\
             module m(output logic [15:0] o); assign o = p::f(8'd{aa},8'd{bb}); initial begin #1; $display(\"o=%0d\",o); end endmodule"
        );
        assert_eq!(run(&src).0, format!("o={exp}"), "op {op}");
    }
}

#[test]
fn pkg_call_context_width_matches_import_bare() {
    // The differential that exposed the bug: the explicit `p::f` and import+bare forms
    // must AGREE on an OVERFLOWING function (both frame → 40000, not the inline 64).
    let expl = "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); return a*b; endfunction endpackage\n\
        module m(output logic [15:0] o); assign o = p::f(8'd200, 8'd200); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    let bare = "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); return a*b; endfunction endpackage\n\
        module m import p::*; (output logic [15:0] o); assign o = f(8'd200, 8'd200); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(expl).0, "o=40000");
    assert_eq!(run(expl).0, run(bare).0);
}

#[test]
fn pkg_call_name_assign_style() {
    // Classic Verilog function-NAME assignment (`f = expr;`, no `return`) works and
    // carries context width — the return var is named by the function, so the frame
    // body's `f = a + b` resolves it (16-bit → 510).
    let expl = "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); f = a + b; endfunction endpackage\n\
        module m(output logic [15:0] o); assign o = p::f(8'd255,8'd255); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(expl).0, "o=510");
    let bare = "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); f = a + b; endfunction endpackage\n\
        module m import p::*; (output logic [15:0] o); assign o = f(8'd255,8'd255); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(expl).0, run(bare).0);
}

#[test]
fn pkg_call_reads_own_return_name() {
    // The body may read its own partially-assigned return value by name
    // (`f = a*2; return f + 1;`). The frame path gives it a real return-var net.
    let src = "package p; function automatic int f(input int a); f = a*2; return f + 1; endfunction endpackage\n\
        module m; int o; initial begin #1; o = p::f(10); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=21");
}

#[test]
fn pkg_call_inside_module_function_body() {
    // A package call INSIDE another (framed) module function's body: the callee frame
    // is reserved + lowered on demand while the OUTER function's body is still being
    // built, so its blocks must not corrupt the outer function's CFG.
    let src = "package p; function automatic logic [15:0] add(input logic [7:0] a, input logic [7:0] b); return a+b; endfunction endpackage\n\
        module m; logic [15:0] o;\n\
        function automatic logic [15:0] g(input logic [7:0] x); return p::add(x, x); endfunction\n\
        initial begin #1; o = g(8'd200); $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=400");
}

#[test]
fn pkg_call_in_recursive_function() {
    // A package call inside a RECURSIVE module frame function.
    let src = "package p; function automatic logic [31:0] inc(input logic [31:0] a); return a+1; endfunction endpackage\n\
        module m; logic [31:0] o;\n\
        function automatic logic [31:0] cnt(input int n); if(n<=0) return 0; else return p::inc(cnt(n-1)); endfunction\n\
        initial begin #1; o = cnt(5); $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=5");
}

#[test]
fn pkg_call_reused_many_times() {
    // The same package function called repeatedly (frame reserved once, reused).
    let src = "package p; function automatic logic [15:0] sq(input logic [7:0] a); return a*a; endfunction endpackage\n\
        module m; logic [15:0] o; int i; initial begin o=0; for(i=1;i<=4;i++) o=o+p::sq(i[7:0]); #1; $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=30"); // 1+4+9+16
}

#[test]
fn pkg_call_in_always_ff() {
    // Package call in a clocked block.
    let src = "package p; function automatic logic [15:0] f(input logic [7:0] a, input logic [7:0] b); return a+b; endfunction endpackage\n\
        module m; logic clk; logic [7:0] x; logic [15:0] o; always_ff @(posedge clk) o <= p::f(x, x);\n\
        initial begin clk=0; x=8'd50; #1; clk=1; #1; $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=100");
}

#[test]
fn pkg_call_inside_task_body() {
    // A package call inside a frame TASK body: the callee frame is reserved+lowered on
    // demand while the task body is being built, so the task's block base must be
    // captured AFTER its body closure (else the CFG / nested-task-call keys corrupt).
    let src = "package p; function automatic logic [15:0] add(input logic [7:0] a, input logic [7:0] b); return a+b; endfunction endpackage\n\
        module m; logic [15:0] o;\n\
        task automatic go(input logic [7:0] x, output logic [15:0] r); r = p::add(x, x); endtask\n\
        initial begin #1; go(8'd200, o); $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=400");
}

#[test]
fn pkg_call_inside_class_method() {
    // A package call inside a class method body (same block-base concern as tasks).
    let src = "package p; function automatic logic [15:0] add(input logic [7:0] a, input logic [7:0] b); return a+b; endfunction endpackage\n\
        class C; logic [7:0] v; function new(logic [7:0] x); v=x; endfunction function logic [15:0] dbl(); return p::add(v, v); endfunction endclass\n\
        module m; C c; logic [15:0] o; initial begin c=new(8'd150); #1; o=c.dbl(); $display(\"o=%0d\", o); end endmodule";
    assert_eq!(run(src).0, "o=300");
}

// ════════════════════════ PKG — loud ════════════════════════

#[test]
fn pkg_call_control_flow_is_loud() {
    // A package function with control flow is not pure-inlinable → loud (workaround:
    // import + bare). Correct-or-loud.
    let src = "package cp; function automatic logic [7:0] cf(input logic [7:0] m); if (m>10) return 8'd1; else return 8'd0; endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = cp::cf(8'd20); endmodule";
    assert!(!ok(src), "control-flow package function must be loud");
}

#[test]
fn pkg_call_bare_external_ref_is_loud() {
    // A body reading a package-internal localparam by its BARE name is NOT
    // self-contained (that bare name would not resolve / could collide in the caller
    // module scope) → loud.
    let src = "package p; localparam logic [7:0] OFF = 8'd5;\n\
        function automatic logic [7:0] addoff(input logic [7:0] m); return (m + OFF); endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = p::addoff(8'd10); endmodule";
    assert!(!ok(src), "bare external reference must be loud");
}

#[test]
fn pkg_call_nested_user_call_is_loud() {
    // A body calling ANOTHER function is not self-contained → loud.
    let src = "package p; function automatic logic [7:0] h(input logic [7:0] x); return x+1; endfunction\n\
        function automatic logic [7:0] g(input logic [7:0] m); return h(m); endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = p::g(8'd10); endmodule";
    assert!(!ok(src), "nested user call must be loud");
}

#[test]
fn pkg_call_unknown_function_is_loud() {
    // A real package but no such function → loud (not a silent 0).
    let src = "package p; function automatic logic [7:0] f(input logic [7:0] x); return x; endfunction endpackage\n\
        module m(output logic [7:0] o); assign o = p::nope(8'd10); endmodule";
    assert!(!ok(src), "unknown package function must be loud");
}

#[test]
fn pkg_scoped_value_reference_still_works() {
    // Regression: a plain `pkg::CONST` VALUE reference (no call) is unaffected.
    let src = "package p; localparam logic [7:0] K = 8'd42; endpackage\n\
        module m(output logic [7:0] o); assign o = p::K; initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=42");
}

// ════════════════════════ UARR2 — supported ════════════════════════

#[test]
fn uarr2_passthrough_each_element() {
    // f(mem,i) forwards its own formal `a` on to g(a,i). mem='{10,20,30,40} →
    // 10/20/30/40 (== the packed twin g(f(32'h40302010)) = a[(i*8)+:8]).
    for (i, exp) in [(0, "10"), (1, "20"), (2, "30"), (3, "40")] {
        let src = format!(
            "module m(output logic [7:0] o);\n\
             function automatic logic [7:0] g(input logic [7:0] a[0:3], input int i); return a[i]; endfunction\n\
             function automatic logic [7:0] f(input logic [7:0] a[0:3], input int i); return g(a, i); endfunction\n\
             logic [7:0] mem[0:3];\n\
             initial begin mem='{{8'h10,8'h20,8'h30,8'h40}}; o=f(mem,{i}); $display(\"o=%h\",o); end endmodule"
        );
        assert_eq!(run(&src).0, format!("o={exp}"), "i={i}");
    }
}

#[test]
fn uarr2_passthrough_byte_unsigned() {
    // A 2-state (`byte unsigned`) element pass-through — the value survives; the
    // callee slot's 2-state coercion applies at binding. Packed twin gives the same.
    let src = "module m(output logic [7:0] o);\n\
        function automatic byte unsigned g(input byte unsigned a[0:3], input int i); return a[i]; endfunction\n\
        function automatic byte unsigned f(input byte unsigned a[0:3], input int i); return g(a, i); endfunction\n\
        byte unsigned mem[0:3];\n\
        initial begin mem='{8'h10,8'h20,8'h30,8'h40}; o=f(mem,2); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=30");
}

#[test]
fn uarr2_passthrough_64bit_element() {
    // Wider element (`[63:0]`, the SHA-2 shape) forwards correctly.
    let src = "module m(output logic [63:0] o);\n\
        function automatic logic [63:0] g(input logic [63:0] a[0:1], input int i); return a[i]; endfunction\n\
        function automatic logic [63:0] f(input logic [63:0] a[0:1], input int i); return g(a, i); endfunction\n\
        logic [63:0] mem[0:1];\n\
        initial begin mem='{64'hDEADBEEF_00000001, 64'hCAFEF00D_00000002}; o=f(mem,1); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=cafef00d00000002");
}

#[test]
fn uarr2_module_array_actual_still_works() {
    // Regression: a bare MODULE-array actual (round-6 form) is unaffected.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] g(input logic [7:0] a[0:3], input int i); return a[i]; endfunction\n\
        logic [7:0] mem[0:3];\n\
        initial begin mem='{8'h10,8'h20,8'h30,8'h40}; o=g(mem,1); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=20");
}

// ════════════════════════ UARR2 — loud ════════════════════════

#[test]
fn uarr2_shape_mismatch_is_loud() {
    // Caller formal [0:3], callee formal [0:7] (different count) → loud.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] g(input logic [7:0] a[0:7], input int i); return a[i]; endfunction\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3], input int i); return g(a, i); endfunction\n\
        logic [7:0] mem[0:3];\n\
        initial begin mem='{8'h10,8'h20,8'h30,8'h40}; o=f(mem,0); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "count mismatch must be loud");
}

#[test]
fn uarr2_direction_mismatch_is_loud() {
    // Caller [0:3] (ascending), callee [3:0] (descending) → loud (§7.6 positional
    // copy would silently reverse).
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] g(input logic [7:0] a[3:0], input int i); return a[i]; endfunction\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3], input int i); return g(a, i); endfunction\n\
        logic [7:0] mem[0:3];\n\
        initial begin mem='{8'h10,8'h20,8'h30,8'h40}; o=f(mem,0); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "direction mismatch must be loud");
}

#[test]
fn uarr2_elem_width_mismatch_is_loud() {
    // Caller element [7:0], callee element [15:0] → loud.
    let src = "module m(output logic [15:0] o);\n\
        function automatic logic [15:0] g(input logic [15:0] a[0:3], input int i); return a[i]; endfunction\n\
        function automatic logic [15:0] f(input logic [7:0] a[0:3], input int i); return g(a, i); endfunction\n\
        logic [7:0] mem[0:3];\n\
        initial begin mem='{8'h10,8'h20,8'h30,8'h40}; o=f(mem,0); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "element-width mismatch must be loud");
}

// ════════════════════════ regression ════════════════════════

#[test]
fn round6_uarr_formal_still_works() {
    // The round-6 UARR (unpacked-array formal on a function PORT) still elaborates.
    let src = "module m(output logic [7:0] o); logic [7:0] s;\n\
        function automatic logic [7:0] pick(input logic [7:0] arr[0:3], input int idx); return arr[idx]; endfunction\n\
        logic [7:0] tbl[0:3];\n\
        initial begin tbl='{8'h10,8'h20,8'h30,8'h40}; s=8'd2; o=pick(tbl,s[1:0]); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=30");
}
