//! Uppercase format-spec letters in `$display`-family format strings (IEEE 1364 —
//! the conversion letter is case-insensitive, except `%E`/`%G`/`%F` uppercase the
//! exponent letter / inf-nan like C printf). vita already handled `%D`/`%H`/`%O`/
//! `%B`/`%X`/`%V`/`%P`, but `%S`/`%C`/`%M` fell through to a literal `%S` (and left
//! the argument unconsumed), and `%E`/`%G` rendered a lowercase exponent — both
//! silent-wrongs. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ucf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn uppercase_s_is_string_alias() {
    // `%S` == `%s`, including the field width: `%5S` of "hi" → "   hi".
    let out = run(
        "module top; initial begin $display(\"[%S][%5S]\",\"hi\",\"hi\"); $finish; end endmodule\n",
    );
    assert!(out.contains("[hi][   hi]"), "%S alias; got:\n{out}");
}

#[test]
fn uppercase_c_is_char_alias() {
    let out =
        run("module top; initial begin $display(\"[%C][%c]\",65,66); $finish; end endmodule\n");
    assert!(out.contains("[A][B]"), "%C alias; got:\n{out}");
}

#[test]
fn uppercase_m_is_scope_alias() {
    let out = run("module sub; initial $display(\"[%M]\"); endmodule\n\
         module top; sub u(); initial #1 $finish; endmodule\n");
    assert!(out.contains("[top.u]"), "%M alias; got:\n{out}");
}

#[test]
fn uppercase_e_g_uppercase_exponent() {
    // `%E`/`%G` uppercase the exponent letter; `%e`/`%g` stay lowercase; `%F` of a
    // finite value is unchanged.
    let out = run(
        "module top; real sml,big; initial begin sml=3.14; big=1.5e20; \
         $display(\"e[%e]E[%E]g[%g]G[%G]F[%F]\",sml,sml,big,big,sml); $finish; end endmodule\n",
    );
    assert!(
        out.contains("e[3.140000e+00]E[3.140000E+00]g[1.5e+20]G[1.5E+20]F[3.140000]"),
        "%E/%G uppercase exponent; got:\n{out}"
    );
}

#[test]
fn uppercase_eg_nonfinite_but_f_lowercase() {
    // `%E`/`%G` uppercase inf/nan (→ INF/NAN); `%F` keeps them LOWERCASE (it equals
    // `%f` for every value) — iverilog-pinned. (An adversarial differential caught
    // an over-eager `%F` uppercasing.)
    let out = run(
        "module top; real p,n,z; initial begin p=1.0/0.0; n=-1.0/0.0; z=0.0/0.0; \
         $display(\"[%E][%F][%G][%E][%F]\",p,p,p,n,n); $display(\"[%E][%F]\",z,z); \
         $finish; end endmodule\n",
    );
    assert!(
        out.contains("[INF][inf][INF][-INF][-inf]") && out.contains("[NAN][nan]"),
        "%E/%G upper, %F lower for inf/nan; got:\n{out}"
    );
}

#[test]
fn uppercase_t_consumes_its_arg() {
    // `%T` aliases `%t` and must CONSUME one arg, so a following `%d` reads the next
    // value (the bug left `%T` literal and shifted every later arg). `%T` shares
    // vita's documented plain-decimal `%t` rendering, so it equals vita `%t` exactly.
    let out = run("module top; integer a,b; initial begin a=5; b=9; \
         $display(\"T[%T]d[%d]\",a,b); $display(\"t[%t]d[%d]\",a,b); $finish; end endmodule\n");
    // Both lines identical (T behaves as t) and the %d sees b=9 (no arg shift).
    assert!(
        out.contains("T[5]d[          9]") && out.contains("t[5]d[          9]"),
        "%T consumes arg, == %t; got:\n{out}"
    );
}

#[test]
fn lowercase_specs_unchanged() {
    // Byte-identity: the lowercase forms are untouched.
    let out = run(
        "module top; reg [7:0] r; real x; initial begin r=8'hab; x=3.14; \
         $display(\"[%h][%d][%s][%e]\",r,r,\"hi\",x); $finish; end endmodule\n",
    );
    assert!(
        out.contains("[ab][171][hi][3.140000e+00]"),
        "lowercase unchanged; got:\n{out}"
    );
}
