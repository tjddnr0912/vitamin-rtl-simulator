//! A2a — body array parameter `localparam <type> NAME [dims] = '{…}`
//! (IEEE 1800 §6.20.2).
//!
//! No live oracle (iverilog-13.0: "sorry: unpacked array parameters are not
//! supported yet") → hand-IEEE pin + the vita-INTERNAL equivalence
//! differential: the parser desugars the parameter to the EXACT `NetVarDecl`
//! the equivalent variable-array decl parses to, so the two forms must be
//! byte-identical on stdout. On top of the storage reuse, elaborate registers
//! the net as an elaboration constant: every write path (procedural =/<=,
//! whole-array `= '{…}` / array copy, force, continuous assign, $readmem,
//! SYS-READ dests, task output actuals) is a loud E3009 — a parameter must
//! never be silently mutable.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_aparam_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The internal-differential teeth: `localparam <ty> X […] = '{…}` must be
/// byte-identical (stdout) to the equivalent var decl `<ty> X […] = '{…}`.
fn assert_twin(reads: &str, param_decl: &str, var_decl: &str) {
    let p = format!("module t; {param_decl} initial begin {reads} $finish; end endmodule\n");
    let v = format!("module t; {var_decl} initial begin {reads} $finish; end endmodule\n");
    let (po, pe, pc) = run(&p);
    let (vo, _, vc) = run(&v);
    assert_eq!(pc, 0, "param form must elaborate clean:\n{pe}");
    assert_eq!(vc, 0, "var twin must elaborate clean");
    assert_eq!(po, vo, "param form ≠ var twin (byte-identity broken)");
}

#[test]
fn int_array_reads_match_var_twin() {
    assert_twin(
        "$display(\"%0d %0d %0d\", RHO[0], RHO[4], $size(RHO));\n\
         for (int i = 0; i < 5; i++) $display(\"%0d\", RHO[i]);",
        "localparam int RHO [0:4] = '{9, 8, 7, 6, 5};",
        "int RHO [0:4] = '{9, 8, 7, 6, 5};",
    );
}

#[test]
fn typed_variants_match_var_twin() {
    // logic vector element (explicit packed range rides through the desugar)
    assert_twin(
        "$display(\"%h %h\", T[0], T[3]);",
        "localparam logic [7:0] T [0:3] = '{8'h20, 8'h40, 8'h80, 8'hff};",
        "logic [7:0] T [0:3] = '{8'h20, 8'h40, 8'h80, 8'hff};",
    );
    // byte atom: kind-implied width AND signedness (negative values print signed)
    assert_twin(
        "$display(\"%0d %0d\", B[0], B[1]);",
        "localparam byte B [0:1] = '{-1, -2};",
        "byte B [0:1] = '{-1, -2};",
    );
    // 4-state integer + SV size-form dim + an init referencing an earlier param
    assert_twin(
        "$display(\"%0d %0d %0d\", A[0], A[1], A[2]);",
        "localparam int N = 5; localparam integer A [3] = '{N, N+1, N*2};",
        "localparam int N = 5; integer A [3] = '{N, N+1, N*2};",
    );
    // real elements
    assert_twin(
        "$display(\"%g %g\", R[0], R[1]);",
        "localparam real R [0:1] = '{1.5, 2.5};",
        "real R [0:1] = '{1.5, 2.5};",
    );
    // multi-dim
    assert_twin(
        "$display(\"%0d %0d\", M[1][2], M[0][0]);",
        "localparam int M [0:1][0:2] = '{'{1,2,3},'{4,5,6}};",
        "int M [0:1][0:2] = '{'{1,2,3},'{4,5,6}};",
    );
}

#[test]
fn unsigned_qualifier_matches_var_twin() {
    // `int unsigned` must come out unsigned (the explicit qualifier wins over
    // the atom default — the desugar mirrors `signed_eff`, not the scalar
    // param path's folded bool).
    assert_twin(
        "$display(\"%0d\", U[0]);",
        "localparam int unsigned U [0:0] = '{-1};",
        "int unsigned U [0:0] = '{-1};",
    );
}

// ───────────────────────── write paths stay loud ─────────────────────────

