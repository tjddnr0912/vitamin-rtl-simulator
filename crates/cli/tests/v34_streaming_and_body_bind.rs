//! V34-1 / V34-2: **two unsupported surfaces whose rejections read like typos.**
//!
//! A streaming concatenation (`{<<8{a}}`, IEEE 1800-2017 §11.4.14) produced
//! `expected expression, found '<<'` and nothing else — no occurrence of the words
//! "streaming", "pack" or the clause number anywhere in the binary — so a reader
//! could not tell an unimplemented operator from a mistyped one. Written as an
//! assignment TARGET (`{>>{b0,b1,b2,b3}} = a;`) it printed THREE errors at one
//! `line:col`, each about a different missing token.
//!
//! A `bind` directive inside a module body (§23.11) said `expected '(' before port
//! connections, found identifier 'chk'` — the word "bind" appeared nowhere — even
//! though vita HAS binds: `elaborate/driver.rs` keys the bind table by target MODULE
//! name and wires the checker in each target instance's own scope, never consulting
//! the directive's enclosing scope, so a body bind with a module target means exactly
//! what the already-supported unit-scope spelling means. It is now parsed and hoisted.
//!
//! Oracles. The bind expectations are pinned to **verilator 5.050**, which is the only
//! oracle here: iverilog 13.0 rejects `bind` in EVERY position, unit scope included
//! (`b2.sv:8: syntax error / I give up.`). The concatenation values in the
//! no-false-positive test are pinned to **iverilog 13.0** (`r=241a q=12341234
//! c=2300`), which agrees with vita.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_top(src: &str, top: Option<&str>) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_v34_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg("t.sv");
    if let Some(t) = top {
        c.arg("--top").arg(t);
    }
    let out = c.current_dir(&d).output().expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_top(src, None)
}

fn errors(out: &str) -> usize {
    out.matches("error[VITA-").count()
}

// ───────────────────────── (A) streaming operators ─────────────────────────

/// The rhs form is refused by NAME: the operator, what it does, and the clause.
#[test]
fn a_streaming_rhs_names_the_operator_and_the_clause() {
    // ⚠️ This test asserted the WORDING of a refusal until the operator itself
    // shipped in the same slice. A wording assertion outlives the limitation it
    // describes, so it is now a VALUE assertion — the strictly stronger statement.
    // Values are LIVE verilator 5.050's; iverilog 13 does not parse the construct.
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [31:0] a = 32'hAABBCCDD;\n  \
         logic [31:0] r1, r2, r3, r4;\n  initial begin\n    \
         r1 = {<<8{a}}; r2 = {<<{a}}; r3 = {>>{a}}; r4 = {<<16{a}};\n    \
         $display(\"R=%08h %08h %08h %08h\", r1, r2, r3, r4); end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R=ddccbbaa bb33dd55 aabbccdd ccddaabb"),
        "byte reversal / bit reversal / identity / 16-bit slices:\n{out}"
    );
}

/// `{<<{a}}` (no slice size) is the same operator and gets the same message — the
/// census found the old text identical for every spelling, which was the problem.
#[test]
fn a_streaming_rhs_without_a_slice_size_is_named_too() {
    // Same conversion as above: a no-slice-size `{<<{a}}` reverses BITS. `8'hA5` is
    // `1010_0101`, whose bit reversal is `1010_0101` — a palindrome, which would make
    // this cell certify itself, so it uses `8'h1A` (`0001_1010` → `0101_1000` = 58).
    // verilator 5.050 prints both.
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [7:0] a = 8'h1A, r;\n  \
         logic [15:0] b = 16'h1234, s;\n  \
         initial begin r = {<<{a}}; s = {<<4{b}}; \
         $display(\"r=%02h s=%04h\", r, s); end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=58 s=4321"), "{out}");
}

/// ⭐ The lvalue (unpack) spelling used to print THREE errors at one column
/// ("expected identifier" / "expected '}'" / "expected '=' or '<=' after lvalue").
/// The balanced skip leaves the cursor where a well-formed concatenation would have
/// left it, so the enclosing `= a;` still parses and the cascade is gone.
#[test]
fn a_streaming_lvalue_is_one_named_error_not_three() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [31:0] a = 32'hAABBCCDD;\n  \
         logic [7:0] b0, b1, b2, b3;\n  \
         initial begin {>>{b0,b1,b2,b3}} = a; $display(\"%h\", b0); end\nendmodule\n",
    );
    assert_ne!(code, Some(0));
    assert_eq!(errors(&out), 1, "was 3 at one line:col:\n{out}");
    assert!(out.contains("streaming operator"), "{out}");
}

/// The predicate is EXACT, not a heuristic: `<<`/`>>` cannot begin any legal
/// expression, so no supported concatenation can trip it. Values pinned to iverilog
/// 13.0 (`r=241a q=12341234 c=2300`) — a false positive here would refuse a design
/// both oracles run.
#[test]
fn concatenations_containing_shifts_still_run_and_give_the_oracle_values() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [7:0] a = 8'h12, b = 8'h34;\n  \
         logic [15:0] r;\n  logic [31:0] q;\n  initial begin\n    \
         r = {a << 1, b >> 1};\n    q = {2{a, b}};\n    \
         $display(\"r=%h q=%h\", r, q);\n    \
         $display(\"c=%h\", {a[3:0], b[7:4], 8'h00});\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=241a q=12341234"), "{out}");
    assert!(out.contains("c=2300"), "{out}");
}

