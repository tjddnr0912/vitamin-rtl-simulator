//! Round-10 external report — twelve gaps between vita and running the existing
//! `tb/*.sv` verification environment. Fixed here (correct-or-loud throughout):
//!
//!   G1  string METHOD on a `string` formal (`s.len()`)          → supported
//!   G2  dynamic-array (`[]`) subroutine formal                  → INPUT read-only SUPPORTED (round-11 R2)
//!   G3  signed-element sized unpacked-array formal (`byte b[]`) → supported (sign)
//!   G4  `string` function RETURN type                          → parses; call loud
//!   G5  unpacked-struct typedef as a tf-port                   → SUPPORTED (round-11 R5)
//!   G6  bare struct typedef var after `import p::*`            → supported
//!   G6B `p::rec_t` struct var at MODULE scope                  → supported
//!   G7  `@(posedge clk iff en)` event guard                    → supported (desugar)
//!   G8  method chained on a method result (`s.substr().atoi()`)→ supported
//!   G9  labeled concurrent assertion (`lbl : assert property`) → supported
//!   G10 named tf-call arguments (`f(1, .b(2))`)                → supported
//!   G11 time-unit literals in delays (`#1ns`, `#(3*1ns)`)      → supported (scaled)
//!   G12 per-file filename+line in multi-file diagnostics       → fixed
//!
//! Oracles: G1/G3(sign)/G7/G8/G10/G11 verified vs iverilog 13.0 (or its equivalent
//! form where iverilog rejects the exact syntax); G2/G4/G5 are correct-or-loud
//! (iverilog "sorry"/rejects, vita is loud, no silent-wrong). No sim-ir change
//! (format_version stays 19); AST adds `FunctionDef.ret_string`, `EventExpr.iff`, and
//! the `TimeLit`/`NamedArg`/`MethodCall` `ExprKind` variants (schema-hash re-pin).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn workdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r10_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `src` through one-shot vita; return (first `o=`/`y=`/`v=`/`PASS` stdout line,
/// success).
fn run(src: &str) -> (String, bool) {
    let d = workdir();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| {
            l.starts_with("o=") || l.starts_with("y=") || l.starts_with("v=") || l.trim() == "PASS"
        })
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

/// Does `src` elaborate + simulate successfully (exit 0)?
fn ok(src: &str) -> bool {
    run(src).1
}

/// (stdout, stderr, success) for a MULTI-file one-shot run (G12).
fn run_files(files: &[(&str, &str)]) -> (String, String, bool) {
    let d = workdir();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    for (name, body) in files {
        let f = d.join(name);
        std::fs::write(&f, body).unwrap();
        cmd.arg(f.to_str().unwrap());
    }
    let out = cmd.current_dir(&d).output().expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

// ───────────────────────────── G1: string method on a formal ─────────────
#[test]
fn g1_string_method_on_frame_formal() {
    let src = "module m(output int o);\n\
        function automatic int f(input string s); return s.len(); endfunction\n\
        initial begin o=f(\"abcd\"); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=4");
}

#[test]
fn g1_string_atoi_on_formal() {
    let src = "module m(output int o);\n\
        function automatic int f(input string s); return s.atoi(); endfunction\n\
        initial begin o=f(\"42abc\"); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=42");
}

#[test]
fn g1_string_compare_regression() {
    // The whole-read compare path must keep working (not regressed by the method route).
    let src = "module m(output int o);\n\
        function automatic bit f(input string s); return (s == \"hi\"); endfunction\n\
        initial begin o=f(\"hi\"); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=1");
}

