//! PRE-EXISTING SILENT-WRONG (found by the S3a adversarial review, fixed alongside
//! it): a STRING value written to a PACKED destination inside a subroutine body was
//! not converted at all — neither the text nor the width.
//!
//! Two halves, and the second was found by the review of the fix for the first:
//!
//! 1. `frame_rhs_value`'s `$sformatf` intercept was gated on the destination's net
//!    KIND being `string`, so a packed destination fell through to the plain `eval`
//!    path — which produces no text. `reg [63:0] p; p = $sformatf("v=%0d", 7);`
//!    inside a function came back as the raw integer bytes, at exit 0, while the
//!    identical statement at module scope rendered correctly.
//! 2. Rendering it was only half. The frame funnel stored the rendered `Value`
//!    whole and relied on `resize_keep_sign`, which returns an `is_str` value
//!    UNCHANGED (its own doc: "the write funnel owns §6.16 conversion"). The module
//!    funnel resizes and then stores BITS, dropping the `Value` wrapper. So the
//!    frame slot of a `reg [63:0]` held a 24-BIT value: `~p`, `p >> 64` and a
//!    compare against a padded literal all read wrong, and for a text LONGER than
//!    the destination the slot held more bits than the net's declared width.
//! 3. Resizing was still not enough. `Value::resize` clears `is_str` on both of its
//!    real paths and NOT on its `new_width == self.width` early return — the path a
//!    rendered text takes when it is exactly as wide as its destination. There the
//!    flag survived, bare `%s` rendered the raw bytes instead of the packed
//!    NUL→space form, and `resize_keep_sign` short-circuited on the same flag so a
//!    SIGNED destination read back UNSIGNED (`integer p = s;` with
//!    `s == 32'hf0f1f2f3` gave 4042388211 in a frame against -252579085 at module
//!    scope — and iverilog answers -252579085 in both).
//!
//! ## Two oracles, and the narrow claim is the one that is true
//!
//! iverilog 13 refuses a packed `$sformatf` destination outright (*"cannot be
//! implicitly cast to the target type"*), so THAT statement has no external oracle
//! — pinned instead by the strongest internal form: the same statement at module
//! scope, observed **inside the frame** rather than after the return copy-out
//! re-packs it.
//!
//! But the PACKING RULE does have an external oracle, through the legal spelling of
//! the same conversion: `reg [15:0] p = {"ab","cde"};` is `de` / 25701 in iverilog,
//! and a 64-bit destination keeps all five characters. Both are asserted below, so
//! the rule the fix implements is anchored and not merely self-consistent.
//!
//! ## Why the observation point matters
//!
//! The first version of this file printed the value AFTER returning it to a module
//! variable — which re-packs it through the module funnel — and asserted through
//! `%0s`, the one specifier that hides a padding difference. Measured: it passed
//! with the width still wrong, and it also passed with the fix. A test that cannot
//! separate the two states is not a test of either.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// ⚠️ The `native` column is weaker than it looks and is kept for the shapes where
/// it means something. Tier-3's `eval_call` forwards to the engine's
/// `run_frame_call`, so a frame body executes the SAME code on all three backends —
/// and a design carrying a `string` NET is refused by the S0 `string` row, which
/// makes that column a second VM run. What it does check is that admitting these
/// shapes did not move the tier-3 gate.
fn run_on(src: &str, backend: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fsfd_{}_{n}", std::process::id()));
    // START CLEAN — the rule this slice added to ENGINEERING_RULES, applied to the
    // file the slice added. `n` restarts at 0 per test process and PIDs are
    // recycled, so a killed run can leave this directory behind.
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let s = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    s.lines()
        .filter(|l| l.starts_with('!'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run `stmt` against a `decl` destination twice — once at module scope, once
/// inside a `function automatic` — and require the same answer.
///
/// The report line is emitted where the value LIVES (inside the function, before
/// any return), and it carries `%h` and `~p` as well as `%0s`: those two are the
/// width-sensitive views, and they are what the earlier `%0s`-after-return shape
/// could not see.
fn both_scopes(decl: &str, stmt: &str, want: &str) {
    // FOUR columns, and the first one is here because the other three could not
    // see the last defect: bare `%s` is the only render that keys on `is_str`, so
    // a value that reached the slot still flagged as a string is invisible to
    // `%0s` (which strips leading NULs on both forms), to `%h` (same bits) and to
    // `~p` (same bits again). `%0d` is the SIGN column — `resize_keep_sign`
    // short-circuits on the same flag, so an unconverted value also reads back
    // unsigned.
    let show = "$display(\"![%s]|%0s|%h|%0d|%0d\", p, p, p, p, ~p);";
    let module =
        format!("module top;\n  {decl}\n  initial begin {stmt} {show} $finish; end\nendmodule\n");
    let frame = format!(
        "module top;\n\
           function automatic integer mk(input integer x);\n\
             {decl}\n\
             begin {stmt} {show} mk = 0; end\n\
           endfunction\n\
           integer r;\n\
           initial begin r = mk(7); $finish; end\n\
         endmodule\n"
    );
    // THREE scopes, and the third is not decoration. A `task automatic` whose body
    // leaves the `&self` frame-call subset — a single `$display` does it — is
    // lifted to `run_process`, whose frame-local write lane calls
    // `frame_write_lvalue` DIRECTLY. That lane bypassed the coercion when it was
    // installed one level up, so functions were fixed and tasks were not, and this
    // file could not see it: it contained the word `task` zero times. The `$display`
    // is what selects the lane, so it is load-bearing, not a print.
    let lifted_task = format!(
        "module top;\n\
           task automatic mk(input integer x);\n\
             {decl}\n\
             begin {stmt} {show} end\n\
           endtask\n\
           initial begin mk(7); $finish; end\n\
         endmodule\n"
    );
    for backend in ["bytecode", "interp", "native"] {
        let m = run_on(&module, backend);
        let f = run_on(&frame, backend);
        let t = run_on(&lifted_task, backend);
        assert_eq!(
            m, f,
            "{backend}: module scope and FUNCTION body must agree on TEXT and WIDTH\n\
             --- module ---\n{module}\n--- frame ---\n{frame}"
        );
        assert_eq!(
            m, t,
            "{backend}: module scope and a LIFTED TASK body must agree too\n\
             --- module ---\n{module}\n--- task ---\n{lifted_task}"
        );
        assert_eq!(
            m, want,
            "{backend}: the shared answer must also be the RIGHT one (all three \
             moving together is not enough)"
        );
    }
}

#[test]
fn sformatf_into_a_packed_destination_matches_module_scope() {
    // 3 characters into 64 bits: zero-padded on the left, so `~p` sees the pad.
    both_scopes(
        "reg [63:0] p;",
        "p = $sformatf(\"v=%0d\", 7);",
        "![     v=7]|v=7|0000000000763d37|7748919|18446744073701802696",
    );
}

#[test]
fn sformatf_longer_than_the_destination_is_left_truncated() {
    // 5 characters into 16 bits: the LEFT end is dropped, and the slot must not end
    // up wider than the net — the invariant the unfixed funnel broke. The `~p`
    // column is the arithmetic check that the width really is 16 and not 40:
    // 0xFFFF ^ 0x6465 = 39834 (a 40-bit `~` would be ~1.1e12).
    both_scopes(
        "reg [15:0] p;",
        "p = $sformatf(\"abcde\");",
        "![de]|de|6465|25701|39834",
    );
}

#[test]
fn sformatf_into_a_two_state_int_matches_module_scope() {
    // `int` is SIGNED 32-bit, so `~0x62636465` prints as -1650680934 — which is
    // also the check that the destination's SIGN survived the conversion, not just
    // its width.
    both_scopes(
        "int p;",
        "p = $sformatf(\"abcde\");",
        "![bcde]|bcde|62636465|1650680933|-1650680934",
    );
}

/// The SAME conversion with no `$sformatf` in sight — a plain `string` variable
/// copied into a packed local. This was wrong in a frame body before the fix and
/// right at module scope, independently of the intercept, which is why the fix had
/// to land on the write funnel rather than on the intercept.
#[test]
fn a_string_variable_copied_into_a_packed_local_matches_module_scope() {
    both_scopes(
        "string s; reg [15:0] p;",
        "s = \"abcde\"; p = s;",
        "![de]|de|6465|25701|39834",
    );
}

/// The packing rule, pinned against iverilog through the spelling iverilog accepts.
///
/// `{"ab","cde"}` is an ordinary concatenation of string literals — a packed
/// constant, not a string handle — so iverilog compiles it and answers. Measured
/// with iverilog 13.0: `de` / 25701 for the 16-bit destination, all five characters
/// for the 64-bit one.
#[test]
fn the_packing_rule_matches_iverilog_on_the_spelling_it_accepts() {
    let src = "module top;\n\
           reg [15:0] narrow;\n\
           reg [63:0] wide;\n\
           initial begin\n\
             narrow = {\"ab\", \"cde\"};\n\
             wide   = {\"ab\", \"cde\"};\n\
             $display(\"!%0s|%0d\", narrow, narrow);\n\
             $display(\"!%0s|%0d\", wide, wide);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    for backend in ["bytecode", "interp", "native"] {
        assert_eq!(
            run_on(src, backend),
            "!de|25701\n!abcde|418262508645",
            "{backend}: iverilog 13 answers de/25701 and abcde/418262508645"
        );
    }
}

/// `$sformatf` whose argument is the FORMAL — the render reads a frame slot, an
/// axis the module-scope twin cannot reach at all.
#[test]
fn sformatf_with_a_frame_local_argument_renders_in_a_frame_body() {
    let frame = "module top;\n\
           function automatic integer mk(input integer x);\n\
             reg [63:0] p;\n\
             begin p = $sformatf(\"x=%0d\", x); $display(\"!%0s|%h\", p, p); mk = 0; end\n\
           endfunction\n\
           integer r;\n\
           initial begin r = mk(42); $finish; end\n\
         endmodule\n";
    for backend in ["bytecode", "interp", "native"] {
        assert_eq!(
            run_on(frame, backend),
            "!x=42|00000000783d3432",
            "{backend}"
        );
    }
}

/// The `string` destination — the case the old gate DID handle — must not have
/// moved. Removing a condition is how a fix takes the working case with it, and
/// widening the write funnel is how it truncates one.
///
/// `native` is absent on purpose and not by omission: a `string` net is refused by
/// the S0 `string` eligibility row, so tier-3 never runs this design.
#[test]
fn a_string_destination_in_a_frame_body_is_unchanged() {
    let src = "module top;\n\
           function automatic int mk(input integer x);\n\
             string s;\n\
             begin s = $sformatf(\"s=%0d\", x); $display(\"!%0s|%0d\", s, s.len()); mk = 0; end\n\
           endfunction\n\
           int n;\n\
           initial begin n = mk(7); $finish; end\n\
         endmodule\n";
    for backend in ["bytecode", "interp"] {
        assert_eq!(run_on(src, backend), "!s=7|3", "{backend}");
    }
}

/// **The equal-width case, on a SIGNED destination** — the row the first two
/// versions of this file missed, and the one that mattered most.
///
/// `Value::resize` clears `is_str` on both of its real paths and NOT on its
/// `new_width == self.width` early return. A four-byte text into a 32-bit
/// destination takes exactly that path, so the flag survived — and
/// `resize_keep_sign` short-circuits on it too, which meant the SIGN was never
/// stamped either: the frame read back unsigned where module scope read signed.
///
/// Anchored against iverilog through a Verilog-2005 spelling of the same
/// assignment (a 32-bit value into a signed 32-bit destination), which answers
/// -252579085 at module scope AND inside a function body.
#[test]
fn an_equal_width_string_into_a_signed_destination_keeps_its_sign() {
    let src = "module top;\n\
           string s;\n\
           function automatic integer f(input string v);\n\
             integer p;\n\
             begin p = v; f = p; end\n\
           endfunction\n\
           integer m;\n\
           initial begin\n\
             s = \"0000\";\n\
             s.putc(0, 8'hF0); s.putc(1, 8'hF1); s.putc(2, 8'hF2); s.putc(3, 8'hF3);\n\
             m = s;\n\
             $display(\"!%0d|%h\", m, m);\n\
             $display(\"!%0d|%h\", f(s), f(s));\n\
             $finish;\n\
           end\n\
         endmodule\n";
    for backend in ["bytecode", "interp"] {
        assert_eq!(
            run_on(src, backend),
            "!-252579085|f0f1f2f3\n!-252579085|f0f1f2f3",
            "{backend}: the frame must read back SIGNED, as module scope and \
             iverilog both do"
        );
    }
}

/// A CLASS FIELD destination, wider than the 32-bit handle its lvalue names.
///
/// This is the design that separates "resize to `lvalue_width(lhs)`" from "resize
/// to the NET's width", and equally "coerce before the class dispatch" from
/// "coerce after it" — the lvalue chunk carries the FIELD's width while the net is
/// the handle. Both adversarial lenses flagged that no test in this repository
/// assigns a string to a class field, so both mutations survived the suite.
///
/// The handle is a MODULE net written from inside the frame: a `new()` in a frame
/// body is refused at elaborate, and this is the shape that actually reaches
/// `frame_or_class_write`'s class arm.
///
/// `native` is absent because the S0 `class` eligibility row refuses the design.
#[test]
fn a_string_into_a_wide_class_field_uses_the_field_width() {
    let src = "module top;\n\
           class C; reg [63:0] w; endclass\n\
           C o;\n\
           function automatic integer mk(input integer x);\n\
             string s;\n\
             begin\n\
               s = \"abcde\"; o.w = s; $display(\"!frm-str %0s %h\", o.w, o.w);\n\
               o.w = $sformatf(\"v=%0d\", x); $display(\"!frm-fmt %0s %h\", o.w, o.w);\n\
               mk = 0;\n\
             end\n\
           endfunction\n\
           integer r; string t;\n\
           initial begin\n\
             o = new(); r = mk(7);\n\
             t = \"abcde\"; o.w = t; $display(\"!mod-str %0s %h\", o.w, o.w);\n\
             o.w = $sformatf(\"v=%0d\", 7); $display(\"!mod-fmt %0s %h\", o.w, o.w);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    for backend in ["bytecode", "interp"] {
        let out = run_on(src, backend);
        let l: Vec<&str> = out.lines().collect();
        assert_eq!(l.len(), 4, "{backend}: {out}");
        // Same conversion, same answer, whichever body the assignment sits in.
        assert_eq!(
            l[0].trim_start_matches("!frm-str"),
            l[2].trim_start_matches("!mod-str"),
            "{backend}: string -> class field\n{out}"
        );
        assert_eq!(
            l[1].trim_start_matches("!frm-fmt"),
            l[3].trim_start_matches("!mod-fmt"),
            "{backend}: $sformatf -> class field\n{out}"
        );
        // …and the shared answer must be the FIELD's 64 bits, not the handle's 32
        // and not the text's own 40.
        assert_eq!(l[0], "!frm-str abcde 0000006162636465", "{backend}");
        assert_eq!(l[1], "!frm-fmt v=7 0000000000763d37", "{backend}");
    }
}

/// A string ACTUAL bound to a PACKED formal — the IN direction, which the
/// `both_scopes` shapes cannot reach because they build the value inside the body.
///
/// IEEE §13.4.3 sizes each actual to the FORMAL's type, and §6.16 says what that
/// means for a string: `bind_formal` clears `is_str` BEFORE resizing, because
/// `resize_keep_sign` short-circuits on the flag and would otherwise hand the
/// formal the whole 40-bit text, unsigned. Without that clear the formal reads
/// `abcde`/`6162636465` where the module twin `m = s` reads `de`/`6465`.
///
/// Both frame entries are driven: a `function automatic` (`run_frame_call`'s bind)
/// and a `task automatic` (`enter_task_frame`'s).
#[test]
fn a_string_actual_is_sized_to_a_packed_formal() {
    let src = "module top;\n\
           string s;\n\
           reg [15:0] m;\n\
           function automatic integer f(input reg [15:0] p);\n\
             begin $display(\"!%0s|%h|%0d\", p, p, p); f = 0; end\n\
           endfunction\n\
           task automatic t(input reg [15:0] p);\n\
             begin $display(\"!%0s|%h|%0d\", p, p, p); end\n\
           endtask\n\
           integer r;\n\
           initial begin\n\
             s = \"abcde\";\n\
             m = s; $display(\"!%0s|%h|%0d\", m, m, m);\n\
             r = f(s);\n\
             t(s);\n\
             $finish;\n\
           end\n\
         endmodule\n";
    for backend in ["bytecode", "interp"] {
        let out = run_on(src, backend);
        let l: Vec<&str> = out.lines().collect();
        assert_eq!(l.len(), 3, "{backend}: {out}");
        assert_eq!(l[0], l[1], "{backend}: module vs function formal\n{out}");
        assert_eq!(l[0], l[2], "{backend}: module vs task formal\n{out}");
        // …and iverilog answers `de`/25701 for the same conversion spelled the way
        // it accepts (`reg [15:0] p = {"ab","cde"}`), which the sibling test pins.
        assert_eq!(l[0], "!de|6465|25701", "{backend}");
    }
}

/// The MODULE lane's own §6.16 conversion, on a signed class field at exactly the
/// text's width — the shape that proved `Value::resize`'s equal-width early return
/// had to drop `is_str` at the primitive rather than at the frame funnel.
///
/// `class_field_write` stores a whole `Value` (it does not go through
/// `store_words`), and its `resize_keep_sign` short-circuits on the flag, so a
/// module-scope `o.si = s` read back UNSIGNED — 4042388211 for -252579085 — while
/// the frame path, once fixed, read it signed. One statement, two answers,
/// decided by which body it sat in: the class the whole slice exists to remove.
#[test]
fn a_string_into_a_signed_class_field_keeps_its_sign_in_every_scope() {
    let src = "module top;\n\
           class C; integer si; endclass\n\
           C o;\n\
           string s;\n\
           function automatic integer f(input integer x);\n\
             begin o.si = s; $display(\"!%0d\", o.si); f = 0; end\n\
           endfunction\n\
           task automatic t();\n\
             begin o.si = s; $display(\"!%0d\", o.si); end\n\
           endtask\n\
           integer r;\n\
           initial begin\n\
             o = new();\n\
             s = \"0000\";\n\
             s.putc(0, 8'hF0); s.putc(1, 8'hF1); s.putc(2, 8'hF2); s.putc(3, 8'hF3);\n\
             o.si = s; $display(\"!%0d\", o.si);\n\
             r = f(0);\n\
             t();\n\
             $finish;\n\
           end\n\
         endmodule\n";
    for backend in ["bytecode", "interp"] {
        assert_eq!(
            run_on(src, backend),
            "!-252579085\n!-252579085\n!-252579085",
            "{backend}: module, function and lifted-task lanes must all read signed"
        );
    }
}
