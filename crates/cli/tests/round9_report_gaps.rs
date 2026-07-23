//! Round-9 external report — four gaps blocking the existing `hash_top` testbench
//! environment from running under vita. All were LOUD (correct-or-loud OK); this
//! round turns each into a bounded, correct feature.
//!
//!   PKG2  — a package-scoped call `pkg::fn(args)` whose body READS other symbols
//!           of the SAME package (enum labels / localparams). Round-7/8 accepted a
//!           self-contained fn (own formals/locals only); now the frame-body
//!           lowering injects the package's constants under the call scope so a
//!           bare `C`/`LIM` resolves to `pkg::C`/`pkg::LIM`. Also: a package-scoped
//!           TYPE cast `pkg::T'(e)` (was misread as a size cast). Routed through
//!           the FRAME path, so the result matches the bare-import call.
//!
//!   FIO   — `$feof(fd)` in EXPRESSION / condition context (`while (!$feof(fd))`).
//!           `$feof` is pure (reads the EOF flag, no state mutation), so it now
//!           maps like any value sysfunc. The six fd-ADVANCING file reads stay
//!           direct-rhs-only (a second eval under unspecified order would
//!           double-advance) — LOUD elsewhere.
//!
//!   bind  — top-level `bind <target> <checker> u(...)` (IEEE §23.11). Attaches an
//!           observer checker instance inside every instance of the target,
//!           reusing the ordinary child-instantiation path (wired in the target
//!           instance's scope). Named connections, input-only checker.
//!
//!   USTRUCT — a scalar UNPACKED struct (record) with `string`/`int`/`logic`
//!           members. vita has no heterogeneous-aggregate storage, so the variable
//!           desugars to N independent member nets `k$field`. An array-of-records,
//!           a decl-init `'{…}` pattern, and a whole-struct copy stay LOUD (the
//!           record-array form the TB ultimately needs is a deep follow-on).
//!
//! Oracles: PKG2 + FIO diff against iverilog 13.0 (it runs both directly). bind and
//! unpacked structs are NOT supported by iverilog 13.0 ("syntax error" / "sorry"),
//! so they are validated against the equivalent manual desugar (a hand-instantiated
//! checker / separate plain member variables) which iverilog DOES run — verified
//! byte-identical in the bring-up diffs.
//!
//! IR-0: sim-ir / format_version 19 UNCHANGED. `bind` adds `TopItem::Bind` (an AST
//! `.vu` re-pin); PKG2 / FIO / unpacked-struct are front-end + elaborate only.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (first `o=` stdout line, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r9_{}_{n}", std::process::id()));
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
        .find(|l| l.starts_with("o="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

/// Does `src` elaborate + simulate successfully (exit 0)?
fn ok(src: &str) -> bool {
    run(src).1
}

// ════════════════════════ PKG2 — same-package const body refs ════════════════════════

const CP: &str = "package cp;\n\
    typedef enum logic [1:0] { A=2'd0, B=2'd1, C=2'd2, D=2'd3 } e_t;\n\
    localparam int LIM = 100;\n\
    function automatic logic is_hi(input e_t m); is_hi = (m == C) || (m == D); endfunction\n\
    function automatic int clamp(input int x); clamp = (x > LIM) ? LIM : x; endfunction\n\
endpackage\n";

#[test]
fn pkg2_enum_label_body_ref() {
    // is_hi body reads C, D — enum labels of the SAME package.
    let src = format!(
        "{CP}module m; logic y0,y1,y2,y3; initial begin #1;\n\
         y0=cp::is_hi(cp::A); y1=cp::is_hi(cp::B); y2=cp::is_hi(cp::C); y3=cp::is_hi(cp::D);\n\
         $display(\"o=%0d%0d%0d%0d\", y0,y1,y2,y3); end endmodule"
    );
    assert_eq!(run(&src).0, "o=0011");
}

#[test]
fn pkg2_localparam_body_ref() {
    // clamp body reads LIM — a localparam of the SAME package.
    let src = format!(
        "{CP}module m; int c0,c1; initial begin #1;\n\
         c0=cp::clamp(50); c1=cp::clamp(250); $display(\"o=%0d_%0d\", c0,c1); end endmodule"
    );
    assert_eq!(run(&src).0, "o=50_100");
}

#[test]
fn pkg2_scoped_type_cast() {
    // The repro's `cp::e_t'(s)` package-scoped TYPE cast (was misread as a size cast).
    let src = format!(
        "{CP}module m; logic [1:0] s; logic y; initial begin\n\
         s=2'd3; #1; y=cp::is_hi(cp::e_t'(s)); $display(\"o=%0d\", y); end endmodule"
    );
    assert_eq!(run(&src).0, "o=1");
}

#[test]
fn pkg2_context_width_preserved() {
    // Frame routing keeps IEEE §11.6.1 assignment-context width — an 8+8→16 add
    // does NOT drop the carry (255+255 = 510, not 254).
    let src = "package q; function automatic int add8(input byte unsigned a, input byte unsigned b); add8 = a + b; endfunction endpackage\n\
        module m; int r; initial begin #1; r=q::add8(8'd255, 8'd255); $display(\"o=%0d\", r); end endmodule";
    assert_eq!(run(src).0, "o=510");
}

#[test]
fn pkg2_bare_import_agrees() {
    // The scoped call and the bare-import call must agree.
    let scoped = format!(
        "{CP}module m; logic y; initial begin #1; y=cp::is_hi(cp::C); $display(\"o=%0d\", y); end endmodule"
    );
    let bare = format!(
        "{CP}module m; import cp::*; logic y; initial begin #1; y=is_hi(C); $display(\"o=%0d\", y); end endmodule"
    );
    assert_eq!(run(&scoped).0, run(&bare).0);
    assert_eq!(run(&scoped).0, "o=1");
}

#[test]
fn pkg2_write_to_pkg_const_still_loud() {
    // A body that WRITES a package const is not self-contained → loud.
    let src = "package q; localparam int K=1; function automatic int f(input int x); K = x; f = K; endfunction endpackage\n\
        module m; int r; initial begin r=q::f(3); $display(\"o=%0d\", r); end endmodule";
    assert!(!ok(src));
}

#[test]
fn pkg2_control_flow_still_loud() {
    // A control-flow package fn is still loud (workaround: import + bare call).
    let src = "package q; function automatic int f(input int x); if (x>0) f=1; else f=0; endfunction endpackage\n\
        module m; int r; initial begin r=q::f(3); $display(\"o=%0d\", r); end endmodule";
    assert!(!ok(src));
}

#[test]
fn pkg2_other_package_ref_still_loud() {
    // A body referencing ANOTHER package's symbol is not self-contained → loud.
    let src = "package a; localparam int G=7; endpackage\n\
        package b; function automatic int f(input int x); f = x + G; endfunction endpackage\n\
        module m; int r; initial begin r=b::f(1); $display(\"o=%0d\", r); end endmodule";
    assert!(!ok(src));
}

// ════════════════════════ FIO — $feof in expression context ════════════════════════

#[test]
fn fio_feof_in_while_condition() {
    // Write a 3-line file, count lines read with `while (!$feof(fd))`.
    let dir = std::env::temp_dir().join(format!("vita_r9feof_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let vec = dir.join("vec9.txt");
    std::fs::write(&vec, "a\nb\nc\n").unwrap();
    let src = format!(
        "module m; initial begin int fd; string line; int code; int n;\n\
         fd=$fopen(\"{}\", \"r\"); n=0;\n\
         while (!$feof(fd)) begin code=$fgets(line, fd); if (code!=0) n++; end\n\
         $fclose(fd); $display(\"o=%0d\", n); end endmodule",
        vec.to_str().unwrap()
    );
    assert_eq!(run(&src).0, "o=3");
}

#[test]
fn fio_feof_in_operand() {
    // `$feof` as an ordinary operand on a not-yet-EOF fd → 0.
    let dir = std::env::temp_dir().join(format!("vita_r9feof2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let vec = dir.join("vecb.txt");
    std::fs::write(&vec, "x\n").unwrap();
    let src = format!(
        "module m; initial begin int fd; int a;\n\
         fd=$fopen(\"{}\", \"r\"); a=$feof(fd)+7; $display(\"o=%0d\", a); $fclose(fd); end endmodule",
        vec.to_str().unwrap()
    );
    assert_eq!(run(&src).0, "o=7");
}

#[test]
fn fio_feof_direct_rhs_still_works() {
    // The pre-existing direct-rhs `e = $feof(fd)` form is untouched.
    let src = "module m; initial begin int fd; int e; fd=$fopen(\"none\",\"r\"); e=$feof(fd); $display(\"o=%0d\", e); end endmodule";
    assert!(ok(src)); // bad fd → -1, but elaborates + runs
}

#[test]
fn fio_side_effecting_in_condition_still_loud() {
    // A fd-ADVANCING read in a condition would double-advance → stays loud.
    let src = "module m; initial begin int fd; fd=$fopen(\"x\",\"r\");\n\
        while ($fgetc(fd) != -1) begin end end endmodule";
    assert!(!ok(src));
}

// ════════════════════════ bind — observer checker attach ════════════════════════

const BIND_DESIGN: &str = "module dut (input logic clk, input logic [3:0] d);\n\
    logic [3:0] r; always_ff @(posedge clk) r <= d;\n\
endmodule\n\
module chk (input logic clk, input logic [3:0] d);\n\
    always @(posedge clk) if (d >= 15) $display(\"o=VIOLATION\"); else $display(\"o=ok\");\n\
endmodule\n";

const BIND_TOP: &str = "module top;\n\
    logic clk = 0; logic [3:0] d = 0;\n\
    dut u (.clk(clk), .d(d));\n\
    initial begin d=3; #1 clk=1; #1 clk=0; d=15; #1 clk=1; #1 clk=0; $finish; end\n\
endmodule\n";

#[test]
fn bind_checker_observes_target() {
    // Bound checker sees dut's internal clk/d and fires at the d=15 edge.
    let src = format!("{BIND_DESIGN}bind dut chk u_chk (.clk(clk), .d(d));\n{BIND_TOP}");
    assert!(ok(&src));
    // first "o=" line is the d=3 edge → "ok"
    assert_eq!(run(&src).0, "o=ok");
}

#[test]
fn bind_equiv_manual_instance() {
    // bind ≡ manually instantiating the checker inside the target.
    let bound = format!("{BIND_DESIGN}bind dut chk u_chk (.clk(clk), .d(d));\n{BIND_TOP}");
    let manual = format!(
        "module dut (input logic clk, input logic [3:0] d);\n\
         logic [3:0] r; always_ff @(posedge clk) r <= d;\n\
         chk u_chk (.clk(clk), .d(d));\n\
         endmodule\n\
         module chk (input logic clk, input logic [3:0] d);\n\
         always @(posedge clk) if (d >= 15) $display(\"o=VIOLATION\"); else $display(\"o=ok\");\n\
         endmodule\n{BIND_TOP}"
    );
    let cmd = |s: &str| -> String {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("vita_r9b_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.sv");
        std::fs::write(&f, s).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f.to_str().unwrap())
            .current_dir(&dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.starts_with("o="))
            .collect::<Vec<_>>()
            .join("|")
    };
    assert_eq!(cmd(&bound), cmd(&manual));
    assert_eq!(cmd(&bound), "o=ok|o=VIOLATION");
}

#[test]
fn bind_output_port_checker_loud() {
    // A checker with an OUTPUT port would drive a target net (multi-driver) → loud.
    let src = "module dut (input logic clk); logic r; always_ff @(posedge clk) r<=1; endmodule\n\
        module chk (input logic clk, output logic o); assign o=clk; endmodule\n\
        bind dut chk u (.clk(clk), .o(r));\n\
        module top; logic clk=0; dut u(.clk(clk)); endmodule";
    assert!(!ok(src));
}

#[test]
fn bind_unknown_target_loud() {
    let src = "module chk (input logic clk); endmodule\n\
        bind nosuch chk u (.clk(clk));\n\
        module top; logic clk=0; endmodule";
    assert!(!ok(src));
}

// ════════════════════════ USTRUCT — scalar unpacked struct ════════════════════════

const KAT: &str = "package p;\n\
    typedef struct { string name; logic [4:0] mode; string msg_hex; int xof_out_bytes; } kat_t;\n\
endpackage\n";

#[test]
fn ustruct_member_read_write() {
    let src = format!(
        "{KAT}module m; initial begin p::kat_t k;\n\
         k.name=\"t0\"; k.mode=5'd3; k.msg_hex=\"abcd\"; k.xof_out_bytes=32;\n\
         $display(\"o=%s_%0d_%s_%0d\", k.name, k.mode, k.msg_hex, k.xof_out_bytes); end endmodule"
    );
    assert_eq!(run(&src).0, "o=t0_3_abcd_32");
}

#[test]
fn ustruct_member_arithmetic() {
    let src = format!(
        "{KAT}module m; initial begin p::kat_t k;\n\
         k.mode=5'd3; k.xof_out_bytes=32; k.mode=k.mode+5'd1; k.xof_out_bytes=k.xof_out_bytes*2;\n\
         $display(\"o=%0d_%0d\", k.mode, k.xof_out_bytes); end endmodule"
    );
    assert_eq!(run(&src).0, "o=4_64");
}

#[test]
fn ustruct_equiv_separate_vars() {
    // The desugar is exactly N separate member nets.
    let uns = format!(
        "{KAT}module m; initial begin p::kat_t k;\n\
         k.name=\"hi\"; k.mode=5'd7; $display(\"o=%s_%0d\", k.name, k.mode); end endmodule"
    );
    let sep = "module m; initial begin string k_name; logic [4:0] k_mode;\n\
        k_name=\"hi\"; k_mode=5'd7; $display(\"o=%s_%0d\", k_name, k_mode); end endmodule";
    assert_eq!(run(&uns).0, run(sep).0);
    assert_eq!(run(&uns).0, "o=hi_7");
}

#[test]
fn ustruct_array_now_soa() {
    // r18 (Fix A): a fixed array of a NON-packable record (`kat_t` has `string` members)
    // now lowers to struct-of-arrays (per-member native arrays `$unp$arr$field`), so the
    // declaration + element member access work (was loud — no aggregate storage).
    let src = format!(
        "{KAT}module m; initial begin p::kat_t arr[4]; arr[0].mode = 5'h1f; \
         $display(\"o=%h\", arr[0].mode); end endmodule"
    );
    assert!(ok(&src));
}

#[test]
fn ustruct_decl_init_still_loud() {
    let src = format!(
        "{KAT}module m; initial begin p::kat_t k = '{{\"a\", 5'd1, \"b\", 1}}; $display(\"o=x\"); end endmodule"
    );
    assert!(!ok(&src));
}

#[test]
fn ustruct_whole_copy_still_loud() {
    let src = format!(
        "{KAT}module m; initial begin p::kat_t k; p::kat_t k2; k.mode=5'd1; k2 = k; $display(\"o=x\"); end endmodule"
    );
    assert!(!ok(&src));
}

#[test]
fn ustruct_packed_struct_unaffected() {
    // The packed-struct path is byte-identical (regression guard).
    let src = "package p; typedef struct packed { logic [4:0] mode; logic [31:0] len; } cfg_t; endpackage\n\
        module m; initial begin p::cfg_t c; c.mode=5'd3; c.len=32'd128; $display(\"o=%0d_%0d\", c.mode, c.len); end endmodule";
    assert_eq!(run(src).0, "o=3_128");
}

#[test]
fn ustruct_dollar_name_no_alias() {
    // R1 adversarial: the member-net mangle must NOT alias a user identifier
    // spelled `k$mode` (`$` is legal MID-identifier). The `$unp$` prefix (which a
    // SV simple identifier cannot start with) keeps them distinct: k.mode=7,
    // k$mode=19 → "7_19", NOT the aliased "19_19".
    let src = "package p; typedef struct { logic [4:0] mode; int x; } t; endpackage\n\
        module top; initial begin p::t k; logic [4:0] k$mode;\n\
        k.mode=5'd7; k$mode=5'd19; $display(\"o=%0d_%0d\", k.mode, k$mode); end endmodule";
    assert_eq!(run(src).0, "o=7_19");
}
