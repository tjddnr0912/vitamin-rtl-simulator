//! Generate- and interface-scope variable decl-init pre-sweep (IEEE 1800 §6.8).
//!
//! The §6.8 pre-sweep that turns an unpacked-array `'{…}` / non-constant scalar
//! variable initializer into a t0 `initial` block used to walk the MODULE body
//! ONLY. A generate block or interface body decl-init was therefore silently
//! DROPPED — the array/non-const value stayed at its X/0 default while iverilog
//! printed the initialized value (a live-oracle silent-wrong). This test pins
//! the fix: a new `GenPhase::VarInit` walk (one flush per generate scope) + an
//! interface-body collect+flush, both in-scope so the bare-name lvalue resolves
//! to the scoped net. (Queue / dyn-array / string decl-inits in these scopes
//! stay a LOUD reject — a documented follow-on — see
//! `queue_pattern_in_generate_stays_loud`.)
//!
//! Oracles:
//! - CONSTANT-element arrays and self-contained non-const inits are live
//!   iverilog-pinned (iverilog fully supports array VARIABLE `'{…}` inits in
//!   generate/interface).
//! - The desugared array PARAMETER form has no live oracle (iverilog: "sorry:
//!   unpacked array parameters are not supported yet") → its teeth is the
//!   vita-internal twin: `localparam <ty> T[..]='{…}` ≡ the equivalent VARIABLE
//!   `<ty> T[..]='{…}` (byte-identical stdout), and the var twin IS iverilog-live.
//!
//! Known race (documented, NOT a regression from this fix): a generate/interface
//! decl-init that READS a module-scope non-constant variable observes a value
//! set by vita's module-first init order, whereas iverilog runs the generate
//! init first (module var still at its default). Both orderings are legal under
//! §6.8 (variable-initialization order across scopes is unspecified). See
//! `cross_scope_module_read_is_a_documented_init_order_race`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_geniface_{}_{n}", std::process::id()));
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

fn assert_ok_stdout(src: &str, want_prefix: &str) {
    let (o, e, c) = run(src);
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert!(
        o.starts_with(want_prefix),
        "stdout mismatch:\n  want prefix: {want_prefix:?}\n  got: {o:?}"
    );
}

// ─────────────── constant-element arrays (iverilog-pinned) ───────────────

#[test]
fn gen_for_const_array() {
    assert_ok_stdout(
        "module top; genvar gi;\n\
         generate for (gi=0; gi<1; gi=gi+1) begin : blk\n\
           int arr [0:2] = '{10, 20, 30};\n\
           initial $display(\"%0d %0d %0d\", arr[0], arr[1], arr[2]);\n\
         end endgenerate\n\
         initial #1 $finish; endmodule\n",
        "10 20 30\n",
    );
}

#[test]
fn gen_if_const_array() {
    assert_ok_stdout(
        "module top; generate if (1) begin : g\n\
           int arr [0:2] = '{4, 5, 6};\n\
           initial $display(\"%0d %0d %0d\", arr[0], arr[1], arr[2]);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "4 5 6\n",
    );
}

#[test]
fn gen_case_const_array() {
    assert_ok_stdout(
        "module top; parameter SEL=1;\n\
         generate case (SEL)\n\
           1: begin : g int a[0:1]='{55,66}; initial $display(\"%0d %0d\", a[0], a[1]); end\n\
           default: begin : g2 int a[0:1]='{0,0}; initial $display(\"nope\"); end\n\
         endcase endgenerate initial #1 $finish; endmodule\n",
        "55 66\n",
    );
}

#[test]
fn iface_const_array() {
    assert_ok_stdout(
        "interface iface; int arr [0:2] = '{7, 8, 9}; endinterface\n\
         module top; iface u();\n\
         initial begin #1; $display(\"%0d %0d %0d\", u.arr[0], u.arr[1], u.arr[2]); $finish; end\n\
         endmodule\n",
        "7 8 9\n",
    );
}

#[test]
fn nested_generate_array() {
    // for inside if — the pre-sweep flushes at the innermost scope.
    assert_ok_stdout(
        "module top; genvar gi;\n\
         generate if (1) begin : outer\n\
           for (gi=0; gi<1; gi=gi+1) begin : inner\n\
             int arr [0:1] = '{77, 88};\n\
             initial $display(\"%0d %0d\", arr[0], arr[1]);\n\
           end\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "77 88\n",
    );
}

#[test]
fn sibling_gen_for_iterations_get_independent_flushes() {
    // Each unrolled iteration is a distinct scope (`b[0]`, `b[1]`); the element
    // exprs read the genvar, so the two blocks must NOT collide or share a flush.
    assert_ok_stdout(
        "module top; genvar gi;\n\
         generate for (gi=0; gi<2; gi=gi+1) begin : b\n\
           int a[0:1] = '{gi*10, gi*10+1};\n\
           initial $display(\"iter %0d: %0d %0d\", gi, a[0], a[1]);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "iter 0: 0 1\niter 1: 10 11\n",
    );
}

