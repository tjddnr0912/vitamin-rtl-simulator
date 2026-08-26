//! R6 — a `string` OUTPUT/INOUT formal in a STATIC (inlined) task.
//!
//! PRE: every such formal was refused by one whole-port gate in
//! `elaborate/src/inline_task.rs`, whose stated reason was *"its copy-out target
//! must be a simple net"*. That reason was refuted by measurement: the copy-out
//! machinery is entirely present in the Output|Inout arm (`out_lval`, the inout
//! copy-IN, the `out_subst` binding, the exit `BlockingAssign`), and the
//! formal-local has been a real heap-backed `NetKind::String` slot since the INPUT
//! narrowing. What actually blocked it was the arm's `array_len != 1` rejection: a
//! scalar `string` net is recorded with `array_len: 0` (netdecl records no packed
//! extent for a string), so an ordinary `string a;` actual read as "a whole
//! unpacked array" to a check aimed at array actuals.
//!
//! The gate was NARROWED, not deleted, and the decision moved to where the ACTUAL
//! is known — the formal alone cannot decide it. Three reasons survive, each with
//! its own message:
//!   * a domain mismatch (`string` formal ← packed actual, or the reverse) — the
//!     two oracles SPLIT there, so it stays loud;
//!   * a select actual (`t(a[3:0])`, `t(a[1])`) — a `string` has no
//!     bit-addressable storage, and iverilog rejects the same code;
//!   * a non-lvalue actual — unchanged.
//!
//! Oracles: iverilog 13.0 and verilator 5.050 agree on every accepted cell below;
//! the values asserted here are theirs.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r6strout_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

// ───────────────────────── accepted: the report's own repro ─────────────────────

#[test]
fn static_task_string_output_and_inout() {
    // The bug report verbatim. iverilog + verilator: "in hi" / "made" / "made+io".
    let src = "module tb;\n\
         task t_in (input  string s); $display(\"in %s\", s); endtask\n\
         task t_out(output string s); s = \"made\";            endtask\n\
         task t_io (inout  string s); s = {s, \"+io\"};        endtask\n\
         string a = \"x\";\n\
         initial begin t_in(\"hi\"); t_out(a); $display(\"%s\", a); t_io(a); $display(\"%s\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("in hi"), "input formal, got:\n{out}");
    assert!(out.contains("\nmade\n"), "output copy-out, got:\n{out}");
    assert!(out.contains("made+io"), "inout copy-in+out, got:\n{out}");
}

#[test]
fn static_task_string_mixed_directions() {
    // input + inout + output + a non-string output in ONE call: the copy-in of the
    // inout must see the caller's value, and each copy-out lands on its own actual.
    // Both oracles: A=|pre-mid| P=|pre-mid!| N=7.
    let src = "module tb;\n\
         task t_mix(input string pre, inout string s, output string post, output int n);\n\
           s = {pre, s}; post = {s, \"!\"}; n = s.len();\n\
         endtask\n\
         string a = \"mid\"; string p; int n;\n\
         initial begin t_mix(\"pre-\", a, p, n); $display(\"A=|%s| P=|%s| N=%0d\", a, p, n); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=|pre-mid| P=|pre-mid!| N=7"),
        "mixed directions, got:\n{out}"
    );
}

#[test]
fn static_task_string_out_no_intermediate_leak() {
    // §13.5.3: the formal is a per-call-site LOCAL and there is a SINGLE copy-out at
    // exit, so passing the SAME net as both an input and an output must not let the
    // output's intermediate writes reach the input snapshot. Both oracles:
    // |clobber/orig| — the input still reads "orig" after `o` was overwritten.
    let src = "module tb;\n\
         task t_both(input string i, output string o); o = \"clobber\"; o = {o, \"/\", i}; endtask\n\
         string a = \"orig\";\n\
         initial begin t_both(a, a); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=|clobber/orig|"),
        "in/out aliasing, got:\n{out}"
    );
}

#[test]
fn static_task_string_out_nested_through_out_subst() {
    // The outer task's own string formal is the inner task's actual — it resolves
    // through `out_subst`, not through a module net, so this exercises the branch
    // that made the old gate's "must be a simple net" claim look plausible.
    // Both oracles: |inner+outer|.
    let src = "module tb;\n\
         task t_inner(output string s); s = \"inner\"; endtask\n\
         task t_outer(output string s); t_inner(s); s = {s, \"+outer\"}; endtask\n\
         string a = \"x\";\n\
         initial begin t_outer(a); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=|inner+outer|"),
        "nested static, got:\n{out}"
    );
}

#[test]
fn static_task_string_inout_accumulates_across_calls() {
    // §13.4.1: a static task's formal is ONE static instance, so three `inout` calls
    // accumulate. Both oracles: |a...|.
    let src = "module tb;\n\
         task t_app(inout string s); s = {s, \".\"}; endtask\n\
         string a = \"a\";\n\
         initial begin t_app(a); t_app(a); t_app(a); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=|a...|"), "inout across calls, got:\n{out}");
}

