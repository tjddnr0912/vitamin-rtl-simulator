//! A `localparam real` declared INSIDE a generate block was loud — even a bare
//! `localparam real X = 2.5;`. The generate-scope parameter arm folded only
//! through the integer const domain, which returns None for a real, so the whole
//! declaration reported "not a foldable constant expression" and every later read
//! of the name raised a second, misleading "undeclared net/variable".
//!
//! It now routes through `param_real_value` first, exactly as the module-scope
//! path does — so the §11.8.1 ordering (any real operand puts the expression in
//! the real domain) and the i64-twin rule (a twin only when the initializer was
//! wholly integral) are the same rules in both scopes rather than two spellings.
//! Values pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_grp_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

/// A literal, an expression over an outer real param, and pure real arithmetic —
/// all inside a generate-if block.
#[test]
fn generate_scope_real_localparams_fold() {
    let (out, c) = run("module m;\n  localparam real R = 5.0;\n\
           generate if (1) begin : g\n\
             localparam real X = 2.5;\n\
             localparam real Y = R / 2;\n\
             localparam real Z = 1.5 + 1.0;\n\
             initial begin $display(\"V=%0.2f %0.2f %0.2f\", X, Y, Z); #1 $finish; end\n\
           end endgenerate\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("V=2.50 2.50 2.50"),
        "generate-scope reals; got:\n{out}"
    );
}

/// A genvar-dependent real, alongside an integer localparam in the same block —
/// the real side map and the integer param table must both be populated per
/// unrolled instance.
#[test]
fn a_genvar_dependent_real_folds_per_instance() {
    let (out, c) = run("module m;\n  genvar i;\n\
           generate for (i = 0; i < 2; i = i + 1) begin : g\n\
             localparam real S = 1.5 * (i + 1);\n\
             localparam int  N = i + 10;\n\
             initial begin $display(\"L%0d=%0.2f/%0d\", i, S, N); #1 $finish; end\n\
           end endgenerate\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("L0=1.50/10"), "instance 0; got:\n{out}");
    assert!(out.contains("L1=3.00/11"), "instance 1; got:\n{out}");
}

/// The rules are the SAME in both scopes: an exactly-integral initializer keeps
/// its integral capability, and a non-integral real stays loud in an integer
/// context (a generate-scope real must not become usable as a width just because
/// it moved inside a block).
#[test]
fn generate_scope_follows_the_same_twin_rule_as_module_scope() {
    let (out, c) = run("module m;\n\
           generate if (1) begin : g\n\
             localparam real W = 4;\n\
             logic [W-1:0] bus;\n\
             initial begin bus = '1; $display(\"B=%0d\", $bits(bus)); #1 $finish; end\n\
           end endgenerate\nendmodule\n");
    assert_eq!(c, Some(0), "integral real initializer; got:\n{out}");
    assert!(
        out.contains("B=4"),
        "integral twin in generate scope; got:\n{out}"
    );

    // A non-integral one has no twin, so an integral use stays loud.
    let (out, c) = run("module m;\n\
           generate if (1) begin : g\n\
             localparam real W = 2.5;\n\
             logic [W-1:0] bus;\n\
             initial begin bus = '1; $display(\"%0d\", $bits(bus)); #1 $finish; end\n\
           end endgenerate\nendmodule\n");
    assert_ne!(
        c,
        Some(0),
        "non-integral real in a width must stay loud; got:\n{out}"
    );
}
