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
//!   R5  unpacked-struct typedef tf-port (`inout rec_t r`)         → SUPPORTED
//!       R5-A: the port expands to per-member formals (parser 1→N). R5-B: a
//!       FUNCTION with output/inout formals now copies out (hoist → task-terminator
//!       path + return-capture); a one-shot-hoist-unsafe position stays loud.
//!
//!   R2  dynamic-array INPUT tf-formal (`input byte b[]`)          → SUPPORTED (read-only alias)
//!       A read-only `input` dyn-array formal aliases the caller's DynArray net
//!       (`b.size()`/`b[i]` read the caller's heap; the fn is inlined). A WRITE to
//!       the formal / inout / output stays loud.
//!
//!   N3  dynamic array of a PACKABLE record (`rec_t arr[]`)        → READ supported
//!       A packable (all-integral) record is a packed layout: `rec_t arr[]` is one wide
//!       DynArray net, `arr[i].field` a part-select. Decl + `'{…}` init + member READ +
//!       `.size()`/`new[]` work. A member/whole-element WRITE, and a NON-packable record
//!       (string/real member → heterogeneous heap), stay loud.
//!
//! Deep-storage gaps still correct-or-LOUD (documented follow-ons, NOT silent-wrong):
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
fn n3_packable_record_array_read_supported() {
    // N3: a dynamic array of a PACKABLE record (all-integral members) is now supported
    // — the record is a packed layout, so `rec_t arr[]` is one wide `logic` DynArray net
    // and `arr[i].field` is a part-select on the element. Decl + `'{…}` init + member
    // READ (const & runtime index) all compose from existing dyn-array + part-select
    // machinery (parser-only). (hand-IEEE — iverilog rejects it.)
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{1,2}, '{3,4} };\n\
        initial begin if(arr[1].a==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_record_array_both_members_and_runtime_index() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{1,2}, '{3,4} };\n\
        initial begin int i=1; if(arr[0].a==1 && arr[0].b==2 && arr[i].a==3 && arr[i].b==4) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_record_array_new_and_size() {
    let src = "package p; typedef struct { logic [7:0] x; logic [7:0] y; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[3]; if(arr.size()==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// N3 Phase 3 heterogeneous heap: a string/real-member record array is now SUPPORTED —
// each member lowers to its OWN typed dyn array (`$unp$arr$field`: string → string dyn,
// real → real dyn, int → int dyn), so `arr[i].field` is a native per-field dyn element.
// decl-init `'{ '{…},… }` rides the per-field var-init flush.
#[test]
fn n3b_string_member_record_array_supported() {
    let src = "package p; typedef struct { string s; int n; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{\"hi\",1}, '{\"bye\",2} };\n\
        initial begin if(arr[0].s==\"hi\" && arr[0].n==1 && arr[1].s==\"bye\" && arr[1].n==2) \
        $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_string_member_record_new_and_writes() {
    let src = "package p; typedef struct { string s; int n; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[2]; arr[0].s=\"hello\"; arr[0].n=7; arr[1]='{\"wo\",9}; \
        if(arr[0].s==\"hello\" && arr[0].n==7 && arr[1].s==\"wo\" && arr[1].n==9 && arr.size()==2) \
        $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_real_member_record_array_supported() {
    let src = "package p; typedef struct { real x; int y; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{1.5, 3}, '{-2.5, 4} };\n\
        initial begin arr[0].x=9.25; \
        if(arr[0].x==9.25 && arr[0].y==3 && arr[1].x==-2.5 && arr[1].y==4) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// N3 Phase 2: standalone `string s[]` / `real r[]` dynamic arrays (the per-field building
// block) — new[]/element r-w/default ""/0.0/size/decl-init.
#[test]
fn n3_standalone_string_dyn_array() {
    let src = "module t; string s[] = '{\"a\", \"bb\", \"ccc\"};\n\
        initial begin s[1]=\"changed\"; if(s[0]==\"a\" && s[1]==\"changed\" && s[2]==\"ccc\" && s.size()==3) \
        $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3_standalone_real_dyn_array() {
    let src = "module t; real r[];\n\
        initial begin r=new[3]; r[0]=1.5; r[1]=-2.25; \
        if(r[0]==1.5 && r[1]==-2.25 && r[2]==0.0 && r.size()==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// correct-or-loud: an `event`-member record (no dyn-array element form) stays loud.
#[test]
fn n3b_event_member_record_array_is_loud() {
    let src = "package p; typedef struct { event e; int n; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[1]; $finish; end endmodule";
    assert!(loud(src));
}

// N3 WRITE path: a record-array element MEMBER write `arr[i].field = v` is a part-select
// on the dyn element (engine read-modify-write) — the sibling field is preserved.
#[test]
fn n3b_record_array_member_write_supported() {
    let src = "package p; typedef struct { logic [7:0] a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{8'h11, 8'h22} };\n\
        initial begin arr[0].a=8'hAB; if(arr[0].a==8'hAB && arr[0].b==8'h22) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// member write over a signed member + runtime index; `new[]`'d then written.
#[test]
fn n3b_record_array_member_write_signed_runtime_index() {
    let src = "package p; typedef struct { byte a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin int i=1; arr=new[2]; arr[0].a=-5; arr[i].b=8; \
        if(arr[0].a==-5 && arr[1].b==8) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// whole-element write `arr[i] = '{…}` — the pattern desugars to a packed concat, then a
// whole-element dyn write (already supported).
#[test]
fn n3b_record_array_whole_element_write() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[2]; arr[0]='{5,6}; arr[1]='{-3,100}; \
        if(arr[0].a==5 && arr[0].b==6 && arr[1].a==-3 && arr[1].b==100) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// correct-or-loud: a member SUB-select write (`arr[i].a[3:0] = v`) has no dbase remap on
// the write path → loud, never a silent wrong.
#[test]
fn n3b_record_array_member_subselect_write_is_loud() {
    let src = "package p; typedef struct { logic [7:0] a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[1]; arr[0].a[3:0]=4'hF; $finish; end endmodule";
    assert!(loud(src));
}

// An all-2-state record's member write coerces X/Z→0 (IEEE §6.11.3), matching the
// whole-element `'{…}` desugar; a 4-state member correctly preserves X.
#[test]
fn n3b_record_array_two_state_member_write_coerces_x() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr []; logic [31:0] xv='x;\n\
        initial begin arr=new[1]; arr[0].a=xv; if(arr[0].a===0) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_record_array_four_state_member_write_preserves_x() {
    let src = "package p; typedef struct { logic [7:0] a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr []; logic [7:0] xv='x;\n\
        initial begin arr=new[1]; arr[0]='{0,0}; arr[0].a=xv; \
        if(arr[0].a===8'hxx && arr[0].b==0) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// N3 heterogeneous heap (SoA), Phase 1: a MIXED 2-/4-state record (a 2-state `int`/`bit`
// member AND a 4-state `logic` member) is now supported — each member lowers to its OWN
// typed dyn array (`$unp$arr$field`), so a fresh `new[]` `int` field reads 0 and a `logic`
// field reads X, with correct per-field semantics (this was the soundness RANK-1 silent-
// wrong under the single-net packed path). Member read/write, `new[]`, `'{…}`, `.size()`.
#[test]
fn n3b_mixed_two_four_state_record_array_soa() {
    // fresh new[] → 2-state field defaults 0 (not X), 4-state field defaults X.
    let src = "package p; typedef struct { int cnt; logic [7:0] flags; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[4]; \
        if(arr[2].cnt==0 && arr[2].flags===8'hxx && arr[2].cnt+1==1) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_mixed_record_member_write_and_pattern_and_size() {
    let src = "package p; typedef struct { int cnt; logic [7:0] flags; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[3]; arr[0]='{5,8'hAA}; arr[1].cnt=-3; arr[1].flags=8'hBB; \
        if(arr[0].cnt==5 && arr[0].flags==8'hAA && arr[1].cnt==-3 && arr[1].flags==8'hBB && arr.size()==3) \
        $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn n3b_mixed_record_decl_init_and_sibling_isolation() {
    let src = "package p; typedef struct { int a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{9, 8'h11}, '{7, 8'h22} };\n\
        initial begin arr[0].a=42; \
        if(arr[0].a==42 && arr[0].b==8'h11 && arr[1].a==7 && arr[1].b==8'h22) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// correct-or-loud: a WHOLE-element read of a SoA record (`arr[i]` as a value) has no flat
// surface across the per-field dyn arrays → loud, never a silent wrong.
#[test]
fn n3b_mixed_record_whole_element_read_is_loud() {
    let src = "package p; typedef struct { int a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[2]; if(arr[0]==arr[1]) $display(\"x\"); $finish; end endmodule";
    assert!(loud(src));
}

// N3 Phase 3: a fresh `new[]` string-member record defaults each field to its IEEE type
// default — a `string` field to "" and an `int` field to 0.
#[test]
fn n3b_string_member_record_new_defaults() {
    let src = "package p; typedef struct { string s; int n; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[1]; if(arr[0].s==\"\" && arr[0].n==0) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// Adversarial soundness review RANK 1: a SIGNED member must read back sign-extended
// (the whole-field read is `$signed`-wrapped, mirroring the scalar packed-struct path)
// — else `arr[i].a * arr[i].b` / `<` on negatives were silently unsigned.
#[test]
fn n3b_record_array_signed_member_sign_extends() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{-3, 4} };\n\
        initial begin if(arr[0].a * arr[0].b == -12 && arr[0].a < 0) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// RANK 2: an all-2-state record (`bit`/`byte`/`int` members) defaults its fields to 0,
// not X (the DynArray net is `Bit`, not `Logic`) — a `new[n]`'d element reads 0.
#[test]
fn n3b_record_array_all_two_state_defaults_zero() {
    let src = "package p; typedef struct { byte a; int b; bit [3:0] c; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [];\n\
        initial begin arr=new[1]; if(arr[0].a==0 && arr[0].b==0 && arr[0].c==0) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// RANK 3: a sub-select of a NON-zero-LSB member is correct-or-LOUD (the `[w-1:0]`
// normalization does not remap the member's declared base) — never a silent X. A
// zero-LSB member sub-select stays correct (unchanged).
#[test]
fn n3b_record_array_nonzero_lsb_member_subselect_is_loud() {
    let src = "package p; typedef struct { logic [15:8] a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{8'hA5, 8'h11} };\n\
        initial begin $display(\"%h\", arr[0].a[11:8]); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn n3b_record_array_zero_lsb_member_subselect_ok() {
    let src = "package p; typedef struct { logic [7:0] a; logic [7:0] b; } rec_t; endpackage\n\
        module t; import p::*; rec_t arr [] = '{ '{8'hA5, 8'h11} };\n\
        initial begin if(arr[0].a[3:0]==4'h5 && arr[0].b[7:4]==4'h1) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
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
fn r2_dynamic_array_input_formal_supported() {
    // R2: a READ-ONLY `input` dyn-array formal is now supported — it ALIASES the
    // caller's DynArray net (the read-only function is routed to the inline path,
    // where `b.size()`/`b[i]` resolve against the caller's `dyn_heap` — no copy, no
    // engine change). The exact report src runs. (hand-IEEE — iverilog rejects it.)
    let src = "module t;\n\
        function automatic int f(input byte b[]); return b.size(); endfunction\n\
        initial begin byte a[]; a=new[3]; if(f(a)==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r2b_element_read_and_sum_supported() {
    let src = "module t;\n\
        function automatic int f(input byte b[]); return b[0]+b[1]+b[2]; endfunction\n\
        initial begin byte a[]; a=new[3]; a[0]=1; a[1]=2; a[2]=3; if(f(a)==6) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// correct-or-loud: the alias is READ-ONLY. Any WRITE to the dyn formal, an
// inout/output dyn formal, or a non-straight-line body keeps the function FRAMED,
// where the dyn formal is loud-rejected — never a silent caller mutation.
#[test]
fn r2b_write_element_is_loud() {
    let src = "module t;\n\
        function automatic int f(input byte b[]); b[0]=9; return b.size(); endfunction\n\
        initial begin byte a[]; a=new[3]; if(f(a)==3) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r2b_new_in_body_is_loud() {
    let src = "module t;\n\
        function automatic int f(input byte b[]); b=new[5]; return b.size(); endfunction\n\
        initial begin byte a[]; a=new[3]; if(f(a)==5) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r2b_inout_dyn_formal_is_loud() {
    let src = "module t;\n\
        function automatic int f(inout byte b[]); return b.size(); endfunction\n\
        initial begin byte a[]; a=new[3]; if(f(a)==3) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r2b_signedness_mismatch_actual_is_loud() {
    // The alias reads the CALLER net's storage, so `b[i]` takes the caller's
    // signedness. A signed `byte b[]` formal fed an UNSIGNED `byte a[]` actual would
    // read 0xFF as 255 (unsigned) instead of the IEEE-correct -1 (the formal's signed
    // element type). Loud on the width/signedness mismatch, never a silent wrong sign.
    let src = "module t;\n\
        function automatic int f(input byte b[]); return b[0]; endfunction\n\
        initial begin byte unsigned a[]; a=new[1]; a[0]=8'hFF; if(f(a)==-1) $display(\"PASS\"); $finish; end endmodule";
    assert!(loud(src));
}

// Adversarial-review (soundness) silent-wrongs, fixed:
#[test]
fn r2b_two_state_return_coerces_xz() {
    // The R2 carve-out forces a 2-state-return fn onto the inline path, which lacks the
    // frame return slot's X/Z→0 coercion (§6.11.3). An `int` return of a 4-state
    // element read (`logic b[]`, unwritten → X) must coerce to 0, not leak X. Coerced
    // in `inline_resolved_func`. (A 4-state `logic` return correctly PRESERVES X.)
    let src = "module m;\n\
        function automatic int r2(input logic [7:0] b[]); return b[0]; endfunction\n\
        logic [7:0] a[]; initial begin a=new[1]; a[0]=8'b0001_000x; if(r2(a)==16) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r2b_formal_shadows_outer_same_named_net() {
    // The dyn-array formal must SHADOW an outer same-named dyn net — `dyn_handle_read`
    // consults the `dyn_subst` alias BEFORE the scoped net lookup, so a module-level
    // `int b[]` (size 100) does not win over the formal `b` aliased to actual `a`
    // (size 3).
    let src = "module m;\n\
        int b[];\n\
        function automatic int r2(input int b[]); return b.size(); endfunction\n\
        int a[]; initial begin b=new[100]; a=new[3]; if(r2(a)==3) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// R5-B: a FUNCTION with an output/inout formal is now SUPPORTED. Its call carries
// copy-in + copy-out (like a task) PLUS a return value, so it lowers to a
// `Terminator::Call` (via `emit_frame_func_out_call`) — hoisted out of a
// once-evaluated expression to a temp when nested. The EXACT report src (a function
// with an inout struct formal used in an `if` condition) now runs.
#[test]
fn r5_unpacked_struct_inout_function_supported() {
    let src = "package p; typedef struct { string name; int count; } rec_t; endpackage\n\
        module t; import p::*;\n\
        function automatic int f(inout rec_t r); r.count=r.count+1; return r.count; endfunction\n\
        initial begin rec_t r; r.count=1; if(f(r)==2) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// R5-B copy-out is OBSERVED: the caller's actual is written back (hand-IEEE — iverilog
// rejects function output/inout formals; vita renders the IEEE-correct value).
#[test]
fn r5b_scalar_inout_copyout_observed() {
    let src = "module t;\n\
        function automatic int inc(inout int a); a=a+1; return a*10; endfunction\n\
        initial begin int x=5; int r; r=inc(x); if(r==60 && x==6) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r5b_struct_inout_copyout_observed() {
    let src = "package p; typedef struct { int a; int b; } rec_t; endpackage\n\
        module t; import p::*;\n\
        function automatic int f(inout rec_t r); r.a=r.a+1; r.b=r.b+2; return r.a+r.b; endfunction\n\
        initial begin rec_t r; int y; r.a=10; r.b=20; y=f(r); if(y==33 && r.a==11 && r.b==22) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r5b_output_formal_supported() {
    let src = "module t;\n\
        function automatic int mk(output int a); a=42; return 7; endfunction\n\
        initial begin int x; int r; r=mk(x); if(r==7 && x==42) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// correct-or-loud boundaries: positions where a one-shot hoist would change
// semantics (a re-evaluated / conditionally-evaluated / non-hoist-site call) stay
// loud — never a silent wrong.
#[test]
fn r5b_while_condition_is_loud() {
    let src = "module t;\n\
        function automatic int nxt(inout int a); a=a+1; return a; endfunction\n\
        initial begin int x=0; while(nxt(x)<3) $display(\"iter\"); $finish; end endmodule";
    assert!(loud(src)); // re-evaluated per iteration → cannot be hoisted once
}

#[test]
fn r5b_short_circuit_rhs_is_loud() {
    let src = "module t;\n\
        function automatic int f(inout int a); a=a+1; return a; endfunction\n\
        logic g;\n\
        initial begin int x=0; g=0; if(g && f(x)>0) $display(\"Y\"); $finish; end endmodule";
    assert!(loud(src)); // `&&` RHS is conditional → not hoisted (no silent unconditional call)
}

#[test]
fn r5b_eval_order_read_of_mutated_is_loud() {
    // `y = x + f(x)`: IEEE evaluates the `x` operand BEFORE `f(x)`, so it must read x's
    // OLD value. Hoisting f(x) (which mutates x) to before the statement would make
    // that `x` read the NEW value = a silent eval-order wrong (12 vs 11). The hoist is
    // declined when a mutated actual is read elsewhere in the expression → loud.
    let src = "module t;\n\
        function automatic int f(inout int a); a=a+1; return a; endfunction\n\
        initial begin int x=5; int y; y = x + f(x); $display(\"y=%0d\",y); $finish; end endmodule";
    assert!(loud(src));
}

#[test]
fn r5b_disjoint_operand_supported() {
    // `y = f(x) + z`: z is NOT mutated by f, so the hoist is safe (z is read
    // in place, unaffected by f's copy-out of x) → supported.
    let src = "module t;\n\
        function automatic int f(inout int a); a=a+1; return a; endfunction\n\
        initial begin int x=5; int z=100; int y; y = f(x) + z; if(y==106 && x==6) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r5b_mutated_read_in_methodcall_arg_is_loud() {
    // Completeness of the eval-order guard: a mutated var read INSIDE a method-call
    // arg (`c.m(x) + f(x)`) must also decline the hoist — else `c.m(x)` would read
    // the post-`f` x. (Found by the soundness review; `reads_ident_outside_inout`
    // walks MethodCall/New/AssignPattern/… so this is loud, not silent.)
    let src = "module t;\n\
        class C; function int m(int z); return z*2; endfunction endclass\n\
        function automatic int f(inout int a); a=a+1; return a; endfunction\n\
        initial begin C c; int x=5; int y; c=new(); y = c.m(x) + f(x); $display(\"y=%0d\",y); $finish; end endmodule";
    assert!(loud(src));
}

// R5-A: the unpacked-struct tf-port ITSELF is now supported — the record port
// expands to one member formal per field (`$unp$r$field`), reusing the scalar
// unpacked-struct member desugar. A TASK gets full inout copy-in/out via the
// proven task-terminator path; a FUNCTION gets an input struct formal.
#[test]
fn r5a_task_inout_struct_formal_supported() {
    let src = "package p; typedef struct { string name; int count; } rec_t; endpackage\n\
        module t; import p::*;\n\
        task automatic bump(inout rec_t r); r.count=r.count+1; endtask\n\
        initial begin rec_t r; r.count=1; bump(r); if(r.count==2) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn r5a_function_input_struct_formal_supported() {
    let src = "package p; typedef struct { string name; int count; } rec_t; endpackage\n\
        module t; import p::*;\n\
        function automatic int rd(input rec_t r); return r.count+10; endfunction\n\
        initial begin rec_t r; r.count=5; if(rd(r)==15) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

// ───────────── V2A (§4.5.170): TASK `input` dynamic-array formal ─────────────
// A STATIC task with an `input` dyn-array formal aliases the caller's DynArray
// read-only via `dyn_subst` — the same R2 machinery the function path uses.
#[test]
fn v2a_task_input_dyn_array_supported() {
    // `.size()` + element reads in a static task. sum = 10+20+30 = 60, size 3.
    let src = "module t;\n\
        byte arr[];\n\
        task consume(input byte b[], output int r);\n\
          integer i; r=0; for(i=0;i<b.size();i=i+1) r=r+b[i]; r=r+b.size(); endtask\n\
        initial begin int x; arr=new[3]; arr[0]=10; arr[1]=20; arr[2]=30;\n\
          consume(arr,x); if(x==63) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_task_dyn_pass_by_value() {
    // IEEE §13.5.1: an `input` dyn-array formal is pass-by-VALUE. The body mutates the
    // caller's array (`a[0]=999`) AFTER entry; the formal `b` must read the pre-call
    // SNAPSHOT (10), NOT the mutation — the alias-vs-copy silent-wrong the adversarial
    // 2-lens caught. vita snapshots the caller's handle into a fresh DynArray temp
    // (`alloc_dyn_snapshot` + `handle_copy_stmts`) at task entry.
    let src = "module t;\n\
        int a[];\n\
        task consume(input int b[], output int r); a[0]=999; r=b[0]; endtask\n\
        initial begin int x; a=new[2]; a[0]=10; a[1]=20; consume(a,x);\n\
          if(x==10) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_task_dyn_pass_by_value_indirect() {
    // Pass-by-value must also hold when a CALLEE mutates the array: `poke()` writes
    // `a[1]` before `b[1]` is read. The snapshot isolates b → 20, not 777.
    let src = "module t;\n\
        int a[];\n\
        task poke(); a[1]=777; endtask\n\
        task consume(input int b[], output int r); poke(); r=b[1]; endtask\n\
        initial begin int x; a=new[2]; a[0]=10; a[1]=20; consume(a,x);\n\
          if(x==20) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_task_signed_int_element() {
    // A signed `int` element reads negative (no unsigned collapse). 100+(-7)=93.
    let src = "module t;\n\
        int arr[];\n\
        task consume(input int b[], output int r); r=b[0]+b[1]; endtask\n\
        initial begin int x; arr=new[2]; arr[0]=100; arr[1]=-7;\n\
          consume(arr,x); if(x==93) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_task_dyn_reforward_supported() {
    // Re-forwarding: a dyn-array formal passed on to a NESTED task (transitive
    // read-only alias via `dyn_array_actual_net`'s `dyn_subst` consult). inner sees
    // size 3, element[1]=8.
    let src = "module t;\n\
        byte arr[];\n\
        task inner(input byte c[], output int r); r=c.size()*100 + c[1]; endtask\n\
        task outer(input byte b[], output int r); inner(b, r); endtask\n\
        initial begin int x; arr=new[3]; arr[0]=7; arr[1]=8; arr[2]=9;\n\
          outer(arr,x); if(x==308) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_function_reforward_still_works() {
    // The shared `dyn_array_actual_net` change must also let a FUNCTION re-forward its
    // dyn-array formal (and not regress the non-forward function path). 3+8 = 11.
    let src = "module t;\n\
        byte arr[];\n\
        function automatic int inner(input byte c[]); return c.size()+c[1]; endfunction\n\
        function automatic int outer(input byte b[]); return inner(b); endfunction\n\
        initial begin arr=new[3]; arr[0]=7; arr[1]=8; arr[2]=9;\n\
          if(outer(arr)==11) $display(\"PASS\"); $finish; end endmodule";
    assert_eq!(run(src).0, "PASS");
}

#[test]
fn v2a_automatic_task_dyn_array_loud() {
    // An AUTOMATIC (frame) task with a dyn-array formal needs the handle-in-frame-slot
    // infra (V5) — loud until then (NOT a silent mis-lower). correct-or-loud.
    let src = "module t;\n\
        byte arr[];\n\
        task automatic consume(input byte b[]); $display(\"o=%0d\", b[0]); endtask\n\
        initial begin arr=new[1]; arr[0]=5; consume(arr); $finish; end endmodule";
    assert!(loud(src), "automatic-task dyn-array formal must stay loud");
}

#[test]
fn v2a_task_dyn_write_loud() {
    // Writing the read-only input alias (`b[0]=x`) stays loud (E3010) — never a silent
    // corruption of the caller's array. Mirrors the function R2 write asymmetry.
    let src = "module t;\n\
        byte arr[];\n\
        task consume(input byte b[]); b[0]=9; endtask\n\
        initial begin arr=new[2]; consume(arr); $finish; end endmodule";
    assert!(loud(src), "write to input dyn-array formal must stay loud");
}

#[test]
fn v2a_task_dyn_sign_mismatch_loud() {
    // `byte b[]` <- `byte unsigned arr[]`: an element-signedness mismatch would read
    // 0xFF as 255 (unsigned) vs -1 (signed) — loud rather than silent-wrong.
    let src = "module t;\n\
        byte unsigned arr[];\n\
        task consume(input byte b[]); $display(\"o=%0d\", b[0]); endtask\n\
        initial begin arr=new[1]; arr[0]=8'hFF; consume(arr); $finish; end endmodule";
    assert!(loud(src), "sign-mismatched dyn-array actual must stay loud");
}

#[test]
fn v2a_task_dyn_queue_actual_loud() {
    // A queue actual (`int q[$]`) to a dyn-array formal is a different NetKind — loud.
    let src = "module t;\n\
        int q[$];\n\
        task consume(input int b[]); $display(\"o=%0d\", b[0]); endtask\n\
        initial begin q.push_back(5); consume(q); $finish; end endmodule";
    assert!(loud(src), "queue actual to dyn-array formal must stay loud");
}