// ───────────────── accepted: `ref` / `const ref` on the static path ─────────────

#[test]
fn static_task_string_ref_formal() {
    // `ref` parses to `PortDir::Inout`, so it rode the same gate. verilator: |x!|
    // (iverilog has no `ref` support at all — "sorry: Reference ports").
    let src = "module tb;\n\
         task t_ref(ref string s); s = {s, \"!\"}; endtask\n\
         string a = \"x\";\n\
         initial begin t_ref(a); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=|x!|"), "static ref string, got:\n{out}");
}

#[test]
fn ref_spelling_is_echoed_in_the_diagnostic() {
    // R6: `ref`/`const ref` both desugar to `PortDir::Inout`, so a rejection used to
    // print "inout formal" for source containing no such keyword. `TfDirSpelling`
    // now carries the user's own word into the message. The rejection itself is the
    // select case (a `string` has no bit-addressable storage).
    let src = "module tb;\n\
         task t_ref(ref string s); s = \"z\"; endtask\n\
         string a = \"abcd\";\n\
         initial begin t_ref(a[1]); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("`string` ref formal `s`"),
        "must say `ref`, not `inout`, got:\n{out}"
    );
    assert!(!out.contains("inout formal"), "leaked `inout`:\n{out}");

    let src2 = "module tb;\n\
         task t_cr(const ref string s); endtask\n\
         string a = \"abcd\";\n\
         initial begin t_cr(a[1]); end\n\
         endmodule\n";
    let (out2, code2) = run(src2);
    assert_ne!(code2, Some(0), "must stay loud, got:\n{out2}");
    assert!(
        out2.contains("`string` const ref formal `s`"),
        "must say `const ref`, got:\n{out2}"
    );
}

// ───────────────── accepted: string METHODS on the formal itself ────────────────

#[test]
fn string_method_statement_on_static_out_formal() {
    // `string_handle` resolves by SOURCE name, but an inline task's out formal is
    // bound through `out_subst` to a MANGLED local (`__taskarg_…`), so `s.itoa(42);`
    // used to miss it and fall to the misleading "unsupported hierarchical task call
    // `s.itoa`". Loud, but for the wrong reason. Both oracles: |42| len=2.
    let src = "module tb;\n\
         task t_itoa(output string s); s.itoa(42); endtask\n\
         string a = \"x\";\n\
         initial begin t_itoa(a); $display(\"A=|%s| len=%0d\", a, a.len()); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=|42| len=2"), "itoa on formal, got:\n{out}");
}

#[test]
fn string_method_reads_on_static_inout_formal() {
    // `.len()` and `.substr()` on the formal — the READ half already routed through
    // `out_subst` (`expr_is_string_ast`), which is why these worked in a body whose
    // `.itoa()` did not. Both oracles: |he-hello| n=5.
    let src = "module tb;\n\
         task t_len(inout string s, output int n); n = s.len(); s = {s.substr(0,1), \"-\", s}; endtask\n\
         string a = \"hello\"; int n;\n\
         initial begin t_len(a, n); $display(\"A=|%s| n=%0d\", a, n); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=|he-hello| n=5"),
        "len/substr on formal, got:\n{out}"
    );
}

#[test]
fn string_element_read_on_static_inout_formal() {
    // §6.16.2 byte select on the formal. Both oracles: c=98 ('b'), A=|abc!|.
    let src = "module tb;\n\
         task t_g(inout string s, output int c); c = s[1]; s = {s, \"!\"}; endtask\n\
         string a = \"abc\"; int c;\n\
         initial begin t_g(a, c); $display(\"A=|%s| c=%0d\", a, c); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=|abc!| c=98"), "s[1] on formal, got:\n{out}");
}

// ─────────────────────────── still loud, narrowed reasons ──────────────────────

#[test]
fn string_formal_with_packed_actual_stays_loud() {
    // ORACLE SPLIT, measured 2026-08-26: iverilog copies the string's 32-bit code
    // out (W=1835099237, and `%s` still prints "made"), verilator drops the write
    // entirely (W=5). No defensible answer ⇒ loud, and the message says why.
    let src = "module tb;\n\
         task t_out(output string s); s = \"made\"; endtask\n\
         logic [31:0] w = 32'd5;\n\
         initial begin t_out(w); $display(\"W=%0d\", w); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("is bound to a packed net (`w`)"),
        "must name the domain mismatch, got:\n{out}"
    );
    // The old blanket sentence must be gone.
    assert!(
        !out.contains("its copy-out target must be a simple net"),
        "stale gate message survived:\n{out}"
    );
}

