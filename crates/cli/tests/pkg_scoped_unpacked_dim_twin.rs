//! §3 ⑤ⓕ residue: the package-scoped twin of an UNPACKED-array typedef whose
//! dimension names a package constant.
//!
//! §4.5.415 taught the `pkg::T` twin to respell the package's own constants as
//! `pkg::W`, because the bare name is undefined wherever the twin is used without
//! importing the package — and it respelled `range` and `packed`. `unpacked` is a
//! THIRD container of the same expression type and was left as written, so
//! `typedef logic [7:0] a_t [0:N-1];` in a package carried a bare `N` out.
//!
//! That cost two ladder rungs at once:
//!   * LOUD where the oracles fold — `$bits(pk::a_t)` and a declaration `pk::a_t v;`
//!     at a use site that never imported the package, while the PACKED twin
//!     `$bits(pk::p_t)` beside it folded;
//!   * SILENT-WRONG where the importer happens to declare the same name — with a
//!     `localparam N = 9;` in the module, the bare `N` bound THERE and `$bits`
//!     read 72 where both oracles read 32.
//!
//! Every value is pinned to LIVE iverilog 13.0 and verilator 5.052.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_psu_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn shows(src: &str, want: &str) {
    let (out, code) = run(src);
    assert_eq!(code, 0, "expected exit 0:\n{out}");
    assert!(out.contains(want), "wanted `{want}`:\n{out}");
}

const PKG: &str = "package pk; localparam N = 4; localparam M = 3;\n\
     typedef logic [7:0] a_t [0:N-1];\n\
     typedef logic [7:0] s_t [N];\n\
     typedef logic [7:0] d2_t [0:N-1][0:M-1];\n\
     typedef logic [N-1:0] p_t;\n\
   endpackage\n";

/// Every unpacked spelling the respell has to carry, through the scoped name, at a
/// use site that never imported the package.
#[test]
fn a_scoped_unpacked_typedef_folds_its_package_constant() {
    for (ty, want) in [("a_t", "R=32"), ("s_t", "R=32"), ("d2_t", "R=96")] {
        shows(
            &format!(
                "{PKG}module top;\n\
                   initial begin $display(\"R=%0d\", $bits(pk::{ty})); $finish; end\n\
                 endmodule\n"
            ),
            want,
        );
    }
}

/// The SILENT half, and the reason the respell exists at all: an importer that
/// declares the same NAME. The bare `N` used to bind to the module's 9 and size the
/// type 72 bits at exit 0; both oracles read the package's 4, i.e. 32.
#[test]
fn an_importers_same_named_constant_does_not_capture_the_dim() {
    shows(
        &format!(
            "{PKG}module top;\n\
               localparam N = 9;\n\
               initial begin $display(\"R=%0d\", $bits(pk::a_t)); $finish; end\n\
             endmodule\n"
        ),
        "R=32",
    );
}

/// A DECLARATION of the scoped type, not just a `$bits` of it — the same twin sizes
/// the net, and it was loud for the same reason.
#[test]
fn a_declaration_of_the_scoped_type_sizes_from_the_package() {
    shows(
        &format!(
            "{PKG}module top;\n\
               pk::a_t v;\n\
               initial begin v[2] = 8'ha5;\n\
                 $display(\"R=%0d %h\", $bits(v), v[2]); $finish; end\n\
             endmodule\n"
        ),
        "R=32 a5",
    );
}

/// Controls: a `parameter` spelling of the package constant (§6.20.1 makes it a
/// localparam, so it must answer identically), the packed twin that already
/// respelled, an all-literal typedef, and the wildcard-imported bare name.
#[test]
fn the_neighbouring_spellings_are_unchanged() {
    shows(
        "package pk; parameter N = 4; typedef logic [7:0] a_t [0:N-1]; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(pk::a_t)); $finish; end endmodule\n",
        "R=32",
    );
    shows(
        &format!(
            "{PKG}module top; initial begin $display(\"R=%0d\", $bits(pk::p_t)); $finish; end endmodule\n"
        ),
        "R=4",
    );
    shows(
        "package pk; typedef logic [7:0] a_t [0:3]; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(pk::a_t)); $finish; end endmodule\n",
        "R=32",
    );
    shows(
        &format!(
            "{PKG}module top; import pk::*;\n\
               initial begin $display(\"R=%0d\", $bits(a_t)); $finish; end endmodule\n"
        ),
        "R=32",
    );
}

/// `Dyn` / `Queue` / `Assoc` pass through the respell unchanged and stay loud —
/// they carry no bound to bind, and neither oracle answers `$bits` of one.
#[test]
fn a_dynamic_scoped_typedef_stays_loud() {
    let (out, code) = run("package pk; typedef logic [7:0] d_t []; endpackage\n\
         module top; initial begin $display(\"R=%0d\", $bits(pk::d_t)); $finish; end endmodule\n");
    assert_eq!(code, 1, "{out}");
    assert!(!out.contains("R="), "{out}");
}
