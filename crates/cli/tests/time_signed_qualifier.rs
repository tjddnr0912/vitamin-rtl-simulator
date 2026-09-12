//! An explicit `signed` qualifier on a `time` declaration (ROADMAP §5.2 row 3).
//!
//! `time` is 64-bit and UNSIGNED BY DEFAULT (IEEE 1800 §6.11.2), but `signed` /
//! `unsigned` are legal qualifiers on it like on any other integer atom type. The
//! parser resolved the qualifier correctly (`hdl-parser/src/decls.rs`
//! `atom_default_signed(Time) = false`, then `opt_signed`), and elaborate then threw
//! the resolved flag away: `array_geom::kind_signedness` mapped `Time` to unsigned
//! unconditionally, and `const_fn_width.rs` carried a second copy of the same
//! hard-code. So `time signed k; k = -8;` read UNSIGNED on every consumer —
//! `k/2` gave 9223372036854775804, `k < 0` gave 0, `%0d` gave 18446744073709551608.
//!
//! `kind_signedness` has one caller (`range_to_dims_opt`) and ~25 `range_to_dims*`
//! callers behind it, so one edit moved every declaration site at once; the tests
//! below are one per SITE (module var, both port forms, block-local, typedef,
//! package, unpacked array, queue, class property, function return type, both task
//! routes) plus the consumer, sign-extension and boundary axes.
//!
//! Every expected value was measured live on iverilog 13 (`-g2012`) and verilator
//! 5.052 (`--binary --timing`); the two agree on every line here except where a
//! comment says otherwise. The CONTROLS (plain `time`, `time unsigned`,
//! `typedef time`, a plain `time` member of a packed struct) are pinned in the same
//! file so a future change cannot flip the DEFAULT while fixing the qualifier.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_timesigned_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    // stdout AND stderr: `$display` goes to stdout, diagnostics to stderr, and the
    // last test in this file pins a parse ERROR.
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

fn ok(src: &str) -> String {
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "must run quiet;\n{out}");
    out
}

#[test]
fn a_signed_time_module_variable_is_signed_on_every_consumer() {
    // The root cell. All four probes were unsigned before: 9223372036854775804,
    // 18446744073709551608, 0, 2.
    let out = ok(r#"module t;
  time signed k;
  initial begin
    k = -8;
    $display("k/2=%0d", k/2);
    $display("k=%0d", k);
    $display("lt=%0d", k < 0);
    $display("mod=%0d", k%3);
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("k/2=-4"), "signed divide;\n{out}");
    assert!(out.contains("k=-8"), "signed %0d;\n{out}");
    assert!(out.contains("lt=1"), "signed compare;\n{out}");
    assert!(out.contains("mod=-2"), "signed modulo;\n{out}");
}

#[test]
fn the_defect_needs_a_sink_at_least_sixty_four_bits_wide() {
    // ⚠️ THE TRAP THIS ROW WAS FILED WITH. Returning `k/2` through a 32-bit
    // `function integer` TRUNCATES the wrong unsigned quotient 9223372036854775804
    // to 32 bits, where it reads back as -4 — the right answer for the wrong
    // reason. This cell was CORRECT before the fix and is correct after it, so it
    // is the accidental-immunity control; the `longint` twin below is the one that
    // actually showed the defect.
    let out = ok(r#"module t;
  function integer f(input time signed k); f = k/2; endfunction
  initial begin $display("f=%0d", f(-8)); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("f=-4"),
        "32-bit sink is immune either way;\n{out}"
    );

    // Same design, 64-bit sink: 9223372036854775804 before, -4 now.
    let out = ok(r#"module t;
  function automatic longint signed f(input time signed k);
    $display("in-body k/2=%0d", k/2);
    $display("in-body k=%0d", k);
    $display("in-body lt=%0d", k < 0);
    f = k/2;
  endfunction
  initial begin
    $display("f=%0d", f(-8));
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("in-body k/2=-4"), "{out}");
    assert!(out.contains("in-body k=-8"), "{out}");
    assert!(out.contains("in-body lt=1"), "{out}");
    assert!(out.contains("f=-4"), "{out}");
}