// ────────── G2: dyn-array INPUT formal — now SUPPORTED (round-11 R2 supersede) ─
#[test]
fn g2_dyn_array_input_formal_now_supported() {
    // round-10 kept a dynamic-array (`[]`) formal loud (runtime-sized, outside the
    // fixed md-packed slot model). round-11 R2 supports a READ-ONLY `input` one by
    // ALIASING the caller's DynArray net (the read-only function inlines; `b[i]`/
    // `.size()` read the caller's heap). A WRITE / inout / output stays loud.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] b[]); return b[0]; endfunction\n\
        logic [7:0] a[];\n\
        initial begin a=new[2]; a[0]=8'h5a; o=f(a); $display(\"o=%h\",o); end endmodule";
    let (out, _err, success) = run_files(&[("t.sv", src)]);
    assert!(
        success,
        "a read-only input dyn-array formal is now supported (R2)"
    );
    assert!(out.contains("o=5a"), "got:\n{out}");
}

// ───────────────────────────── G3: signed-element formal ──────────────────
#[test]
fn g3_signed_byte_element_negative() {
    // The sign-critical case — a negative `byte` element reads negative, NOT 255.
    let src = "module m(output int o);\n\
        function automatic int f(input byte b[0:3], input int i); return b[i]; endfunction\n\
        byte g[0:3];\n\
        initial begin g[1]=-1; o=f(g,1); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=-1");
}

#[test]
fn g3_unsigned_element_regression() {
    // The unsigned twin stays unsigned (the $signed re-stamp is confined to signed
    // elements) — `byte unsigned` 0xFF reads 255, not -1.
    let src = "module m(output int o);\n\
        function automatic int f(input byte unsigned b[0:1], input int i); return b[i]; endfunction\n\
        byte unsigned g[0:1];\n\
        initial begin g[0]=8'hFF; o=f(g,0); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=255");
}

// ─────────────────────── G4: string return (now SUPPORTED, round-14 V1) ────
#[test]
fn g4_string_return_now_supported() {
    // round-10's primary complaint was an E2002 PARSE cascade; round-11 cleared parse
    // and it was loud because the frame-local `string s` was a Wire slot (E3018 on the
    // assign). round-14 V1 gives frame-local strings a real `NetKind::String` heap slot
    // (`frame_local_net_kind`) + frame-aware `str_bytes`, so build-and-return works:
    // f(42) == "42" ⇒ o=1. (The pkg-scoped and class-method string-return paths below
    // stay loud — a separate materialization gap.)
    let src = "module m(output int o);\n\
        function automatic string f(input int n); string s; s=$sformatf(\"%0d\",n); return s; endfunction\n\
        initial begin o=(f(42)==\"42\"); $display(\"o=%0d\",o); end endmodule";
    let (out, code) = run(src);
    assert!(code, "expected supported, got:\n{out}");
    assert_eq!(out, "o=1", "string build-and-return, got:\n{out}");
}

#[test]
fn g4_pkg_string_return_loud() {
    // Adversarial-review regression: the ret_string loud guard must also cover the
    // package-scoped call path (`pkg::f()`) — else a string return there silently reads
    // empty and flips a `==` comparison (a HIGH silent-wrong).
    let src = "package p; function string g(); g=\"PKG\"; endfunction endpackage\n\
        module top(output int o); initial begin o=(p::g()==\"PKG\"); $display(\"o=%0d\",o); $finish; end endmodule";
    assert!(!ok(src), "a package string return must be loud");
}

#[test]
fn g4_class_method_string_return_loud() {
    // …and the class-method call path (`c.m()`).
    let src = "module top(output int o);\n\
        class C; function string name(); name=\"obj\"; endfunction endclass\n\
        C c; initial begin c=new(); o=(c.name()==\"obj\"); $display(\"o=%0d\",o); $finish; end endmodule";
    assert!(!ok(src), "a class-method string return must be loud");
}

