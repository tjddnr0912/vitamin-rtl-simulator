//! `%p` on an AGGREGATE — IEEE 1800 §21.2.1.7 (external report V34-5).
//!
//! `$display("%p", arr)` used to be a hard error, in two spellings that both
//! answer a question about the whole VALUE:
//!
//! ```text
//!   a whole unpacked array has no value in this context
//!   a dynamic-storage handle has no whole-value surface
//! ```
//!
//! Both are right about the value. `%p` asks for a RENDERING, and it is defined
//! for exactly the aggregates those two messages refuse, so in a `%p` argument
//! position the aggregate is the operand rather than a value.
//!
//! ## The oracle, measured
//!
//! **iverilog 13 does not implement `%p`.** Measured on this machine, not
//! assumed: `$display("%p", 42)` warns `unknown format $display<%p>`, prints the
//! four characters `<%p>` and then the argument in the default radix, and an
//! aggregate argument is refused outright (`$display does not support argument
//! type (vpiMemory)` for an unpacked array, `(116)` for a queue). So the
//! project's primary oracle has nothing to say about this conversion, and
//! verilator 5.050 — which implements it — is the only tool that does. Every
//! expected string below is verilator's, measured. The three cells where vita
//! deliberately differs are pinned in their own tests with the reason.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fmtpagg_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // stdout AND stderr: the printed lines land on stdout, the diagnostics (and
    // `$info`/`$error` reports) on stderr, and half of what this file pins is a
    // diagnostic. Splitting them here once cost four tests that asserted against
    // an empty string and looked like product failures.
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code(),
    )
}