#[test]
fn both_task_routes_read_a_signed_time_formal_as_signed() {
    // The defect was not route-specific: `--obs-dir` run.json reports the static
    // task as route `inlined` and the automatic one as `frame`, and both were
    // 9223372036854775804 / lt=0 before.
    let out = ok(r#"module t;
  task tk(input time signed k);
    $display("task k/2=%0d", k/2);
    $display("task lt=%0d", k < 0);
  endtask
  initial begin tk(-8); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("task k/2=-4") && out.contains("task lt=1"),
        "inlined route;\n{out}"
    );

    let out = ok(r#"module t;
  task automatic tk(input time signed k);
    $display("atask k/2=%0d", k/2);
    $display("atask lt=%0d", k < 0);
  endtask
  initial begin tk(-8); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("atask k/2=-4") && out.contains("atask lt=1"),
        "frame route;\n{out}"
    );

    // A frame-local `automatic time signed` declaration inside a frame function.
    // ⚠️ iverilog ABORTS on this shape (`Assertion failed: (index < get_max(fun_thr,
    // val)), of_RET_VEC4, vthread.cc:5466`), so verilator is the SOLE oracle here.
    let out = ok(r#"module t;
  function automatic longint signed f(input time signed k);
    automatic time signed loc = k;
    f = loc/2;
  endfunction
  initial begin $display("frame f=%0d", f(-8)); #1 $finish; end
endmodule
"#);
    assert!(out.contains("frame f=-4"), "verilator-only oracle;\n{out}");
}

#[test]
fn both_port_declaration_forms_carry_the_qualifier() {
    let out = ok(r#"module sub(input time signed k);
  initial #1 $display("ansi k/2=%0d lt=%0d", k/2, k<0);
endmodule
module t;
  reg signed [63:0] kk;
  sub u(.k(kk));
  initial begin kk = -8; #2 $finish; end
endmodule
"#);
    assert!(out.contains("ansi k/2=-4 lt=1"), "ANSI port;\n{out}");

    let out = ok(r#"module sub(k);
  input k; time signed k;
  initial #1 $display("nonansi k/2=%0d lt=%0d", k/2, k<0);
endmodule
module t;
  reg signed [63:0] kk;
  sub u(.k(kk));
  initial begin kk = -8; #2 $finish; end
endmodule
"#);
    assert!(out.contains("nonansi k/2=-4 lt=1"), "non-ANSI port;\n{out}");
}

#[test]
fn every_other_declaration_site_carries_the_qualifier() {
    // One assertion per site. Each was 9223372036854775804 before the fix; all of
    // them funnel through `range_to_dims*` into the single `kind_signedness` call.
    let out = ok(r#"module t;
  initial begin : blk
    time signed k;
    k = -8;
    $display("blocklocal k/2=%0d", k/2);
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("blocklocal k/2=-4"), "block-local;\n{out}");

    let out = ok(r#"typedef time signed st;
module t;
  st k;
  initial begin k = -8; $display("typedef k/2=%0d", k/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("typedef k/2=-4"), "typedef;\n{out}");

    let out = ok(r#"package p; time signed k; endpackage
module t;
  import p::*;
  initial begin k = -8; $display("pkgvar k/2=%0d", k/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("pkgvar k/2=-4"), "package variable;\n{out}");

    let out = ok(r#"module t;
  time signed arr [0:1];
  initial begin arr[0] = -8; $display("arr k/2=%0d", arr[0]/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("arr k/2=-4"), "unpacked array element;\n{out}");

    let out = ok(r#"module t;
  time signed q[$];
  initial begin q.push_back(-8); $display("queue k/2=%0d", q[0]/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("queue k/2=-4"), "queue element;\n{out}");

    let out = ok(r#"class C; time signed k; endclass
module t;
  C c;
  initial begin c = new(); c.k = -8; $display("class k/2=%0d", c.k/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("class k/2=-4"), "class property;\n{out}");

    let out = ok(r#"module t;
  function time signed f; f = -8; endfunction
  initial begin $display("funcret f/2=%0d", f()/2); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("funcret f/2=-4"),
        "function return type;\n{out}"
    );
}

#[test]
fn the_consumer_axis_is_signed_including_sign_extension() {
    let out = ok(r#"module t;
  time signed k; longint signed w;
  initial begin
    k = -8;
    $display("div=%0d", k/2);
    $display("arsh=%0d", k>>>1);
    $display("lt=%0d", k<0);
    $display("signed=%0d", $signed(k));
    $display("unsigned=%0d", $unsigned(k));
    $display("cmpneg=%0d", k == -8);
    w = k;
    $display("w=%0d", w);
    $display("mul=%0d", k*2);
    #1 $finish;
  end
endmodule
"#);
    assert!(out.contains("div=-4"), "{out}");
    assert!(
        out.contains("arsh=-4"),
        "arithmetic right shift needs the SIGN;\n{out}"
    );
    assert!(out.contains("lt=1"), "{out}");
    // ⚠️ Controls inside the same run: an explicit cast and a 2's-complement
    // equality were already right before the fix (they are sign-blind or override
    // it), so they say nothing about the qualifier — pinned so a later change
    // cannot break them while claiming this row.
    assert!(
        out.contains("signed=-8") && out.contains("unsigned=18446744073709551608"),
        "{out}"
    );
    assert!(out.contains("cmpneg=1"), "{out}");
    assert!(out.contains("w=-8"), "equal-width copy;\n{out}");
    assert!(out.contains("mul=-16"), "{out}");

    // Sign EXTENSION into a wider target — the one consumer where the fix changes
    // the stored BITS, not only the %0d rendering: `00fffffffffffffff8` before.
    let out = ok(r#"module t;
  time signed k; logic signed [71:0] w;
  initial begin k = -8; w = k; $display("sext w=%0d", w); $display("hex w=%h", w); #1 $finish; end
endmodule
"#);
    assert!(out.contains("sext w=-8"), "{out}");
    assert!(
        out.contains("hex w=fffffffffffffffff8"),
        "72-bit sign fill;\n{out}"
    );

    // An NBA does not launder it.
    let out = ok(r#"module t;
  time signed k; time signed q; reg clk=0;
  always #1 clk=~clk;
  always @(posedge clk) q <= k/2;
  initial begin k = -8; @(posedge clk); @(posedge clk); $display("nba q=%0d", q); $finish; end
endmodule
"#);
    assert!(out.contains("nba q=-4"), "{out}");
}

#[test]
fn the_signed_time_boundaries_are_two_s_complement() {
    let out = ok(r#"module t;
  time signed k;
  initial begin
    k = 64'h8000000000000000; $display("min/2=%0d", k/2);
    k = 64'h7FFFFFFFFFFFFFFF; $display("max/2=%0d", k/2);
    k = 0; $display("zero/2=%0d", k/2);
    k = -1; $display("m1 lt=%0d d=%0d", k<0, k);
    #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("min/2=-4611686018427387904"),
        "most-negative;\n{out}"
    );
    // The positive and zero cells are the controls: they read the same either way.
    assert!(out.contains("max/2=4611686018427387903"), "{out}");
    assert!(out.contains("zero/2=0"), "{out}");
    assert!(out.contains("m1 lt=1 d=-1"), "all-ones;\n{out}");
}

#[test]
fn a_const_function_returning_signed_time_folds_signed() {
    // `const_fn_width.rs` held a SECOND copy of the `Time => false` hard-code. This
    // cell was already correct through a different path, so it is a drift watcher:
    // it fails only if the two containers disagree about the same declaration.
    let out = ok(r#"module t;
  function automatic time signed cf(input int x); cf = -8; endfunction
  localparam longint signed P = cf(0)/2;
  initial begin $display("constfn P=%0d", P); #1 $finish; end
endmodule
"#);
    assert!(out.contains("constfn P=-4"), "{out}");
}

#[test]
fn an_unqualified_time_stays_unsigned() {
    // ⚠️ THE REGRESSION WATCHERS. `atom_default_signed(Time) = false`, so a plain or
    // explicitly `unsigned` `time` still arrives `signed = false`; these four cells
    // must keep the UNSIGNED answers they had before the row.
    let out = ok(r#"module t;
  time k;
  initial begin k = -8; $display("k/2=%0d lt=%0d", k/2, k<0); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("k/2=9223372036854775804 lt=0"),
        "plain `time`;\n{out}"
    );

    let out = ok(r#"module t;
  time unsigned k;
  initial begin k = -8; $display("tu k/2=%0d", k/2); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("tu k/2=9223372036854775804"),
        "`time unsigned`;\n{out}"
    );

    let out = ok(r#"typedef time tu;
module t;
  tu c;
  initial begin c = -8; $display("td c/2=%0d", c/2); #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("td c/2=9223372036854775804"),
        "`typedef time`;\n{out}"
    );

    // Both packed-struct members in one run: the plain one stays unsigned, the
    // qualified one is signed. ⚠️ Neither goes through `kind_signedness` at all —
    // the parser's struct-member desugar carries `TypeInfo.signed` itself — so this
    // pair also proves the fix did not DOUBLE-apply a sign on that path.
    let out = ok(r#"module t;
  typedef struct packed { time k; } s_t;
  typedef struct packed { logic [31:0] hi; time signed k; } s2_t;
  s_t s; s2_t s2;
  initial begin s.k = -8; s2.k = -8;
    $display("structplain k/2=%0d", s.k/2);
    $display("struct2 k/2=%0d", s2.k/2); #1 $finish; end
endmodule
"#);
    assert!(out.contains("structplain k/2=9223372036854775804"), "{out}");
    assert!(out.contains("struct2 k/2=-4"), "{out}");
}

#[test]
fn the_other_qualifier_kinds_are_unchanged() {
    // `time` was the ONLY kind whose qualifier was discarded; these ten were
    // already right and are pinned so the shared `kind_signedness` edit is shown to
    // have moved exactly one arm.
    let out = ok(r#"typedef byte unsigned bu; typedef logic signed [7:0] ls;
module t;
  int unsigned a; integer unsigned b; byte unsigned c; bit signed [7:0] d;
  logic signed [7:0] e; longint unsigned g; shortint unsigned h; reg signed [7:0] i2;
  bu ta; ls tb;
  initial begin
    a=-8; b=-8; c=-8; d=-8; e=-8; g=-8; h=-8; i2=-8; ta=-8; tb=-8;
    $display("a=%0d b=%0d c=%0d d=%0d", a/2, b/2, c/2, d/2);
    $display("e=%0d g=%0d h=%0d i2=%0d", e/2, g/2, h/2, i2/2);
    $display("ta=%0d tb=%0d", ta/2, tb/2);
    #1 $finish; end
endmodule
"#);
    assert!(
        out.contains("a=2147483644 b=2147483644 c=124 d=-4"),
        "{out}"
    );
    assert!(
        out.contains("e=-4 g=9223372036854775804 h=32764 i2=-4"),
        "{out}"
    );
    assert!(out.contains("ta=124 tb=-4"), "{out}");
}

#[test]
fn the_time_domain_itself_stays_unsigned() {
    // ⚠️ THE SURFACE THE ROW DID NOT MEASURE. `Time` nets back `$time` capture and
    // `#delay`, so a newly-signed 64-bit could have changed a comparison or a delay
    // computation. It does not: the simulation TIME is a separate 64-bit unsigned
    // quantity, and a `time signed` variable only changes how the VARIABLE reads.
    // Every line below is identical before and after the row, on all three tools.
    let out = ok(r#"module top;
  time t1; time signed t2;
  initial begin
    #17;
    t1 = $time; t2 = $time;
    $display("T1=%0d T2=%0d LT1=%0d LT2=%0d", t1, t2, t1 < 0, t2 < 0);
    $display("D1=%0d D2=%0d", t1/2, t2/2);
    #3 $finish;
  end
endmodule
"#);
    assert!(
        out.contains("T1=17 T2=17 LT1=0 LT2=0"),
        "a positive $time is positive either way;\n{out}"
    );
    assert!(out.contains("D1=8 D2=8"), "{out}");

    let out = ok(r#"module top;
  time d1; time signed d2;
  initial begin
    d1 = 7; d2 = 7;
    #(d1) $display("A=%0d", $time);
    #(d2) $display("B=%0d", $time);
    #1 $finish;
  end
endmodule
"#);
    assert!(
        out.contains("A=7") && out.contains("B=14"),
        "delay from a time variable;\n{out}"
    );

    // ⚠️ The sharp one: a NEGATIVE `time signed` used as a delay. Both oracles
    // advance by 2^64-8, i.e. the delay expression is consumed UNSIGNED even though
    // the variable now reads signed — so the fix must not have made `#(d2)` wait a
    // negative amount, and must not have turned $time's own rendering signed.
    let out = ok(r#"module top;
  time signed d2;
  initial begin
    d2 = -8;
    $display("PRE=%0d", $time);
    #(d2) $display("A=%0d", $time);
    #2 $display("C=%0d", $time);
    $finish;
  end
endmodule
"#);
    assert!(out.contains("PRE=0"), "{out}");
    assert!(
        out.contains("A=18446744073709551608"),
        "delay stays unsigned;\n{out}"
    );
    assert!(
        out.contains("C=18446744073709551610"),
        "and $time renders unsigned;\n{out}"
    );
}

#[test]
fn a_signed_time_parameter_is_still_loud() {
    // ⚠️ NOT FIXED BY THIS ROW, pinned so it is not read as support and is not
    // "fixed" by accident. `parameter`/`localparam` reject the qualifier in the
    // PARSER (`hdl-parser/src/params.rs`), upstream of any signedness decision, so
    // the elaborate-side fix cannot reach them. Both oracles accept both spellings
    // and print -4. Filed as a separate loud→supported item.
    for src in [
        "module t;\n  parameter time signed T = -8;\n  initial begin $display(\"%0d\", T/2); #1 $finish; end\nendmodule\n",
        "module t;\n  localparam time signed T = -8;\n  initial begin $display(\"%0d\", T/2); #1 $finish; end\nendmodule\n",
    ] {
        let (out, code) = run(src);
        assert_eq!(code, Some(1), "still a parse error;\n{out}");
        assert!(
            out.contains("error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN")
                && out.contains("found keyword 'signed'"),
            "loud, not silent;\n{out}"
        );
    }
}