#[test]
fn xz_multibit_logic_array() {
    // 4-state fill must survive (z/x elements, not zero-extended to 0).
    assert_ok_stdout(
        "module top; generate if(1) begin: g\n\
           logic [7:0] a[0:2] = '{8'hZ, 8'hX, 8'd5};\n\
           initial $display(\"%h %h %h\", a[0], a[1], a[2]);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "zz xx 05\n",
    );
}

#[test]
fn two_dim_array() {
    assert_ok_stdout(
        "module top; generate if(1) begin: g\n\
           int m[0:1][0:1] = '{'{1,2},'{3,4}};\n\
           initial $display(\"%0d %0d %0d %0d\", m[0][0], m[0][1], m[1][0], m[1][1]);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "1 2 3 4\n",
    );
}

#[test]
fn nonconst_scalar_same_scope() {
    // A non-const scalar init reading an EARLIER same-scope var: iverilog and
    // vita agree (b sees a=5 → 6) — same-scope decl order is well defined.
    assert_ok_stdout(
        "module top; generate if(1) begin: g\n\
           int a = 5; int b = a + 1;\n\
           initial $display(\"a=%0d b=%0d\", a, b);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "a=5 b=6\n",
    );
}

#[test]
fn const_scalar_in_generate_still_folds() {
    // Regression canary: a CONSTANT scalar init was never dropped (it folds into
    // net.init, scope-agnostic). Must stay working after the VarInit pass.
    assert_ok_stdout(
        "module top; generate if(1) begin: g\n\
           int s = 42; initial $display(\"%0d\", s);\n\
         end endgenerate initial #1 $finish; endmodule\n",
        "42\n",
    );
}

#[test]
fn module_scope_array_init_unchanged() {
    // The module-body sweep must be byte-identical to before (no double-emit,
    // no reordering) — a plain module array init.
    assert_ok_stdout(
        "module top; int arr [0:2] = '{1, 2, 3};\n\
         initial begin $display(\"%0d %0d %0d\", arr[0], arr[1], arr[2]); $finish; end\n\
         endmodule\n",
        "1 2 3\n",
    );
}

// ─────────── array PARAMETER twin (localparam ≡ var; var is live) ───────────

fn assert_gen_twin(reads: &str, ty_dims_init: &str) {
    // `localparam <T>` must be byte-identical (stdout) to plain `<T>` (a var)
    // in the same generate scope.
    let p = format!(
        "module top; generate if(1) begin: g\n localparam {ty_dims_init}\n initial begin {reads} $finish; end\n end endgenerate initial #2 $finish; endmodule\n"
    );
    let v = format!(
        "module top; generate if(1) begin: g\n {ty_dims_init}\n initial begin {reads} $finish; end\n end endgenerate initial #2 $finish; endmodule\n"
    );
    let (po, pe, pc) = run(&p);
    let (vo, _, vc) = run(&v);
    assert_eq!(pc, 0, "param form must elaborate clean:\n{pe}");
    assert_eq!(vc, 0, "var twin must elaborate clean");
    assert_eq!(
        po, vo,
        "generate localparam form ≠ var twin (byte-identity)"
    );
}

#[test]
fn localparam_array_in_generate_matches_var_twin() {
    assert_gen_twin(
        "$display(\"%0d %0d %0d\", T[0], T[1], T[2]);",
        "int T [0:2] = '{5, 6, 7};",
    );
    assert_gen_twin(
        "$display(\"%h %h\", W[0], W[1]);",
        "logic [15:0] W [0:1] = '{16'hBEEF, 16'hCAFE};",
    );
}

#[test]
fn localparam_array_in_interface() {
    // No var-twin harness needed — a plain iverilog-live value read.
    assert_ok_stdout(
        "interface iface; localparam int T [0:2] = '{5,6,7}; endinterface\n\
         module top; iface u();\n\
         initial begin #1; $display(\"%0d %0d %0d\", u.T[0], u.T[1], u.T[2]); $finish; end\n\
         endmodule\n",
        "5 6 7\n",
    );
}

// ─────────────── write-deny survives the lifted scope-gate ───────────────