fn assert_loud(src: &str, needle: &str, what: &str) {
    let (_, err, code) = run(src);
    assert_ne!(code, 0, "{what}: must exit non-zero\n{err}");
    assert!(err.contains(needle), "{what}: missing `{needle}`:\n{err}");
}

#[test]
fn writes_are_loud() {
    const DENY: &str = "cannot assign to parameter";
    let decl = "localparam int R [0:1] = '{1, 2};";
    for (frag, what) in [
        ("initial R[0] = 9;", "element blocking assign"),
        ("initial R[0] <= 9;", "element nonblocking assign"),
        ("initial R = '{5, 6};", "whole-array assignment pattern"),
        (
            "int s [0:1] = '{5, 6}; initial R = s;",
            "whole-array copy write",
        ),
        (
            "initial begin for (int i = 0; i < 2; i++) R[i] = 0; end",
            "loop element write",
        ),
        ("initial force R[0] = 9;", "force"),
        ("assign R[0] = 5;", "continuous assign"),
    ] {
        assert_loud(&format!("module t; {decl} {frag} endmodule\n"), DENY, what);
    }
    // $readmemh writes the memory
    assert_loud(
        &format!("module t; {decl} initial $readmemh(\"nofile.hex\", R); endmodule\n"),
        "cannot $readmem into parameter",
        "$readmemh",
    );
    // hierarchical writes resolve through the DEFERRED sentinel lanes, not the
    // lower_lvalue funnel — both patch-up passes must consult the const set
    // (adversarial find: `u1.RHO[0] = 9` was a silent mutation).
    for (frag, what) in [
        ("u1.R[0] = 9;", "hier element write"),
        ("u1.R[1] <= 9;", "hier NBA element write"),
        ("u1.R[0] += 5;", "hier compound element write"),
    ] {
        assert_loud(
            &format!(
                "module leaf; localparam int R [0:1] = '{{1, 2}}; endmodule\n\
                 module t; leaf u1(); initial begin #1; {frag} end endmodule\n"
            ),
            DENY,
            what,
        );
    }
    // hier part-select of an element
    assert_loud(
        "module leaf; localparam logic [7:0] T [0:1] = '{8'ha5, 8'h5a}; endmodule\n\
         module t; leaf u1(); initial begin #1; u1.T[0][3:0] = 4'hF; end endmodule\n",
        DENY,
        "hier part-select write",
    );
    // hier read stays legal (post-t0 — the t0 cross-module init race is a
    // separate pre-existing decl-init ordering item, twin-identical)
    let (hout, _, hcode) = run(
        "module leaf; localparam int R [0:1] = '{11, 22}; endmodule\n\
         module t; leaf u1(); initial begin #1; $display(\"%0d\", u1.R[1]); $finish; end endmodule\n",
    );
    assert_eq!(hcode, 0);
    assert!(hout.contains("22"), "hier read must pass:\n{hout}");
    // SYS-READ hier-element dest: the deferred-read placeholder hid the net
    // from the deny AND the write lane silently drops such writes even for
    // vars (adversarial find) — loud for both now; whole hier scalar dests
    // keep working.
    assert_loud(
        "module unit; localparam int R [0:1] = '{1, 2}; endmodule\n\
         module t; unit u1(); int rc;\n\
         initial rc = $sscanf(\"42\", \"%d\", u1.R[0]); endmodule\n",
        "hierarchical element select",
        "sscanf hier element param dest",
    );
    assert_loud(
        "module unit; int v [0:1] = '{1, 2}; endmodule\n\
         module t; unit u1(); int rc;\n\
         initial rc = $sscanf(\"42\", \"%d\", u1.v[0]); endmodule\n",
        "hierarchical element select",
        "sscanf hier element var dest (write silently vanished before)",
    );
    let (so, _, sc) = run("module unit; int s; endmodule\n\
         module t; unit u1(); int rc;\n\
         initial begin rc = $sscanf(\"42\", \"%d\", u1.s); #1 $display(\"s=%0d\", u1.s); $finish; end endmodule\n");
    assert_eq!(sc, 0);
    assert!(
        so.contains("s=42"),
        "whole hier scalar dest must keep working:\n{so}"
    );
    // non-lvalue engine-side write lanes (adversarial finds): the assoc
    // iteration key and a clocking-output drive both write a bare Signal.
    assert_loud(
        &format!(
            "module t; {decl} int aa [int]; int st;\n\
             initial begin aa[42] = 7; st = aa.first(R); end endmodule\n"
        ),
        "cannot write the iteration key into parameter",
        "assoc first(key) write",
    );
    assert_loud(
        &format!(
            "module t; {decl} logic clk = 0;\n\
             clocking cb @(posedge clk); output R; endclocking\n\
             initial begin cb.R <= 99; #10 $finish; end\n\
             always #2 clk = ~clk; endmodule\n"
        ),
        "cannot drive (clocking output) parameter",
        "clocking output drive",
    );
    // reads stay fine: copying OUT of the parameter is legal
    let (out, _, code) = run(&format!(
        "module t; {decl} int s [0:1];\n\
         initial begin s = R; $display(\"%0d\", s[1]); $finish; end endmodule\n"
    ));
    assert_eq!(code, 0);
    assert!(out.contains('2'), "read-side copy must pass:\n{out}");
}