// ───────────── G5: unpacked-struct tf-port — now SUPPORTED (R5 supersede) ─────
#[test]
fn g5_unpacked_struct_tf_port_now_supported() {
    // round-10 kept an unpacked-struct typedef tf-port a single clean loud; round-11
    // R5 (R5-A) lifts it to SUPPORTED — the record port expands to one member formal
    // per field (reusing the scalar unpacked-struct member desugar). A FUNCTION takes
    // an input struct formal; a TASK gets full inout copy-in/out (see round11 R5-A
    // tests). (A FUNCTION inout/output formal stays loud — a separate general gap.)
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module m; import p::*;\n\
        function automatic int f(input rec_t r); return r.a; endfunction\n\
        initial begin rec_t r; r.a=7; if(f(r)==7) $display(\"PASS\"); $finish; end endmodule";
    let (out, _err, success) = run_files(&[("t.sv", src)]);
    assert!(success, "unpacked-struct tf-port is now supported (R5)");
    assert!(out.contains("PASS"), "got:\n{out}");
}

// ───────────────────────────── G6 / G6B: struct typedef var ───────────────
#[test]
fn g6_bare_struct_var_after_import() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module m(output int o); import p::*;\n\
        initial begin rec_t r; r.a=7; o=r.a; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=7");
}

#[test]
fn g6b_scoped_struct_var_module_scope() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module m(output int o);\n\
        p::rec_t r;\n\
        initial begin r.a=9; o=r.a; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=9");
}

#[test]
fn g6_record_array_stays_loud() {
    // A record ARRAY still needs aggregate storage vita lacks → loud (correct-or-loud).
    let src = "package p; typedef struct { int a; } rec_t; endpackage\n\
        module m; import p::*; rec_t arr[0:1]; initial $display(\"x\"); endmodule";
    assert!(!ok(src), "a record array must stay loud");
}

// ───────────────────────────── G7: iff event guard ────────────────────────
#[test]
fn g7_iff_matches_manual_desugar() {
    // `@(posedge clk iff en) S` ≡ `@(posedge clk) if (en) S`. Both counters must agree.
    let src = "module m(output int o);\n\
        logic clk=0, en=0; int ci=0, cm=0;\n\
        always #5 clk=~clk;\n\
        always @(posedge clk iff en) ci<=ci+1;\n\
        always @(posedge clk) if (en) cm<=cm+1;\n\
        initial begin en=0; #12 en=1; #40 en=0; #20 o=(ci==cm)?ci:-1; $display(\"o=%0d\",o); $finish; end\n\
        endmodule";
    // ci==cm and both counted the enabled posedges (4).
    assert_eq!(run(src).0, "o=4");
}

#[test]
fn g7_iff_oneshot_wait_fires_at_guarded_edge() {
    // `@(posedge clk iff rdy);` must WAIT until a posedge WHERE rdy is true
    // (IEEE §9.4.2.3), NOT unblock at the first posedge. rdy rises at t=12, so the
    // first posedge with rdy=1 is t=15 (NOT t=5). (Adversarial-review regression: the
    // earlier `@(posedge clk) if(rdy)` desugar silently fired at t=5.) Matches the
    // iverilog `do @(posedge clk); while(!rdy);` oracle (also t=15).
    let src = "module m(output int o);\n\
        logic clk=0, rdy=0;\n\
        always #5 clk=~clk;\n\
        initial begin #12 rdy=1; #40 $finish; end\n\
        initial begin @(posedge clk iff rdy); o=$time; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=15");
}

#[test]
fn g7_iff_guarded_body_runs_at_guarded_edge() {
    // The wait+body form runs the body at the FIRST en-true posedge (t=15, capturing
    // v=22), not falling through at t=5 with x unchanged.
    let src = "module m(output int o);\n\
        logic clk=0, en=0; int v=22;\n\
        always #5 clk=~clk;\n\
        initial begin #12 en=1; #40 $finish; end\n\
        initial begin int x=0; @(posedge clk iff en) x=v; o=x; $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=22");
}

#[test]
fn g7_iff_multiterm_is_loud() {
    // A multi-term list with a per-term guard can't be one body-wrap → loud.
    let src = "module m;\n\
        logic clk=0, rst=0, en=0; int c=0;\n\
        always #5 clk=~clk;\n\
        always @(posedge clk iff en or posedge rst) c<=c+1;\n\
        initial #20 $finish; endmodule";
    assert!(!ok(src), "a multi-term iff guard must be loud");
}

