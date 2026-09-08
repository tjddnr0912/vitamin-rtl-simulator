//! §3 ⑤ ⓕ: a `localparam`/`parameter` whose TYPE is an unpacked-array typedef.
//!
//! `typedef_param_shape` used to decline the whole family on
//! `!info.unpacked.is_empty()` — its shape tuple described the ELEMENT and had no
//! slot for unpacked dims, so binding would have produced a scalar of the element
//! type. It now returns those dims as a sixth field and
//! `finish_param_assignment` takes the `ParamItem::ConstArrayVar` channel the
//! explicit spelling (`localparam int P [0:2]`) has always used, so the two
//! spellings are one AST.
//!
//! ORACLE: **verilator 5.052 only**. iverilog 13.0 refuses every cell here —
//! `sorry: unpacked array parameters are not supported yet.` for the explicit
//! spelling and `Unable to evaluate parameter` for the typedef one, and it aborts
//! outright (`Abort trap: 6`) on the real- and string-element forms. Its refusal
//! is parameter-specific, not typedef-specific: the same `typedef int a_t[0:2];`
//! used for a VARIABLE runs on all three tools, which is the control twin that
//! makes the one-oracle count trustworthy.
//!
//! Composition order is the trap `$bits` cannot catch: `localparam a_t P [0:1]`
//! is `P[0:1][0:2]`, 192 bits either way round, so the pins below read `P[0][1]`
//! and `P[1][2]` instead.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_utap_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

/// A `top` module carrying `decls` and one `$display(fmt, args)`.
fn m(decls: &str, fmt: &str, args: &str) -> String {
    format!(
        "module top;\n  {decls}\n  initial begin $display(\"R {fmt}\", {args}); $finish; end\nendmodule\n"
    )
}

fn expect(decls: &str, fmt: &str, args: &str, want: &str) {
    let (out, code) = run(&m(decls, fmt, args));
    assert_eq!(code, Some(0), "must elaborate:\n{out}");
    assert!(
        out.contains(&format!("R {want}")),
        "expected `R {want}` (verilator):\n{out}"
    );
}

#[test]
fn a_localparam_of_an_unpacked_array_typedef_binds_the_array() {
    // The headline cell: E2002 in PRE, `1 2 3 bits=96 size=3` in verilator.
    expect(
        "typedef int a_t [0:2];\n  localparam a_t P = '{1,2,3};",
        "%0d %0d %0d bits=%0d size=%0d",
        "P[0],P[1],P[2],$bits(P),$size(P)",
        "1 2 3 bits=96 size=3",
    );
}

#[test]
fn the_element_type_travels_with_the_dims() {
    // A packed element keeps its width, and a real element its domain — the shape
    // tuple's other five fields still describe the ELEMENT.
    expect(
        "typedef logic [7:0] c_t [0:3];\n  localparam c_t S = '{8'hAA,8'hBB,8'hCC,8'hDD};",
        "%h %h bits=%0d size=%0d",
        "S[1],S[3],$bits(S),$size(S)",
        "bb dd bits=32 size=4",
    );
    expect(
        "typedef real r_t [0:2];\n  localparam r_t T = '{1.5,2.5,3.5};",
        "%0.1f bits=%0d",
        "T[1],$bits(T)",
        "2.5 bits=192",
    );
    // A typedef's own `signed` must be passed as if explicit, or `logic`'s
    // unsigned default would win the way a keyword prefix's does.
    expect(
        "typedef logic signed [3:0] sg_t [0:1];\n  localparam sg_t G = '{-1,2};",
        "%0d %0d bits=%0d",
        "G[0],G[1],$bits(G)",
        "-1 2 bits=8",
    );
}

#[test]
fn every_dimension_spelling_reaches_the_same_channel() {
    // Two dims on the typedef.
    expect(
        "typedef int b_t [0:1][0:2];\n  localparam b_t R = '{'{1,2,3},'{4,5,6}};",
        "%0d %0d bits=%0d s1=%0d s2=%0d",
        "R[0][2],R[1][0],$bits(R),$size(R,1),$size(R,2)",
        "3 4 bits=192 s1=2 s2=3",
    );
    // The SIZE form `[3]`, normalized to `[0:2]` by the same `parse_dim`.
    expect(
        "typedef int e_t [3];\n  localparam e_t U = '{7,8,9};",
        "%0d %0d bits=%0d size=%0d",
        "U[0],U[2],$bits(U),$size(U)",
        "7 9 bits=96 size=3",
    );
    // A dim that NAMES a parameter — the dims are expressions, not folded numbers.
    expect(
        "localparam N = 4;\n  typedef int n_t [0:N-1];\n  localparam n_t W = '{1,2,3,4};",
        "%0d %0d bits=%0d size=%0d",
        "W[0],W[3],$bits(W),$size(W)",
        "1 4 bits=128 size=4",
    );
}