// ───────────────────────── v1 loud boundaries ─────────────────────────

#[test]
fn v1_boundaries_are_loud() {
    // ANSI header form — no override machinery for aggregates
    assert_loud(
        "module t #(parameter int P [0:1] = '{1,2}); endmodule\n",
        "header",
        "ANSI header array parameter",
    );
    // overridable body `parameter`
    assert_loud(
        "module t; parameter int P [0:1] = '{1,2}; endmodule\n",
        "localparam` for an array parameter",
        "body `parameter` kind",
    );
    // implicit-typed element
    assert_loud(
        "module t; localparam L [0:1] = '{1,2}; endmodule\n",
        "explicit data type",
        "implicit-typed array parameter",
    );
    // non-fixed dim
    assert_loud(
        "module t; localparam int Q [$] = '{1,2}; endmodule\n",
        "fixed array-parameter dimension",
        "queue dim",
    );
    // generate scope: the §6.8 pre-sweep does not collect generate decl-inits
    // (a desugared parameter would silently stay 0) — loud scope-gate.
    assert_loud(
        "module t; generate if (1) begin : g\n\
         localparam int L [0:1] = '{30, 40};\n\
         end endgenerate endmodule\n",
        "generate block",
        "generate-scope array parameter",
    );
    // package scope: stays the package-body v7 reject until A2b-prereq
    assert_loud(
        "package p; localparam int R [0:1] = '{1,2}; endpackage\n\
         module t; import p::*; initial $display(\"%0d\", R[0]); endmodule\n",
        "package body",
        "package array parameter",
    );
    // interface scope: no §6.8 decl-init collection pass there (the generate
    // gate's twin — adversarial find: the value silently read 0)
    assert_loud(
        "interface ifc; localparam int R [0:1] = '{1,2}; endinterface\n\
         module t; ifc i(); initial $display(\"%0d\", i.R[0]); endmodule\n",
        "interface",
        "interface-scope array parameter",
    );
    // port-name collision: the non-ANSI input copy-in would drive the net
    // through a deny-free ContAssign (adversarial find) — reject the merge
    assert_loud(
        "module child(R); input R; localparam int R [0:1] = '{1,2}; endmodule\n\
         module t; logic x = 0; child u(x); endmodule\n",
        "cannot be a port",
        "port/param name collision",
    );
    // scalar params are untouched by the desugar (regression canary)
    let (out, _, code) = run(
        "module t; localparam int A = 1, B = 2; initial $display(\"%0d %0d\", A, B); endmodule\n",
    );
    assert_ne!(
        code, 0,
        "comma-list scalar param stays loud (pre-existing single-name shape)"
    );
    let (out2, _, code2) =
        run("module t; localparam int A = 7; initial begin $display(\"%0d\", A); $finish; end endmodule\n");
    assert_eq!(code2, 0);
    assert!(out2.contains('7'), "scalar localparam regression:\n{out2}");
    let _ = out;
}
