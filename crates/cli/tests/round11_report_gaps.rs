//! Round-11 external report — gaps surfaced behind the round-10 parse wall while
//! bringing up the `tb/*.sv` verification environment. correct-or-loud throughout:
//!
//!   N1  output-direction `string` tf-formal (`output string s`)   → supported
//!   N1B inout-direction `string` tf-formal (`inout string s`)     → supported
//!   N2  `ref` tf-formal direction (`ref logic [31:0] h`)          → supported (→inout)
//!   N4  `void'(call);` discard-cast statement                     → supported
//!   N5  `string`-typed parameter/localparam                       → supported
//!   N5B untyped string-literal parameter (`localparam S = "x"`)   → supported
//!   R4  `string` function RETURN type (round-10 G4 follow-on)     → supported
//!
//! Oracles: N4/N5/N5B/R4 vs iverilog 13.0 (PASS); N1/N1B/N2 are hand-IEEE (iverilog
//! rejects function output/inout/ref formals) — vita renders the IEEE-correct value.
//! No sim-ir change (format_version stays 19). The engine `$sformatf` formatter is
//! refactored to run against `&SimState` so a subroutine body can render it (N1/R4);
//! this is byte-identical for every `$display`/`$write`/`$monitor` path (regressions
//! gated by the golden format tests).
//!
//!   N6  FIXED `string` ARRAY variable (`string files[0:1]`)      → supported (const idx)
//!   N6B `string` method on an indexed element (`files[i].len()`) → supported (parse+elem)
//!
//! Deep-storage gaps still correct-or-LOUD (documented follow-ons, NOT silent-wrong;
//! each needs storage/plumbing that does not exist yet):
//!   N3  array/queue of unpacked structs (`rec_t arr[]`)           → loud (heterogeneous heap)
//!   R2  dynamic-array tf-formal (`input byte b[]`)                → loud (frame dyn formal)
//!   R5  unpacked-struct typedef tf-port (`inout rec_t r`)         → loud (member desugar)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn workdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r11_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `src` through one-shot vita; return (first `PASS`/`x=`/`o=` stdout line, success).
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
        .find(|l| l.trim() == "PASS" || l.starts_with("x=") || l.starts_with("o="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

/// Does `src` fail to elaborate (loud, exit != 0)? Used for the correct-or-loud
/// deep-storage gaps: a clean rejection is the ACCEPTED outcome (never silent-wrong).
fn loud(src: &str) -> bool {
    !run(src).1
}

// ───────────────────────────── N1: output string formal ──────────────────
#[test]
fn n1_output_string_formal() {
    let src = "module t;\n\
        function automatic void f(input int n, output string s); s=$sformatf(\"%0d\",n); endfunction\n\
        initial begin string r; f(42,r); if(r==\"42\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n1b_inout_string_formal() {
    let src = "module t;\n\
        function automatic void f(input int n, inout string s); s=$sformatf(\"%0d\",n); endfunction\n\
        initial begin string r=\"\"; f(42,r); if(r==\"42\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n1_output_string_literal_write() {
    // Isolate the string OUTPUT formal from $sformatf: a bare literal write copies out.
    let src = "module t;\n\
        function automatic void f(output string s); s=\"hello\"; endfunction\n\
        initial begin string r; f(r); if(r==\"hello\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────────────────────── N2: ref formal ────────────────────────────
#[test]
fn n2_ref_formal() {
    let src = "module t;\n\
        function automatic void f(ref logic [31:0] h); h=32'h5; endfunction\n\
        initial begin logic [31:0] hh; f(hh); if(hh==32'h5) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n2_ref_mutate_in_place() {
    // `ref` = copy-in/copy-out: the callee reads AND updates the actual.
    let src = "module t;\n\
        function automatic void inc(ref int x); x=x+1; endfunction\n\
        initial begin int a=41; inc(a); if(a==42) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────────────────────── N4: void'(call) ───────────────────────────
#[test]
fn n4_void_cast_user_func() {
    let src = "module t;\n\
        function automatic int f(); return 7; endfunction\n\
        initial begin void'(f()); $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n4_void_cast_syscall() {
    // The dominant TB idiom: `void'($value$plusargs(...))` — the side effect runs.
    let src = "module t;\n\
        initial begin int v=9; void'($value$plusargs(\"N=%d\", v)); \
        if(v==9) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS"); // no +N plusarg → v unchanged, dest not written
}

// ───────────────────────────── N5: string parameter ──────────────────────
#[test]
fn n5_string_localparam() {
    let src = "module t;\n\
        localparam string S = \"abc\";\n\
        initial begin if(S==\"abc\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n5b_untyped_string_param() {
    let src = "module t;\n\
        localparam S = \"abc\";\n\
        initial begin if(S==\"abc\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n5_string_param_in_sformatf() {
    // A string param used as a $sformatf/$display arg renders its bytes, not a number.
    let src = "module t;\n\
        localparam string NAME = \"cpu\";\n\
        initial begin $display(\"x=%s\", NAME); $finish; end endmodule";
    assert_eq!(run(src).0, "x=cpu");
}

// ───────────────────────────── R4: string return ─────────────────────────
#[test]
fn r4_string_return() {
    let src = "module t;\n\
        function automatic string f(input int n); return $sformatf(\"%0d\",n); endfunction\n\
        initial begin if(f(42)==\"42\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r4_string_return_assign_and_len() {
    let src = "module t;\n\
        function automatic string hex(input int n); return $sformatf(\"%0h\",n); endfunction\n\
        initial begin string s; s=hex(255); \
        if(s==\"ff\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────────────────────── PASS guards (round-10 fixes hold) ──────────
#[test]
fn ok_input_string_formal_method() {
    let src = "module t;\n\
        function automatic int f(input string s); return s.len(); endfunction\n\
        initial begin if(f(\"abcdef\")==6) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn ok_output_logic_formal() {
    let src = "module t;\n\
        function automatic void f(input int n, output logic [31:0] v); v=n*2; endfunction\n\
        initial begin logic [31:0] r; f(21,r); if(r==42) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn ok_inout_logic_formal() {
    let src = "module t;\n\
        function automatic void f(inout logic [31:0] h); h=h+1; endfunction\n\
        initial begin logic [31:0] hh=4; f(hh); if(hh==5) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────── formatter-refactor regression: every sink still byte-identical ──
#[test]
fn fmt_refactor_display_specs_intact() {
    // The `$sformatf` formatter refactor (&Scheduler → &SimState) must render every
    // spec/flag byte-identically. Self-check against the exact expected string.
    let src = "module t;\n\
        initial begin string s; s=$sformatf(\"d=%0d h=%08h s=%s f=%8.2f\", 7, 43981, \"hi\", 3.14159); \
        if(s==\"d=7 h=0000abcd s=hi f=    3.14\") $display(\"PASS\"); else $display(\"x=[%s]\", s); \
        $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn fmt_refactor_sformatf_module_level_intact() {
    let src = "module t;\n\
        initial begin string s; s=$sformatf(\"%0d-%0h\", 10, 255); \
        if(s==\"10-ff\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────── deep-storage gaps: correct-or-LOUD (accepted, not silent) ──────
#[test]
fn n3_record_array_is_loud() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{1,2}, '{3,4} };\n\
        initial begin if(arr[1].a==3) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

// ───────────────────────────── N6/N6B: fixed string array ────────────────
#[test]
fn n6_string_array_read_write() {
    let src = "module t; string files [0:1];\n\
        initial begin files[0]=\"abcdef\"; files[1]=\"xy\"; \
        if(files[0]==\"abcdef\" && files[1]==\"xy\") $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n6b_method_on_array_element() {
    // iverilog compiles this but mis-computes `.len()` on an array element (its own
    // silent-wrong) — vita is IEEE-correct (len("abcdef")==6).
    let src = "module t; string files [0:1];\n\
        initial begin files[0]=\"abcdef\"; if(files[0].len()==6) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n6_string_array_size_form() {
    let src = "module t; string names [3];\n\
        initial begin names[0]=\"a\"; names[2]=\"ccc\"; \
        if(names[0]==\"a\" && names[1]==\"\" && names[2].len()==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n6_runtime_index_is_loud() {
    // A runtime element index (dynamic string array) stays loud — correct-or-loud.
    let src = "module t; string files [0:1]; int i;\n\
        initial begin i=0; files[i]=\"x\"; $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r2_dynamic_array_formal_is_loud() {
    let src = "module t;\n\
        function automatic int f(input byte b[]); return b.size(); endfunction\n\
        initial begin byte a[]; a=new[3]; if(f(a)==3) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r5_unpacked_struct_tfport_is_loud() {
    let src = "package p; typedef struct { string name; int count; } rec_t; endpackage\n\
        module t; import p::*;\n\
        function automatic int f(inout rec_t r); r.count=r.count+1; return r.count; endfunction\n\
        initial begin rec_t r; r.count=1; if(f(r)==2) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}