#[test]
fn packed_formal_with_string_actual_stays_loud_without_cascade() {
    // The mirror direction. It was already loud (via `array_len != 1`), but the
    // formal never got bound, so every mention of it in the body raised a SECOND,
    // misleading E3010 "undeclared net". One error now.
    let src = "module tb;\n\
         task t_out(output logic [31:0] o); o = 32'd7; endtask\n\
         string a = \"zz\";\n\
         initial begin t_out(a); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("is bound to a `string` net (`a`)"),
        "must name the domain mismatch, got:\n{out}"
    );
    assert!(
        !out.contains("E3010"),
        "spurious undeclared-name cascade:\n{out}"
    );
}

#[test]
fn string_formal_with_part_select_actual_stays_loud() {
    // iverilog: "Cannot part select assign to a string ('a')"; verilator refuses to
    // compile. A `string` has no bit-addressable storage — nothing to write into.
    let src = "module tb;\n\
         task t_out(output string s); s = \"made\"; endtask\n\
         string a = \"abcdef\";\n\
         initial begin t_out(a[3:0]); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("no bit-addressable storage"),
        "must give the storage reason, got:\n{out}"
    );
}

#[test]
fn string_formal_with_bit_select_actual_stays_loud() {
    // iverilog compiles this one but TRAPS at run time ("size mismatch when casting
    // string to vector"); verilator refuses. Same storage reason.
    let src = "module tb;\n\
         task t_out(output string s); s = \"made\"; endtask\n\
         string a = \"abcd\";\n\
         initial begin t_out(a[1]); $display(\"A=|%s|\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("no bit-addressable storage"),
        "must give the storage reason, got:\n{out}"
    );
}

#[test]
fn string_formal_with_string_array_element_actual_stays_loud() {
    // RESIDUE, recorded with its measurement: a fixed string ARRAY element with a
    // CONST index (`t_out(names[0])`) IS a whole string net — `lower_lvalue` already
    // yields the element net — and both oracles run it (A0=|made|). It stays loud
    // because this arm's inout copy-IN reads the actual as a packed value of the
    // formal's width (0 for a string); accepting it needs a handle-read path of its
    // own. The message says which of the two select reasons applies.
    let src = "module tb;\n\
         task t_out(output string s); s = \"made\"; endtask\n\
         string a[2];\n\
         initial begin t_out(a[0]); $display(\"A0=|%s|\", a[0]); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("only a bare `string` variable can receive the whole handle"),
        "must give the whole-handle reason, got:\n{out}"
    );
}

#[test]
fn string_formal_with_literal_actual_stays_loud() {
    // A non-lvalue actual for an output formal — unchanged path. iverilog also
    // rejects ("I give up on task port 1 expression").
    let src = "module tb;\n\
         task t_out(output string s); s = \"made\"; endtask\n\
         initial begin t_out(\"lit\"); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("must be a simple net or select"),
        "unchanged reason expected, got:\n{out}"
    );
}

// ───────────────────────────── neighbours (censused) ───────────────────────────

#[test]
fn automatic_task_string_out_still_works() {
    // The old message advised writing `automatic`; that advice was true and the
    // frame path is untouched here. Both oracles: A=made / B=made+io.
    let src = "module tb;\n\
         task automatic t_out(output string s); s = \"made\"; endtask\n\
         task automatic t_io (inout  string s); s = {s, \"+io\"}; endtask\n\
         string a = \"x\";\n\
         initial begin t_out(a); $display(\"A=%s\", a); t_io(a); $display(\"B=%s\", a); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=made"), "automatic output, got:\n{out}");
    assert!(out.contains("B=made+io"), "automatic inout, got:\n{out}");
}

#[test]
fn static_function_string_out_still_works() {
    // The FUNCTION twin never rode this gate and already worked. verilator agrees
    // (A=|fmade| r=7); iverilog rejects any non-input function port outright, which
    // is its own pre-2009 restriction, not a disagreement about the value.
    let src = "module tb;\n\
         function int f_out(output string s); s = \"fmade\"; return 7; endfunction\n\
         string a = \"x\"; int r;\n\
         initial begin r = f_out(a); $display(\"A=|%s| r=%0d\", a, r); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=|fmade| r=7"),
        "static function, got:\n{out}"
    );
}

#[test]
fn string_array_output_formal_stays_loud() {
    // An unpacked-ARRAY formal is a different gate ("an OUTPUT/INOUT array formal is
    // pass-by-reference"), untouched by R6 and left loud. iverilog rejects it too
    // ("Subroutine ports with unpacked dimensions are not yet supported").
    let src = "module tb;\n\
         task t_arr(output string s[2]); s[0]=\"p\"; s[1]=\"q\"; endtask\n\
         string a[2];\n\
         initial begin t_arr(a); $display(\"A=%s %s\", a[0], a[1]); end\n\
         endmodule\n";
    let (out, code) = run(src);
    assert_ne!(code, Some(0), "must stay loud, got:\n{out}");
    assert!(
        out.contains("unpacked-array formal"),
        "array-formal gate expected, got:\n{out}"
    );
}