#[test]
fn write_to_localparam_array_in_generate_is_loud() {
    // The A2a net-id-keyed const-param deny still fires for a USER write, even
    // though the decl-init's own write is exempt (`lowering_decl_init`).
    for stmt in ["a[0] = 9;", "a = '{4,5};", "$readmemh(\"nope.hex\", a);"] {
        let (_, e, c) = run(&format!(
            "module top; generate if(1) begin: g\n\
               localparam int a [0:1] = '{{1,2}};\n\
               initial begin {stmt} $finish; end\n\
             end endgenerate initial #1 $finish; endmodule\n"
        ));
        assert_ne!(c, 0, "`{stmt}` must be loud");
        assert!(
            e.contains("parameter") && (e.contains("cannot") || e.contains("constant")),
            "`{stmt}` wrong diagnostic:\n{e}"
        );
    }
}

#[test]
fn write_to_localparam_array_in_interface_is_loud() {
    let (_, e, c) = run(
        "interface iface; localparam int a [0:1] = '{1,2}; endinterface\n\
         module top; iface u();\n\
         initial begin #1; u.a[0] = 9; $finish; end endmodule\n",
    );
    assert_ne!(
        c, 0,
        "hier write to interface localparam array must be loud"
    );
    assert!(
        e.contains("parameter") && (e.contains("cannot") || e.contains("constant")),
        "wrong diagnostic:\n{e}"
    );
}

// ─────────────────────── documented boundaries ───────────────────────

#[test]
fn cross_scope_module_read_is_a_documented_init_order_race() {
    // A generate init reading a MODULE non-constant var: vita runs the module
    // pre-sweep first (module `k` = 9 applied), so the generate init reads 9;
    // iverilog runs the generate init first and reads k's default (0). Both are
    // legal §6.8 orderings. Pin vita's DETERMINISTIC value so a semantic change
    // is caught (this is NOT oracle-matched — it is a documented race, ㉯ family).
    let (o, e, c) = run("module top; int k = 9;\n\
         generate if(1) begin: g\n\
           int a[0:1] = '{k, k+1};\n\
           initial $display(\"%0d %0d\", a[0], a[1]);\n\
         end endgenerate initial #1 $finish; endmodule\n");
    assert_eq!(c, 0, "must elaborate clean:\n{e}");
    assert!(o.starts_with("9 10\n"), "vita init-order value:\n{o}");
}

#[test]
fn interface_flush_does_not_steal_module_block_local_init() {
    // Adversarial soundness find (fixed): the interface §6.8 pre-sweep runs
    // during the parent's Nets phase, where `hoist_block_local_nets` may already
    // have queued a module block-local non-const init in the shared
    // `pending_var_inits`. Without save/restore the interface flush STOLE that
    // init and re-lowered it in the interface scope — a loud misresolve, and
    // (with same-named interface members) a SILENT module-side drop. Both forms
    // must now match iverilog.
    // (a) loud-misresolve form: interface has NO clashing member.
    assert_ok_stdout(
        "interface if_t; logic x; endinterface\n\
         module top; int src = 0; if_t u();\n\
         initial begin int bl = src + 1; $display(\"bl=%0d\", bl); $finish; end\n\
         endmodule\n",
        "bl=1\n",
    );
    // (b) silent-drop form: interface member names collide with the block-local
    // (`bl`) and its source (`src`). Module `bl` must read module `src`=100 → 101,
    // NOT the interface `src`=0 → 1.
    assert_ok_stdout(
        "interface if_t; int bl; int src; endinterface\n\
         module top; int src = 100; if_t u();\n\
         initial begin int bl = src + 1; $display(\"module bl=%0d\", bl); $finish; end\n\
         endmodule\n",
        "module bl=101\n",
    );
}

#[test]
fn queue_and_dyn_pattern_in_generate_are_supported() {
    // The follow-on this file recorded as "loud = safe". A dynamic-handle decl-init in a
    // generate scope was rejected at net creation via `allow_string_init = false`; that
    // flag was standing in for the real defect, which is that a string/handle declaration's
    // decl-time writes went into the MODULE-scope pending list and resolved their bare-name
    // lvalue outside the generate prefix. Scope-keyed now. iverilog: `1 2 3 sz=3`.
    let (o, _, c) = run("module top; generate if(1) begin: g\n\
           int q[$] = '{1,2,3};\n\
           initial $display(\"%0d %0d %0d sz=%0d\", q[0], q[1], q[2], q.size());\n\
         end endgenerate initial #1 $finish; endmodule\n");
    assert_eq!(c, 0, "{o}");
    assert!(o.contains("1 2 3 sz=3"), "{o}");

    // …and the dynamic-array / string-queue twins, which shared the same block.
    let (o2, _, c2) = run("module top; generate if(1) begin: g\n\
           int d[] = '{7,8}; string sq[$] = '{\"p\",\"q\"};\n\
           initial $display(\"%0d %0d | %s %s\", d[0], d[1], sq[0], sq[1]);\n\
         end endgenerate initial #1 $finish; endmodule\n");
    assert_eq!(c2, 0, "{o2}");
    assert!(o2.contains("7 8 | p q"), "{o2}");
}
