//! P0 (aes_top round-35, 2026-08-31): a package function containing a `case`
//! made the WHOLE design fail elaboration with E3009 as soon as another package
//! function called it — even though nothing instantiated either one. `import
//! p::*;` alone was enough, and the real design reported it once per instance
//! (201 times).
//!
//! ⭐ The cause was not the `case`. A package routine's body is reserved under
//! TWO frame keys — its bare name and `pkg::name` — and the §12.5 scrutinee
//! capture was recorded in a map keyed by SOURCE SPAN alone. The second
//! reservation overwrote the first, so the first frame lowered a write to a net
//! in the *other* frame's window, which the body validator correctly reported as
//! "an assignment to a net outside the function". Fixed by keying that map (and
//! its `repeat`-counter sibling) on `(span, owning frame)`.
//!
//! ⚠️ The module-scope twin of the same two functions always worked, which is
//! the shape of the whole bug: one hazard, two spellings, only one broken.
//!
//! Values pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgspan_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--top")
        .arg("leaf")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

const PKG: &str = "package p;\n\
   typedef enum logic [1:0] { K0, K1, K2 } kl_e;\n\
   function automatic int unsigned nr_of(input kl_e kl);\n\
     unique case (kl) K0: nr_of = 10; K1: nr_of = 12; default: nr_of = 14; endcase\n\
   endfunction\n\
   localparam int NR_MAX = 14;\n\
   function automatic int unsigned entry_stage(input kl_e kl);\n\
     entry_stage = NR_MAX + 1 - nr_of(kl);\n\
   endfunction\n\
   function automatic int unsigned rpt(input logic [2:0] n);\n\
     rpt = 0;\n\
     repeat (n) rpt = rpt + 3;\n\
   endfunction\n\
   function automatic int unsigned rpt2(input logic [2:0] n);\n\
     rpt2 = 100 + rpt(n);\n\
   endfunction\n\
 endpackage\n";

/// THE REGRESSION, in its reported form: neither function is ever called and the
/// module only imports the package. This was 1 error per instance.
#[test]
fn importing_a_package_whose_function_calls_a_case_function_elaborates() {
    let src = format!(
        "{PKG}module leaf (input logic i_a, output logic o_y);\n\
           import p::*;\n\
           assign o_y = ~i_a;\n\
         endmodule\n"
    );
    let (o, e, ok) = run(&src);
    assert!(ok, "vita failed:\nstdout:\n{o}\nstderr:\n{e}");
    assert!(
        !o.contains("E3009") && !e.contains("E3009"),
        "E3009 came back:\n{o}\n{e}"
    );
}

/// And the values are right, not merely quiet — through BOTH span-keyed maps
/// (`case` capture and `repeat` counter) and through a nested package call.
#[test]
fn the_nested_package_call_still_computes_the_oracle_values() {
    let src = format!(
        "{PKG}module leaf (input logic i_a, output logic o_y);\n\
           import p::*;\n\
           initial begin\n\
             $display(\"A=%0d %0d %0d\", nr_of(K0), nr_of(K1), nr_of(K2));\n\
             $display(\"B=%0d %0d %0d\", entry_stage(K0), entry_stage(K1), entry_stage(K2));\n\
             $display(\"C=%0d %0d %0d\", rpt(3'd0), rpt(3'd2), rpt(3'd5));\n\
             $display(\"D=%0d %0d\", rpt2(3'd2), rpt2(3'd5));\n\
           end\n\
           assign o_y = ~i_a;\n\
         endmodule\n"
    );
    let (o, e, ok) = run(&src);
    assert!(ok, "vita failed:\nstdout:\n{o}\nstderr:\n{e}");
    for want in ["A=10 12 14", "B=5 3 1", "C=0 6 15", "D=106 115"] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// The module-scope twin — byte-identical bodies, and it never broke. Kept so a
/// future fix cannot "repair" the package path by breaking this one.
#[test]
fn the_module_scope_twin_is_unaffected() {
    let src = "module leaf (input logic i_a, output logic o_y);\n\
       function automatic int unsigned nr_of(input logic [1:0] kl);\n\
         case (kl) 2'd0: nr_of = 10; default: nr_of = 14; endcase\n\
       endfunction\n\
       function automatic int unsigned entry(input logic [1:0] kl);\n\
         entry = 15 - nr_of(kl);\n\
       endfunction\n\
       initial $display(\"E=%0d %0d\", entry(2'd0), entry(2'd1));\n\
       assign o_y = ~i_a;\n\
     endmodule\n";
    let (o, e, ok) = run(src);
    assert!(ok, "vita failed:\nstdout:\n{o}\nstderr:\n{e}");
    assert!(o.contains("E=5 1"), "got:\n{o}");
}