// ───────────────────────────── G8: chained method call ────────────────────
#[test]
fn g8_chain_substr_atoi() {
    let src = "module m(output int o);\n\
        initial begin string s; s=\"ab127cd\"; o=s.substr(2,4).atoi(); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=127");
}

#[test]
fn g8_chain_equals_split() {
    // The chain must equal the split-via-temp form (which iverilog accepts).
    let chain = run("module m(output int o);\n\
        initial begin string s; s=\"xy089z\"; o=s.substr(2,4).atoi(); $display(\"o=%0d\",o); end endmodule")
    .0;
    let split = run("module m(output int o);\n\
        initial begin string s,t; s=\"xy089z\"; t=s.substr(2,4); o=t.atoi(); $display(\"o=%0d\",o); end endmodule")
    .0;
    assert_eq!(chain, split);
    assert_eq!(chain, "o=89");
}

// ───────────────────────────── G9: labeled assertion ──────────────────────
#[test]
fn g9_labeled_assert_property() {
    let src = "module m;\n\
        logic clk=0; logic [6:0] w=7'd10;\n\
        always #5 clk=~clk;\n\
        property p_le; @(posedge clk) (w <= 7'd64); endproperty\n\
        a_lbl : assert property (p_le) else $error(\"bad\");\n\
        initial begin #40 $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn g9_stray_label_still_loud() {
    // A leading `IDENT :` on a NON-assert item must still error (the gate is closed).
    let src = "module m; logic x; foo : bar baz; initial $display(\"x\"); endmodule";
    assert!(!ok(src), "a stray label must stay loud");
}

// ───────────────────────────── G10: named call args ───────────────────────
#[test]
fn g10_positional_then_named() {
    let src = "module m(output int o);\n\
        function automatic int f(input int a, input int b=7); return a*10+b; endfunction\n\
        initial begin o=f(1, .b(2)); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=12");
}

#[test]
fn g10_all_named_out_of_order() {
    let src = "module m(output int o);\n\
        function automatic int f(input int a, input int b=7); return a*10+b; endfunction\n\
        initial begin o=f(.b(5), .a(2)); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=25");
}

#[test]
fn g10_named_uses_default() {
    let src = "module m(output int o);\n\
        function automatic int f(input int a, input int b=7); return a*10+b; endfunction\n\
        initial begin o=f(.a(3)); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=37");
}

#[test]
fn g10_unknown_formal_is_loud() {
    let src = "module m(output int o);\n\
        function automatic int f(input int a, input int b=7); return a+b; endfunction\n\
        initial begin o=f(1, .zzz(2)); $display(\"o=%0d\",o); end endmodule";
    assert!(!ok(src), "an unknown named formal must be loud");
}

// ───────────────────────────── G11: time-unit literals ────────────────────
#[test]
fn g11_time_literal_scales_by_timescale() {
    // `#1ns` and `#(3*1ns)` scale to the module's time unit. Under 1ns/1ns the $time
    // deltas are 1 and 3 (finish at 4).
    let src = "`timescale 1ns/1ns\n\
        module m; initial begin #1ns; #(3*1ns); $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn g11_time_literal_ps_scale() {
    // Under 1ps/1ps, `#1ns` advances 1000 ticks.
    let src = "`timescale 1ps/1ps\n\
        module m(output int o); initial begin #1ns; o=$time; $display(\"o=%0d\",o); $finish; end endmodule";
    assert_eq!(run(src).0, "o=1000");
}

#[test]
fn g11_plain_delay_and_unit_named_signal_regression() {
    // A plain `#5` and a signal named like a unit (`ns`, used non-adjacent) are unaffected.
    let src = "module m(output int o);\n\
        logic [3:0] ns;\n\
        initial begin ns=4; #5 o=ns; $display(\"o=%0d\",o); #(ns) $finish; end endmodule";
    assert_eq!(run(src).0, "o=4");
}