/// The report's own program, one line at a time. Every expectation is the
/// verilator 5.050 output for the same source.
#[test]
fn p_aggregate_matches_verilator_on_the_reported_shapes() {
    let (out, c) = run("module tb;\n\
         \x20 int a[3]; int q[$]; int m[string]; int n[int]; int d[];\n\
         \x20 typedef struct packed { logic [3:0] x; logic [3:0] y; } s_t; s_t s;\n\
         \x20 initial begin\n\
         \x20   a = '{1,2,3}; q.push_back(7); q.push_back(8); m[\"k\"]=9; n[3]=4; n[1]=2;\n\
         \x20   d = new[2]; d[0]=5; d[1]=6; s = 8'hA5;\n\
         \x20   $display(\"A=%p\", a);\n\
         \x20   $display(\"Q=%p\", q);\n\
         \x20   $display(\"M=%p\", m);\n\
         \x20   $display(\"N=%p\", n);\n\
         \x20   $display(\"D=%p\", d);\n\
         \x20   $display(\"S=%p\", s);\n\
         \x20   $display(\"I=%p\", 42);\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    for want in [
        "A='{'h1, 'h2, 'h3}",    // fixed-size unpacked array
        "Q='{'h7, 'h8}",         // queue, push order
        "M='{\"k\":'h9}",        // string-keyed assoc: quoted key, no space after `:`
        "N='{'h1:'h2, 'h3:'h4}", // integer-keyed assoc, ascending key order
        "D='{'h5, 'h6}",         // dynamic array
        "S=165",                 // a packed struct IS a bit vector, so `%p` is its value
        "I=42",                  // `%p` of a plain integral is legal and prints the value
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// The `0` flag. verilator: it changes NOTHING for an aggregate (the elements are
/// always in the `'h` form) and, for a non-aggregate, selects `'h<hex>` over the
/// decimal form. That equivalence is why the renderer has ONE leaf function:
/// "render an element" and "render a scalar under `%0p`" are the same question.
#[test]
fn p_zero_flag_is_the_element_form_and_aggregates_ignore_it() {
    let (out, c) = run("module tb;\n\
         \x20 int a[2]; int i; logic [7:0] v; logic [3:0] w; real r; string s;\n\
         \x20 initial begin\n\
         \x20   a = '{1,2}; i = -5; v = 8'hA5; w = 4'ha; r = 2.5; s = \"x\";\n\
         \x20   $display(\"A=%p|%0p\", a, a);\n\
         \x20   $display(\"I=%p|%0p\", i, i);\n\
         \x20   $display(\"V=%p|%0p\", v, v);\n\
         \x20   $display(\"W=%p|%0p\", w, w);\n\
         \x20   $display(\"R=%p|%0p\", r, r);\n\
         \x20   $display(\"S=%p|%0p\", s, s);\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    // verilator 5.050, measured:
    for want in [
        "A='{'h1, 'h2}|'{'h1, 'h2}", // the flag is inert on an aggregate
        "I=-5|'hfffffffb",           // signed decimal vs the width's two's complement
        "V=165|'ha5",
        "W=10|'ha",
        "R=2.5|2.5", // a real ignores the flag too
        "S=\"x\"|\"x\"",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// Nesting. A multi-dimensional unpacked array renders one pattern per dimension,
/// row-major — verilator-matched, and the dimension sizes come from the same
/// `net_dims` sidecar `flatten_word` derives its strides from, so a `%p` render
/// and an element read cannot disagree about which word is which.
#[test]
fn p_multidim_array_nests_one_pattern_per_dimension() {
    let (out, c) = run("module tb;\n\
         \x20 int a2[2][3];\n\
         \x20 initial begin\n\
         \x20   a2[0][0]=1; a2[0][1]=2; a2[0][2]=3;\n\
         \x20   a2[1][0]=4; a2[1][1]=5; a2[1][2]=6;\n\
         \x20   $display(\"X=%p\", a2);\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("X='{'{'h1, 'h2, 'h3}, '{'h4, 'h5, 'h6}}"),
        "verilator-matched nesting; got:\n{out}"
    );
}

/// An aggregate with no elements is `'{}`, not `x` and not a diagnostic — a
/// never-`new`ed handle IS the empty aggregate (the same lazy-object contract
/// every other dyn read uses). A non-0-based dimension keeps its element ORDER
/// (verilator prints ascending index, which is also vita's flat word order).
#[test]
fn p_empty_aggregates_and_non_zero_based_dims() {
    let (out, c) = run("module tb;\n\
         \x20 int eq[$]; int ed[]; int em[int]; int u13[1:3]; int d30[3:0];\n\
         \x20 initial begin\n\
         \x20   u13[1]=1; u13[2]=2; u13[3]=3;\n\
         \x20   d30[3]=33; d30[2]=22; d30[1]=11; d30[0]=0;\n\
         \x20   $display(\"E=%p|%p|%p\", eq, ed, em);\n\
         \x20   $display(\"U=%p\", u13);\n\
         \x20   $display(\"D=%p\", d30);\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("E='{}|'{}|'{}"),
        "empty aggregates; got:\n{out}"
    );
    assert!(out.contains("U='{'h1, 'h2, 'h3}"), "[1:3]; got:\n{out}");
    // 0, 11, 22, 33 — ascending INDEX, which is verilator's order too.
    assert!(
        out.contains("D='{'h0, 'hb, 'h16, 'h21}"),
        "[3:0]; got:\n{out}"
    );
}

/// The argument→conversion mapping has to be exact, because it is what decides
/// "this aggregate is an operand here". These are the shapes that break a sloppy
/// mapping: a `%%` that is NOT a conversion, a preceding `%s`, two aggregates in
/// one call, and the file family whose `args[0]` is a descriptor and is never
/// rendered.
#[test]
fn p_argument_position_survives_flags_escapes_and_the_file_family() {
    let (out, c) = run("module tb;\n\
         \x20 int a[3]; int q[$]; integer fd;\n\
         \x20 initial begin\n\
         \x20   a = '{1,2,3}; q.push_back(9);\n\
         \x20   $display(\"T=%p%p\", a, q);\n\
         \x20   $display(\"M=%s|%p|%0d\", \"zz\", a, 5);\n\
         \x20   $display(\"P=%%p|%p\", a);\n\
         \x20   fd = $fopen(\"o.txt\", \"w\");\n\
         \x20   $fdisplay(fd, \"F=%p\", a);\n\
         \x20   $fclose(fd);\n\
         \x20   $display(\"G=%s\", $sformatf(\"%p\", a));\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    for want in [
        "T='{'h1, 'h2, 'h3}'{'h9}",
        "M=zz|'{'h1, 'h2, 'h3}|5",
        "P=%p|'{'h1, 'h2, 'h3}",
        "G='{'h1, 'h2, 'h3}",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// `%p` is NOT a licence for a whole aggregate anywhere else: every other
/// conversion, and an unformatted argument, must stay LOUD. The gate is
/// per-argument-position, so this is the test that says so.
#[test]
fn a_whole_aggregate_under_any_other_conversion_stays_loud() {
    for fmt in ["%d", "%h", "%s", "%b", "%o"] {
        let (out, c) = run(&format!(
            "module tb;\n  int a[3];\n  initial begin a='{{1,2,3}};\n\
             \x20   $display(\"{fmt}\", a); end\nendmodule\n"
        ));
        assert_ne!(
            c,
            Some(0),
            "`{fmt}` of a whole array must stay loud:\n{out}"
        );
        assert!(
            out.contains("a whole unpacked array has no value in this context"),
            "`{fmt}`: the value-surface message is still the right one; got:\n{out}"
        );
    }
    // …and the dynamic-storage twin.
    let (out, c) = run("module tb;\n  int q[$];\n  initial begin q.push_back(1);\n\
         \x20   $display(\"%d\", q); end\nendmodule\n");
    assert_ne!(c, Some(0), "`%d` of a queue must stay loud:\n{out}");
    assert!(
        out.contains("a dynamic-storage handle has no whole-value surface"),
        "got:\n{out}"
    );
}

/// A ONE-ELEMENT unpacked array is refused, on purpose and with the reason.
///
/// `sim_ir::NetVar` records `array_len`, which is `1` for `int a[0:0]` and `1`
/// for a scalar; elaborate tells them apart with `unpacked_array_nets`, a table
/// that never reaches the engine. Rendering it as the bare scalar `7` where
/// verilator prints `'{'h7}` would be a silent-wrong at exit 0 — the exact class
/// this feature exists to remove — so the refusal is the ladder-correct answer
/// until something carries array-ness into the IR.
#[test]
fn p_of_a_one_element_unpacked_array_is_loud_with_its_reason() {
    let (out, c) = run("module tb;\n  int one[0:0];\n  initial begin one[0]=7;\n\
         \x20   $display(\"%p\", one); end\nendmodule\n");
    assert_ne!(c, Some(0), "must stay loud; got:\n{out}");
    assert!(
        out.contains("ONE-ELEMENT unpacked array"),
        "the message must name the shape, not the value; got:\n{out}"
    );
}

/// Where vita deliberately differs from its only oracle, with the reason, so the
/// difference is a decision on record rather than a drift.
#[test]
fn p_recorded_divergences_from_verilator() {
    // ① NEGATIVE integer key. vita iterates its `BTreeMap<i64, _>` in SIGNED
    //    order — the order IEEE §7.9.4 gives `first`/`next` for a signed index
    //    type, and the order vita's own `foreach` already uses. verilator sorts
    //    the rendered hex, so it puts `-1` last. And the key WIDTH is gone by the
    //    IR (`DynObj::Assoc` stores `i64`), so `-1` renders 64-bit where
    //    verilator, which still has the declared `int`, prints `'hffffffff`.
    //    Non-negative keys — every key the corpus uses — agree exactly.
    let (out, c) = run(
        "module tb;\n  int nk[int];\n  initial begin nk[-1]=7; nk[2]=8;\n\
         \x20   $display(\"K=%p\", nk); end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "got:\n{out}");
    assert!(
        out.contains("K='{'hffffffffffffffff:'h7, 'h2:'h8}"),
        "signed key order, 64-bit key width; got:\n{out}"
    );

    // ② An unpacked array of `real`. verilator prints ONLY element 0 (`'{1.5,
    //    -0.25}` comes out as `1.5`) while it renders a QUEUE of the same shape
    //    correctly — it contradicts itself, so it is not an oracle for this cell.
    //    vita applies verilator's own recursive rule instead.
    let (out, c) = run(
        "module tb;\n  real ra[2];\n  initial begin ra[0]=1.5; ra[1]=-0.25;\n\
         \x20   $display(\"R=%p\", ra); end\nendmodule\n",
    );
    assert_eq!(c, Some(0), "got:\n{out}");
    assert!(out.contains("R='{1.5, -0.25}"), "got:\n{out}");

    // ③ x/z digits. verilator is 2-state and cannot even compile the assignment
    //    (`Unsupported LHS tristate construct`), so there is no oracle. An
    //    unknown nibble renders exactly as `%0h` renders it — `'h` IS the hex
    //    form, and a second convention for the same digits could only be wrong.
    let (out, c) = run("module tb;\n  logic [7:0] xa[2];\n\
         \x20 initial begin xa[0] = 8'bxxxx_0101; xa[1] = 8'hzz;\n\
         \x20   $display(\"X=%p\", xa); end\nendmodule\n");
    assert_eq!(c, Some(0), "got:\n{out}");
    assert!(out.contains("X='{'hx5, 'hzz}"), "got:\n{out}");
}

/// `%p` reaches every rendering seam, not only `$display` — a severity task, a
/// `$monitor` re-render, and a subroutine body all run the same
/// `render_template`, so the aggregate surface has to be admitted by every
/// lowering that feeds it.
#[test]
fn p_aggregate_works_in_every_print_seam() {
    let (out, c) = run("module tb;\n  int a[2]; int q[$];\n\
         \x20 function automatic void show();\n\
         \x20   $display(\"FN=%p\", a);\n\
         \x20 endfunction\n\
         \x20 initial begin\n\
         \x20   a = '{1,2}; q.push_back(9);\n\
         \x20   $info(\"IN=%p\", q);\n\
         \x20   show();\n\
         \x20   $monitor(\"MO=%p\", a);\n\
         \x20   #1 a[0] = 42;\n\
         \x20   #1 $finish;\n\
         \x20 end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "got:\n{out}");
    for want in [
        "IN='{'h9}",
        "FN='{'h1, 'h2}",
        "MO='{'h1, 'h2}",
        "MO='{'h2a, 'h2}",
    ] {
        assert!(out.contains(want), "missing `{want}` in:\n{out}");
    }
}

/// A CROSS-INSTANCE aggregate (`dut.mem`, `dut.q`) — the idiom `%p` exists for.
///
/// The child's nets do not exist when the parent body lowers, so the argument is
/// the deferred placeholder, which is ALREADY the `Signal { word: None }` shape
/// the local arms build by hand; only the deferred read guard has to know that in
/// a `%p` position an aggregate is the operand (§4.5.376's `hier_mem_args`, same
/// shape). Both lines below are verilator's, measured.
#[test]
fn p_of_a_hierarchical_aggregate() {
    let (out, c) = run("module child;\n  int mem[3]; int q[$];\n\
         \x20 initial begin mem[0]=1; mem[1]=2; mem[2]=3; q.push_back(4); q.push_back(5); end\n\
         endmodule\n\
         module tb;\n  child u();\n\
         \x20 initial #1 $display(\"H=%p Q=%p\", u.mem, u.q);\n\
         endmodule\n");
    assert_eq!(c, Some(0), "got:\n{out}");
    assert!(
        out.contains("H='{'h1, 'h2, 'h3} Q='{'h4, 'h5}"),
        "got:\n{out}"
    );
}

/// …and the exemption is scoped to the `%p` position there too: a hierarchical
/// whole array under any other conversion keeps the read guard, and the
/// one-element refusal is made at the resolver (the only place that can make it
/// for a cross-instance net) with the same reason.
#[test]
fn a_hierarchical_aggregate_outside_a_p_position_stays_loud() {
    let (out, c) = run("module child;\n  int mem[3]; int one[0:0];\n\
         \x20 initial begin mem[0]=1; one[0]=9; end\n\
         endmodule\n\
         module tb;\n  child u();\n\
         \x20 initial begin #1 $display(\"D=%d\", u.mem); $display(\"O=%p\", u.one); end\n\
         endmodule\n");
    assert_ne!(c, Some(0), "both lines must be loud; got:\n{out}");
    assert!(
        out.contains("hierarchical read of `u.mem` is unsupported"),
        "`%d` keeps the read guard; got:\n{out}"
    );
    assert!(
        out.contains("ONE-ELEMENT unpacked array `u.one`"),
        "the resolver makes the same refusal, and names the path; got:\n{out}"
    );
}