/// `<<<` / `>>>` are `ShlA`/`ShrA`, separate tokens and NOT streaming operators
/// (§11.4.14 lists `<<` and `>>` only). They keep the generic error, so the named
/// one never claims a construct the source did not write.
#[test]
fn an_arithmetic_shift_token_after_a_brace_is_not_called_streaming() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [7:0] a = 8'hA5, r;\n  \
         initial begin r = {<<<8{a}}; $display(\"r=%h\", r); end\nendmodule\n",
    );
    assert_ne!(code, Some(0));
    assert!(
        !out.contains("streaming operator"),
        "the exact-token restriction must hold:\n{out}"
    );
    assert!(out.contains("expected expression"), "{out}");
}

// ───────────────────────── (B) `bind` in a module body ─────────────────────────

/// The reporter's own spelling: a bind inside the body of the module it targets.
/// verilator 5.050 prints `chk 0` / `chk 1` and finishes at 2; so does vita now.
#[test]
fn a_body_bind_on_the_enclosing_module_runs_like_verilator() {
    let (out, code) = run_top(
        "`timescale 1ns/1ns\nmodule chk(input logic c);\n  \
         always @(c) $display(\"chk %b\", c);\nendmodule\n\
         module t;\n  logic c_i = 0;\n  bind t chk c_i_inst(.c(c_i));\n  \
         initial begin #1 c_i = 1; #1 $finish; end\nendmodule\n",
        Some("t"),
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("chk 0"), "{out}");
    assert!(out.contains("chk 1"), "{out}");
    assert!(out.contains("at time 2"), "{out}");
}

/// ⭐ The load-bearing case: the target module has TWO instances, so the bind must
/// fire once per instance — which is what makes "keyed by target module name" the
/// right claim. verilator: `chk v=3` ×2 then `chk v=9` ×2.
#[test]
fn a_body_bind_attaches_to_every_instance_of_its_target() {
    let (out, code) = run_top(
        "`timescale 1ns/1ns\nmodule chk(input logic [3:0] v);\n  \
         always @(v) $display(\"chk v=%0d\", v);\nendmodule\n\
         module sub;\n  logic [3:0] v = 4'd3;\n  initial begin #1 v = 4'd9; end\n\
         endmodule\nmodule t;\n  sub u1();\n  sub u2();\n  \
         bind sub chk c(.v(v));\n  initial begin #2 $finish; end\nendmodule\n",
        Some("t"),
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        out.matches("chk v=3").count(),
        2,
        "once per instance:\n{out}"
    );
    assert_eq!(out.matches("chk v=9").count(), 2, "{out}");
}

/// A body bind and the already-supported unit-scope bind are the SAME directive:
/// elaborate never looks at where it was written. Same design, bind moved out.
#[test]
fn a_body_bind_and_a_unit_scope_bind_agree() {
    let body = "`timescale 1ns/1ns\nmodule chk(input logic [3:0] v);\n  \
         always @(v) $display(\"chk v=%0d\", v);\nendmodule\n\
         module sub;\n  logic [3:0] v = 4'd3;\n  initial begin #1 v = 4'd9; end\n\
         endmodule\nmodule t;\n  sub u1();\n  BINDLINE  initial begin #2 $finish; end\n\
         endmodule\nTAIL\n";
    let (inner, ci) = run_top(
        &body
            .replace("BINDLINE", "bind sub chk c(.v(v));\n")
            .replace("TAIL", ""),
        Some("t"),
    );
    let (outer, co) = run_top(
        &body
            .replace("BINDLINE", "")
            .replace("TAIL", "bind sub chk c(.v(v));"),
        Some("t"),
    );
    assert_eq!(ci, Some(0), "{inner}");
    assert_eq!(co, Some(0), "{outer}");
    assert_eq!(inner, outer, "the position must not change the meaning");
    assert!(inner.contains("chk v=9"), "{inner}");
}

/// An INSTANCE target (`bind t.u chk c(…)`) is a different key than vita's bind table
/// has, and used to report `expected identifier, found '.'` plus two follow-ons.
/// One error now, naming the construct and the working alternative.
#[test]
fn a_bind_instance_target_is_one_named_error() {
    let (out, code) = run_top(
        "`timescale 1ns/1ns\nmodule chk(input logic c);\n  \
         always @(c) $display(\"chk %b\", c);\nendmodule\n\
         module sub;\n  logic c_i = 0;\n  initial begin #1 c_i = 1; #1 $finish; end\n\
         endmodule\nmodule t;\n  sub u();\n  bind t.u chk c_i_inst(.c(c_i));\nendmodule\n",
        Some("t"),
    );
    assert_ne!(code, Some(0));
    assert_eq!(errors(&out), 1, "was 3:\n{out}");
    assert!(out.contains("bind_target_instance_list"), "{out}");
    assert!(out.contains("target MODULE name"), "{out}");
}

/// `bind` is a CONTEXTUAL keyword in vita's lexer, so at module-item position — where
/// a bare identifier legitimately begins an instantiation — the new arm must not
/// swallow an instantiation of a module named `bind`. It separates at the third
/// token: a directive is `bind TARGET CHECKER …`, an instantiation is `bind u (…)`.
#[test]
fn a_module_named_bind_still_instantiates() {
    let (out, code) = run_top(
        "`timescale 1ns/1ns\nmodule bind_like(output logic o);\n  assign o = 1'b1;\n\
         endmodule\nmodule bind (output logic o);\n  assign o = 1'b0;\nendmodule\n\
         module t;\n  logic x, y;\n  bind u1 (.o(x));\n  bind_like u2 (.o(y));\n  \
         initial begin #1 $display(\"x=%b y=%b\", x, y); $finish; end\nendmodule\n",
        Some("t"),
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("x=0 y=1"), "{out}");
}