// ───────────────────────────── G12: multi-file diagnostics ────────────────
#[test]
fn g12_multifile_reports_correct_file_and_line() {
    // a.sv (12 lines, valid) + b.sv (error on LOCAL line 4). Pre-fix: named a.sv with a
    // concat-global line. Now: names b.sv:4.
    let a = "module a;\n  logic x;\n  logic y;\n  initial begin\n    x=0;\n    y=1;\n    #1;\n    x=y;\n    $display(\"a\");\n    $finish;\n  end\nendmodule\n";
    let b =
        "module b;\n  logic z;\n  initial begin\n    z = = 0;\n    $finish;\n  end\nendmodule\n";
    let (_out, err, success) = run_files(&[("a.sv", a), ("b.sv", b)]);
    assert!(!success);
    assert!(
        err.contains("b.sv:4"),
        "diagnostic must name b.sv:4, got:\n{err}"
    );
    assert!(
        !err.contains("a.sv:"),
        "diagnostic must NOT name a.sv, got:\n{err}"
    );
}

#[test]
fn g12_multifile_cross_file_macro_and_pkg() {
    // Shared-compilation-unit invariant: a `\`define` and a package in file 1 are visible
    // in file 2 (the multi-source preprocess keeps them, like the old concatenation).
    let f1 = "`define WIDTH 8\npackage pk; localparam int K = 42; endpackage\n";
    let f2 = "module top(output int o);\n  import pk::*;\n  logic [`WIDTH-1:0] r;\n  initial begin r=K[7:0]; o=r; $display(\"o=%0d\",o); $finish; end\nendmodule\n";
    let (out, _err, success) = run_files(&[("f1.sv", f1), ("f2.sv", f2)]);
    assert!(success, "cross-file macro + package must resolve");
    assert!(out.contains("o=42"), "got:\n{out}");
}

// ───────────────── design-structure exports (--hier-tree / --inst-paths) ──────
/// Run vita with the two design-structure export flags; return (tree, paths) contents.
fn run_hier(src: &str) -> (String, String) {
    let d = workdir();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let tree = d.join("h.tree");
    let paths = d.join("h.paths");
    Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--hier-tree")
        .arg(&tree)
        .arg("--inst-paths")
        .arg(&paths)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        std::fs::read_to_string(&tree).unwrap_or_default(),
        std::fs::read_to_string(&paths).unwrap_or_default(),
    )
}

#[test]
fn feature_hier_tree_module_names() {
    let src = "module leaf(input logic a, output logic b); assign b=a; endmodule\n\
        module mid(input logic i, output logic o); leaf u_leaf(.a(i),.b(o)); endmodule\n\
        module top; logic c=0,x,y; mid u_m1(.i(c),.o(x)); mid u_m2(.i(c),.o(y));\n\
        initial begin #1 $finish; end endmodule";
    let (tree, _paths) = run_hier(src);
    // Indented `instance : module` tree, top at the root.
    assert!(tree.starts_with("top : top\n"), "tree:\n{tree}");
    assert!(tree.contains("  u_m1 : mid\n"), "tree:\n{tree}");
    assert!(tree.contains("    u_leaf : leaf\n"), "tree:\n{tree}");
    assert!(tree.contains("  u_m2 : mid\n"), "tree:\n{tree}");
}

#[test]
fn feature_inst_paths_full_dotted() {
    let src = "module leaf(input logic a, output logic b); assign b=a; endmodule\n\
        module mid(input logic i, output logic o); leaf u_leaf(.a(i),.b(o)); endmodule\n\
        module top; logic c=0,x,y; mid u_m1(.i(c),.o(x)); mid u_m2(.i(c),.o(y));\n\
        initial begin #1 $finish; end endmodule";
    let (_tree, paths) = run_hier(src);
    // Full dotted path per instance, top-down, copy/paste-ready.
    for p in [
        "top\n",
        "top.u_m1\n",
        "top.u_m1.u_leaf\n",
        "top.u_m2\n",
        "top.u_m2.u_leaf\n",
    ] {
        assert!(paths.contains(p), "missing {p:?} in paths:\n{paths}");
    }
}