#[test]
fn the_names_dims_come_before_the_typedefs() {
    // `$bits` is 192 under EITHER composition order, so the pin reads elements.
    // verilator: `P[0][1]` = 2 and `P[1][2]` = 6 ⇒ `[0:1]` then `[0:2]`.
    expect(
        "typedef int a_t [0:2];\n  localparam a_t X [0:1] = '{'{1,2,3},'{4,5,6}};",
        "%0d %0d bits=%0d s1=%0d s2=%0d",
        "X[0][1],X[1][2],$bits(X),$size(X,1),$size(X,2)",
        "2 6 bits=192 s1=2 s2=3",
    );
}

#[test]
fn a_package_scoped_default_and_a_package_localparam_both_bind() {
    // `pk::a_t` as the prefix, and a `localparam` of that type INSIDE the package
    // (IEEE §6.20.1 makes a package `parameter` a localparam, so both spellings
    // reach the channel and neither trips the overridable-`parameter` gate).
    let src = "package pk;\n  typedef int pa_t [0:2];\n  localparam pa_t PP = '{5,6,7};\n\
               endpackage\nmodule top;\n  localparam pk::pa_t Q = '{4,5,6};\n\
               \x20 initial begin $display(\"R %0d %0d bits=%0d PKG %0d %0d\", Q[0], Q[2], $bits(Q), pk::PP[0], pk::PP[2]); $finish; end\nendmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R 4 6 bits=96 PKG 5 7"),
        "the scoped prefix and the package localparam both bind (verilator):\n{out}"
    );
}

#[test]
fn an_ansi_header_default_takes_an_override() {
    // The win the queue row did not claim: routing into `ConstArrayVar` gets the
    // header ARRAY parameter's existing scalar TWIN (`array_param_twin`) for free,
    // so `#(.P(…))` overrides a typedef-typed array parameter.
    let src = "typedef int a_t [0:2];\ntypedef logic signed [3:0] sg_t [0:1];\n\
        module dut #(parameter a_t P = '{1,2,3}, parameter sg_t G = '{-1,2}) ();\n\
        \x20 initial $display(\"R %0d %0d bits=%0d G %0d %0d gb=%0d\", P[0],P[2],$bits(P), G[0],G[1],$bits(G));\n\
        endmodule\nmodule top;\n  dut u0 ();\n  dut #(.P('{7,8,9})) u1 ();\n\
        \x20 dut #(.P('{4,5,6}), .G('{-2,3})) u2 ();\n  initial begin #1; $finish; end\nendmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    for want in [
        "R 1 3 bits=96 G -1 2 gb=8",
        "R 7 9 bits=96 G -1 2 gb=8",
        "R 4 6 bits=96 G -2 3 gb=8",
    ] {
        assert!(out.contains(want), "missing `{want}`:\n{out}");
    }
}

#[test]
fn the_shapes_with_no_channel_stay_loud_and_name_their_own_reason() {
    // Negative controls, each loud for a reason its EXPLICIT twin is equally loud
    // for — so nothing here is a split this slice could have closed.
    //
    // (a) a string element: `parse_array_param` needs a `var_kind` and the `String`
    // arm has none. Its explicit twin `localparam string EX [0:1]` is also loud.
    let (out, code) = run(
        "module top;\n  typedef string st_t [0:1];\n  localparam st_t SS = '{\"a\",\"b\"};\n\
         \x20 initial begin $display(\"R %0s\", SS[0]); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("string unpacked-array typedef parameter"),
        "the string element names its OWN reason, not one about `localparam int`:\n{out}"
    );

    // (b) a module-BODY overridable `parameter` of that type: no override channel
    // at all, exactly like the explicit `parameter int EBP [0:2]`. The message is
    // now the array gate's rather than the typedef decline's — the accurate one.
    let (out, code) = run(
        "module top;\n  typedef int a_t [0:2];\n  parameter a_t BP = '{1,2,3};\n\
         \x20 initial begin $display(\"R %0d\", BP[0]); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("`localparam` for an array parameter"),
        "a body `parameter` array is refused by the array gate:\n{out}"
    );
}

#[test]
fn the_explicit_spelling_is_unchanged() {
    // The channel this slice routes INTO. Byte-identical before and after; here so
    // a future edit to `parse_array_param`'s new preset argument cannot move it.
    expect(
        "localparam int Q [0:2] = '{1,2,3};",
        "%0d %0d %0d bits=%0d size=%0d",
        "Q[0],Q[1],Q[2],$bits(Q),$size(Q)",
        "1 2 3 bits=96 size=3",
    );
    expect(
        "localparam logic [7:0] C [0:3] = '{8'hAA,8'hBB,8'hCC,8'hDD};",
        "%h %h bits=%0d size=%0d",
        "C[1],C[3],$bits(C),$size(C)",
        "bb dd bits=32 size=4",
    );
}

#[test]
fn a_typedef_declaration_of_the_same_type_still_works() {
    // The control twin that gives the one-oracle count its meaning: as a VARIABLE
    // the identical typedef runs on vita, iverilog AND verilator (`1 2 3`), so
    // iverilog's refusal above is about the PARAMETER, not the type.
    expect(
        "typedef int a_t [0:2];\n  a_t V = '{1,2,3};",
        "%0d %0d %0d bits=%0d",
        "V[0],V[1],V[2],$bits(V)",
        "1 2 3 bits=96",
    );
}
