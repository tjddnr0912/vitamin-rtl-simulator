//! IEEE 1800-2017 §26.3: a PACKAGE ROUTINE's body resolves its bare names in the
//! package's scope — its own formals and locals first, then the package's
//! variables and constants — and only a name the package does not declare falls
//! back to the caller.
//!
//! vita lowers every routine body with the CALLER module's flat tables live (an
//! imported routine is injected into the module's `func_table`; a scoped
//! `pk::g()` call is injected under its scoped key), so a bare package VARIABLE
//! inside the body bound wherever the MODULE bound that name: to the module's
//! same-named net (`Z=ee` for both oracles' `Z=123`), to a generate block's, to a
//! module `localparam`, or to nothing (E3010). The scoped spelling refused the
//! body outright, and the inline lane could not even read a package CONSTANT
//! (`gs = C + 1` was `undeclared net/variable t.C`). One stack entry
//! (`pkg_body_scope.rs`, pushed by all four body-lowering lanes) now answers the
//! two shared resolvers, so a whole read, a `[i]` element, a `[m:l]` select, an
//! lvalue and every classifier bind one object.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing). Every
//! value asserted below was measured in BOTH unless the docstring says otherwise.
//! PRE values are from a release binary built at the parent commit.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout without the trailer, stderr, exit code)
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgbody_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    (
        so,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn clean(src: &str) -> String {
    let (o, e, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}\n{e}");
    o
}

fn loud(src: &str, needle: &str) -> String {
    let (o, e, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud run:\n{o}\n{e}");
    assert!(e.contains(needle), "expected {needle:?} in:\n{e}");
    e
}

const PK: &str = r#"package pk;
  logic [15:0] x = 16'h0123;
  int cnt = 7;
  logic [7:0] arr [0:3] = '{1,2,3,4};
  localparam int C = 40;
  function [31:0] g; g = x; endfunction
  function automatic [31:0] ga; ga = x + C; endfunction
  function [31:0] g2; g2 = g() + 1; endfunction
  function [7:0] gi(input int i); gi = arr[i]; endfunction
  function [31:0] gsel; gsel = x[7:4]; endfunction
  function [31:0] gs; gs = C + 1; endfunction
  function [31:0] gf(input [15:0] x); gf = x; endfunction
  function [31:0] gp; gp = pk::x + 1; endfunction
  task automatic tk(output int o); o = cnt; endtask
  task automatic tw; cnt = cnt + 10; endtask
  task ts(output int o); o = x + C; endtask
endpackage
"#;

fn design(body: &str) -> String {
    format!("{PK}module t;\n{body}\nendmodule\n")
}

/// ① THE HEADLINE, both lanes. The bare imported `g()` beside a module `x`, and
/// with no module `x` at all; the scoped `pk::g()` twins of both.
#[test]
fn a_package_body_reads_its_own_package_variable() {
    // PRE: `Z=000000ee` — the module's net.
    let o = clean(&design(
        "import pk::g; logic [7:0] x = 8'hEE; initial begin $display(\"Z=%h\", g()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
    // PRE: E3010 `undeclared net/variable t.x`.
    let o = clean(&design(
        "import pk::g; initial begin $display(\"Z=%h\", g()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
    // PRE: E3009 — the scoped-call gate refused a body naming a package VARIABLE.
    let o = clean(&design(
        "logic [7:0] x = 8'hEE; initial begin $display(\"Z=%h\", pk::g()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
    let o = clean(&design(
        "initial begin $display(\"Z=%h\", pk::g()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
}

/// ② The FRAME lane (`automatic`), reading a variable AND a constant.
#[test]
fn a_frame_body_reads_its_package_variable_and_constant() {
    // PRE: `Z=00000116` (0xEE + 40) and E3009 for the scoped spelling.
    for call in [
        "import pk::ga; initial begin $display(\"Z=%h\", ga()); #1 $finish; end",
        "initial begin $display(\"Z=%h\", pk::ga()); #1 $finish; end",
    ] {
        let o = clean(&design(&format!("logic [7:0] x = 8'hEE; {call}")));
        assert_eq!(o, "Z=0000014b");
    }
}

/// ③ A TRANSITIVE callee that reads a package variable is injected now that the
/// gate admits package variables: `g2` calls `g`.
#[test]
fn a_transitive_callee_reading_a_package_variable_is_injected() {
    // PRE: E3010 `call to undeclared function g` (import) / E3009 "reaches pk::g"
    // (scoped).
    for call in [
        "import pk::g2; initial begin $display(\"Z=%h\", g2()); #1 $finish; end",
        "initial begin $display(\"Z=%h\", pk::g2()); #1 $finish; end",
    ] {
        let o = clean(&design(&format!("logic [7:0] x = 8'hEE; {call}")));
        assert_eq!(o, "Z=00000124");
    }
}

/// ④ The SELECT CHAINS bind the same object: an unpacked-array ELEMENT read and a
/// packed part-select of the package variable, beside a module twin of each name.
#[test]
fn an_element_and_a_part_select_of_the_package_variable_bind_the_package() {
    // PRE: `A=9` (the module's array) and E3009 for the scoped spelling.
    for call in [
        "import pk::gi; initial begin $display(\"A=%0d\", gi(2)); #1 $finish; end",
        "initial begin $display(\"A=%0d\", pk::gi(2)); #1 $finish; end",
    ] {
        let o = clean(&design(&format!(
            "logic [7:0] arr [0:3] = '{{9,9,9,9}}; {call}"
        )));
        assert_eq!(o, "A=3");
    }
    // PRE: `S=0000000e` — bits [7:4] of the module's `8'hEE`.
    let o = clean(&design(
        "import pk::gsel; logic [7:0] x = 8'hEE; initial begin $display(\"S=%h\", gsel()); #1 $finish; end",
    ));
    assert_eq!(o, "S=00000002");
}

/// ⑤ TASKS, both lanes: an automatic task reading the variable into its output;
/// an automatic task WRITING the variable (the write lands in the package, not
/// on the module's same-named net); a STATIC task (the inline task lane, the
/// fourth lane to push the scope) reading a variable and a constant.
#[test]
fn a_package_task_body_reads_and_writes_its_package_variable() {
    // PRE: `T=100`.
    let o = clean(&design(
        "import pk::tk; int cnt = 100; int o; initial begin tk(o); $display(\"T=%0d\", o); #1 $finish; end",
    ));
    assert_eq!(o, "T=7");
    // PRE: `W=120 pk=7` — both increments landed on the MODULE's `cnt`.
    let o = clean(&design(
        "import pk::tw; int cnt = 100; initial begin tw(); tw(); $display(\"W=%0d pk=%0d\", cnt, pk::cnt); #1 $finish; end",
    ));
    assert_eq!(o, "W=100 pk=27");
    // PRE: E3010 `undeclared net/variable t.C`.
    let o = clean(&design(
        "import pk::ts; logic [7:0] x = 8'hEE; int o; initial begin ts(o); $display(\"T=%0d\", o); #1 $finish; end",
    ));
    assert_eq!(o, "T=331");
}

/// ⑥ The INLINE lane reads a package CONSTANT it had no binding for at all —
/// loud → correct, the same hook (the frame lane already bound constants under
/// its own `$func$` scope).
#[test]
fn an_inline_body_reads_its_package_constant() {
    // PRE: E3010 `undeclared net/variable t.C`.
    let o = clean(&design(
        "import pk::gs; initial begin $display(\"S=%0d\", gs()); #1 $finish; end",
    ));
    assert_eq!(o, "S=41");
}

/// ⑦ Every other binder the module can put on the name loses to the package's
/// variable: a module `localparam`, a generate block's local, a wildcard import
/// of the routine, and a continuous-assign call site.
#[test]
fn the_package_variable_wins_over_every_module_binder_of_the_name() {
    // PRE: `Z=000000aa` — the module constant answered (constants resolve before
    // nets in the caller's walk).
    let o = clean(&design(
        "import pk::g; localparam [7:0] x = 8'hAA; initial begin $display(\"Z=%h\", g()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
    // PRE: `Z=000000dd` — the generate block's net.
    let o = clean(&design(
        "import pk::g; logic [7:0] x = 8'hEE; generate if (1) begin : gb logic [7:0] x = 8'hDD; \
         initial begin $display(\"Z=%h\", g()); #1 $finish; end end endgenerate",
    ));
    assert_eq!(o, "Z=00000123");
    // PRE: `Z=000000ee S=0000000e`.
    let o = clean(&design(
        "import pk::g; import pk::gsel; logic [7:0] x = 8'hEE; initial begin $display(\"Z=%h S=%h\", g(), gsel()); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123 S=00000002");
    // PRE: `Z=000000ee`.
    let o = clean(&design(
        "import pk::g; logic [7:0] x = 8'hEE; logic [31:0] w; assign w = g(); initial begin #1 $display(\"Z=%h\", w); $finish; end",
    ));
    assert_eq!(o, "Z=00000123");
}

/// ⑧ CONTROLS, byte-identical to PRE: a FORMAL of the same name wins over the
/// package variable; the explicit `pk::x` spelling inside the body; the variable
/// imported into the module as well; a MODULE function reading the module's `x`.
#[test]
fn controls_a_formal_the_explicit_spelling_and_the_module_twin() {
    let o = clean(&design(
        "import pk::gf; import pk::gp; logic [7:0] x = 8'hEE; \
         function [31:0] mg; mg = x; endfunction \
         initial begin $display(\"F=%h P=%h M=%h\", gf(16'h7), gp(), mg()); #1 $finish; end",
    ));
    assert_eq!(o, "F=00000007 P=00000124 M=000000ee");
    let o = clean(&design(
        "import pk::x; import pk::g; initial begin $display(\"Z=%h x=%h\", g(), x); #1 $finish; end",
    ));
    assert_eq!(o, "Z=00000123 x=0123");
}

/// ⑨ A frame FUNCTION writing a package variable stays LOUD in both lanes — the
/// frame classifier's own rule (a function may not write outside its frame),
/// which the scoped lane now reaches instead of its old gate. Both oracles print
/// `W=8 9 cnt=100 pk=9`; ROADMAP §2 carries it.
#[test]
fn a_function_writing_a_package_variable_stays_loud_in_both_lanes() {
    let pk = "package pk; int cnt = 7; function int gw; cnt = cnt + 1; gw = cnt; endfunction endpackage\n";
    let needle = "assignment to a net outside the function";
    loud(
        &format!("{pk}module t; import pk::gw; int cnt = 100; initial begin $display(\"W=%0d\", gw()); #1 $finish; end endmodule"),
        needle,
    );
    loud(
        &format!("{pk}module t; int cnt = 100; initial begin $display(\"W=%0d\", pk::gw()); #1 $finish; end endmodule"),
        needle,
    );
}

/// ⑩ A FREE name — one the package does not declare — is unchanged: the scoped
/// lane refuses it (the gate's wording now names what it admits), and the
/// import lane still binds it to the caller's net (`Y=ee`, PRE-identical) where
/// BOTH oracles refuse the program (`Unable to bind wire/reg/memory y in pk.gy`).
/// ROADMAP §2 carries the import-lane lenience.
#[test]
fn a_free_name_is_refused_by_the_scoped_lane_and_unchanged_in_the_import_lane() {
    let pk = "package pk; logic [15:0] x = 16'h0123; function [31:0] gy; gy = y; endfunction endpackage\n";
    let e = loud(
        &format!("{pk}module t; logic [7:0] y = 8'hEE; initial begin $display(\"Y=%h\", pk::gy()); #1 $finish; end endmodule"),
        "same-package constants, variables and subroutines",
    );
    assert!(e.contains("a name the package does not declare"), "{e}");
    let o = clean(&format!(
        "{pk}module t; import pk::gy; logic [7:0] y = 8'hEE; initial begin $display(\"Y=%h\", gy()); #1 $finish; end endmodule"
    ));
    assert_eq!(o, "Y=000000ee");
}

/// ⑪ The routine's OWN declarations keep winning, including the pre-existing
/// block-local flatten: a body-local `x` and a block-local `x` in an inline body
/// still read the MODULE's `x` (`ee` for both oracles' 5 / 6 — ROADMAP §2
/// "Scoping / imports / block-locals", the flatten model), byte-identical to PRE.
/// This slice's hook stands down for a name the routine declares, so it neither
/// fixes nor moves that class.
#[test]
fn a_routine_local_of_the_same_name_is_untouched_by_the_hook() {
    let pk = "package pk; logic [15:0] x = 16'h0123;\n\
              function [31:0] gb; begin : blk logic [15:0] x = 16'h5; gb = x; end endfunction\n\
              function [31:0] gl; logic [15:0] x = 16'h6; gl = x; endfunction endpackage\n";
    let o = clean(&format!(
        "{pk}module t; import pk::gb; import pk::gl; logic [7:0] x = 8'hEE; \
         initial begin $display(\"B=%h L=%h\", gb(), gl()); #1 $finish; end endmodule"
    ));
    assert_eq!(o, "B=000000ee L=000000ee");
}

/// ⑫ ROUND-1 SOUNDNESS FINDING (fixed): a routine's BODY-LOCAL `typedef enum`
/// LABELS are its own declarations too (`push_body_enum_labels` binds them
/// innermost), so a same-named package constant or variable must not shadow
/// them. The first draft's `declared` set had formals, locals and block-locals
/// but not labels: `G=7` / `G=f7` / `T=7` / `S=7` where the module twin (and
/// verilator, IEEE §6.21 inner scope) print 3 — iverilog 13 does not honour a
/// routine-local enum label at all (it prints the outer constant even with no
/// package in sight), so the oracle here is the MODULE TWIN, byte-identical.
#[test]
fn a_body_local_enum_label_still_wins_over_a_package_item_of_its_name() {
    let pk = "package pk; localparam int A = 7; logic [15:0] V = 16'h00f7;\n\
              function automatic [31:0] g;  typedef enum int { A = 3, B = 4 } e_t; g = A;  endfunction\n\
              function           [31:0] gs; typedef enum int { V = 3, W = 4 } e_t; gs = V; endfunction\n\
              task automatic t(output int o); typedef enum int { A = 3, B = 4 } e_t; o = A; endtask endpackage\n";
    let o = clean(&format!(
        "{pk}module t; import pk::g; import pk::gs; import pk::t; int o;\n\
         localparam int A = 7;\n\
         function automatic [31:0] mg; typedef enum int {{ A = 3, B = 4 }} e_t; mg = A; endfunction\n\
         initial begin t(o); $display(\"G=%h S=%h T=%0d M=%h\", g(), gs(), o, mg()); #1 $finish; end endmodule"
    ));
    assert_eq!(o, "G=00000003 S=00000003 T=3 M=00000003");
}
