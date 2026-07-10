//! Round-4 RESIDUAL silent-wrongs (pre-existing, surfaced by the round-4
//! adversarial review) — all fixed correct-or-loud. Two documented residuals
//! plus two adjacent silent-wrongs found while verifying the fixes:
//!
//! - **Issue 1 — concat pkg-operand 0-drop**: a package const read (`p::x` or a
//!   bare `import`ed `x`) materialized at the value-inferred 32 bits instead of
//!   its DECLARED width. As the LOW operand of a concat that made the concat 32+
//!   bits wide, shoving the high operand out of the assigned target
//!   (`{4'h5, p::x}` → `0a`, not `5a`). Fixed: `pkg_const_meta` records each
//!   package param's `(width, signed)`; the `PkgScoped` read and the import bind
//!   use `const_param_expr_w` / seed `param_meta`, so the const carries its true
//!   self-width (the local-param path already did this).
//! - **Issue 2 — replication const-array-count 0-width**: a replication count
//!   that reads an unpacked-array element (`{CNT[0]{…}}`) is not a runtime net
//!   the engine can fold → it read 0 → a 0-width result. Now correct-or-loud: a
//!   GAP-G-capturable element (zero-based ascending array — module / generate /
//!   package, directly / in an arithmetic wrapper `{CNT[0]+1{…}}` / inside
//!   `$clog2(CNT[i])`) FOLDS to a literal count; a shape GAP-G cannot fold
//!   (descending / non-zero-based / multi-dimensional), a RUNTIME array element,
//!   or an out-of-range / negative index is LOUD (never the old silent 0-width),
//!   mirroring the loud `localparam R = ROT[i]` binding site and matching
//!   iverilog's rejection of a non-constant / negative count.
//! - **Adjacent — enum-label width** (module AND package): an enum label read at
//!   32 bits, not its base-type width, so `{4'h5, STATE}` dropped the high
//!   operand exactly like Issue 1. Fixed: `enum_base_meta` seeds `param_meta` /
//!   `pkg_const_meta` for labels too.
//!
//! Oracle = iverilog 13.0 where it can run. iverilog cannot elaborate unpacked
//! ARRAY parameters ("sorry"), so the replication-count values are pinned to a
//! hand-computed / scalar-twin oracle (a scalar `localparam C = 2` with the same
//! `{C{4'b01}}` → `0x11`, which iverilog DOES run).
//!
//! Elaborate-only, no AST field, `.vu`/format_version 19 unchanged, IR-0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (first `y=`/`o=` line value, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r4res_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("y=") || l.starts_with("o="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

// ─────────────────── Issue 1: concat pkg-operand width ───────────────────

#[test]
fn concat_pkg_param_lsb_wildcard_import() {
    // `{4'h5, x}` with `x` a bare-imported 4-bit package localparam. Before the
    // fix the 32-bit const shoved `4'h5` past the 8-bit target ⇒ `0a`.
    let (o, ok) = run("package p; localparam logic [3:0] x = 4'hA; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = {4'h5, x}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=5a", "got: {o}");
}

#[test]
fn concat_pkg_param_lsb_explicit_scoped() {
    // Same via explicit `p::x` (the `PkgScoped` read arm).
    let (o, ok) = run("package p; localparam logic [3:0] x = 4'hA; endpackage\n\
         module top; logic [7:0] y;\n\
         initial begin y = {4'h5, p::x}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=5a", "got: {o}");
}

#[test]
fn concat_pkg_param_both_operands() {
    // `{x, x}` into a 16-bit target: two 4-bit consts ⇒ 8-bit `00aa`; a 32-bit
    // const would collapse to `000a`.
    let (o, ok) = run("package p; localparam logic [3:0] x = 4'hA; endpackage\n\
         module top; import p::*; logic [15:0] y;\n\
         initial begin y = {x, x}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=00aa", "got: {o}");
}

#[test]
fn concat_pkg_param_msb_is_byte_identical() {
    // `{x, 4'h5}` — pkg operand as the HIGH bits. This already printed `a5`
    // before the fix (the extra zero-extension bits land above the target), so
    // it must stay `a5` — a regression guard on the MSB path.
    let (o, ok) = run("package p; localparam logic [3:0] x = 4'hA; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = {x, 4'h5}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=a5", "got: {o}");
}

#[test]
fn concat_pkg_signed_param() {
    // A signed 4-bit package param (-1 = 4'hF) as the low concat operand.
    let (o, ok) = run(
        "package p; localparam logic signed [3:0] x = -1; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = {4'h3, x}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=3f", "got: {o}");
}

#[test]
fn pkg_param_full_width_read_unaffected() {
    // Byte-identity guard: a width-INSENSITIVE read (whole assign) is unchanged
    // by the width fix — an 8-bit pkg param assigned to an 8-bit reg is `ab`.
    let (o, ok) = run("package p; localparam logic [7:0] w = 8'hAB; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = w; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=ab", "got: {o}");
}

// ─────────────── Issue 2: replication const-array-count ───────────────
// Oracle: iverilog cannot run unpacked ARRAY params; a scalar twin
// `localparam int C0 = 2; {C0{4'b01}}` runs on iverilog and gives 0x11.

#[test]
fn repl_module_const_array_count() {
    // `{CNT[0]{4'b01}}`, CNT[0]=2 ⇒ {2{4'b0001}} = 8'h11. Before the fix the
    // element read folded to 0 ⇒ 0-width ⇒ `00`.
    let (o, ok) = run(
        "module top; localparam int CNT[0:1] = '{2, 3}; logic [7:0] y;\n\
         initial begin y = {CNT[0]{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=11", "got: {o}");
}

#[test]
fn repl_pkg_const_array_count_wildcard() {
    // Same count via a bare-imported package const array.
    let (o, ok) = run("package p; localparam int CNT[0:1] = '{2, 3}; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = {CNT[0]{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=11", "got: {o}");
}

#[test]
fn repl_pkg_const_array_count_explicit() {
    // Explicit `p::CNT[0]` count.
    let (o, ok) = run("package p; localparam int CNT[0:1] = '{2, 3}; endpackage\n\
         module top; logic [7:0] y;\n\
         initial begin y = {p::CNT[0]{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=11", "got: {o}");
}

#[test]
fn repl_const_array_count_in_arith_wrapper() {
    // `{CNT[0]+1{4'b01}}` = {3{4'b0001}} into a 12-bit target = 12'h111. The
    // element read is nested inside an `Add`, so the fix must recurse.
    let (o, ok) = run(
        "module top; localparam int CNT[0:1] = '{2, 3}; logic [11:0] y;\n\
         initial begin y = {CNT[0]+1{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=111", "got: {o}");
}

#[test]
fn repl_clog2_of_const_array_element_folds() {
    // `{$clog2(CNT[1]){4'b01}}`, CNT[1]=3 ⇒ $clog2(3)=2 ⇒ {2{4'b0001}} = 8'h11.
    // The element read is inside a `$clog2` system-call arg — the gate recurses it.
    let (o, ok) = run(
        "module top; localparam int CNT[0:1] = '{2, 3}; logic [7:0] y;\n\
         initial begin y = {$clog2(CNT[1]){4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=11", "got: {o}");
}

#[test]
fn repl_descending_array_count_is_loud() {
    // A DESCENDING const array element as a count — GAP-G cannot fold the shape,
    // so it is LOUD (never the old silent 0-width). CNT[3]=8 (a valid count), so
    // silent 0 would be a wrong value, not benign.
    let (_o, ok) = run(
        "module top; localparam int CNT[3:0] = '{8, 4, 2, 1}; logic [31:0] y;\n\
         initial begin y = {CNT[3]{4'b0001}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(!ok, "a descending-array replication count must be loud");
}

#[test]
fn repl_multidim_array_count_is_loud() {
    let (_o, ok) = run(
        "module top; localparam int CNT[0:1][0:1] = '{'{2, 3}, '{4, 5}}; logic [31:0] y;\n\
         initial begin y = {CNT[1][0]{4'b0001}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(
        !ok,
        "a multi-dimensional-array replication count must be loud"
    );
}

#[test]
fn repl_out_of_range_index_count_is_loud() {
    let (_o, ok) = run(
        "module top; localparam int CNT[0:1] = '{2, 3}; logic [7:0] y;\n\
         initial begin y = {CNT[9]{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(!ok, "an out-of-range const-array index count must be loud");
}

#[test]
fn repl_negative_count_is_loud() {
    // Matches iverilog "Concatenation repeat may not be negative".
    let (_o, ok) = run(
        "module top; localparam int CNT[0:1] = '{-1, 2}; logic [7:0] y;\n\
         initial begin y = {CNT[0]{4'b1}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(!ok, "a negative replication count must be loud");
}

#[test]
fn repl_runtime_array_count_is_loud() {
    // A runtime (variable) array element is not a constant — an illegal count.
    // iverilog: "not allowed in a constant expression"; vita: loud, not silent 0.
    let (_o, ok) = run(
        "module top; logic [7:0] mem[0:1]; logic [7:0] y;\n\
         initial begin mem[0] = 2; y = {mem[0]{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(!ok, "a runtime-array replication count must be loud");
}

#[test]
fn repl_scalar_count_byte_identical() {
    // A scalar-param count `{C0{4'b01}}` (NO const-array element) must keep its
    // existing lowering (the fix is gated on a const-array element read). Runs
    // on iverilog too ⇒ 0x11.
    let (o, ok) = run("module top; localparam int C0 = 2; logic [7:0] y;\n\
         initial begin y = {C0{4'b01}}; $display(\"y=%h\", y); #1 $finish; end endmodule");
    assert!(ok && o == "y=11", "got: {o}");
}

// ─────────────────── Adjacent: enum-label width ───────────────────

#[test]
fn concat_pkg_enum_label_width() {
    // A package enum label `A` (`enum logic [3:0] {A=4'hA}`) as the low concat
    // operand ⇒ `{4'h5, A}` = 8'h5A. Before the fix the label read at 32 bits.
    let (o, ok) = run(
        "package p; typedef enum logic [3:0] {A=4'hA, B=4'hB} e; endpackage\n\
         module top; import p::*; logic [7:0] y;\n\
         initial begin y = {4'h5, A}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=5a", "got: {o}");
}

#[test]
fn concat_module_enum_label_width() {
    // The module-local twin (same pre-existing bug, same fix).
    let (o, ok) = run(
        "module top; typedef enum logic [3:0] {A=4'hA, B=4'hB} e; logic [7:0] y;\n\
         initial begin y = {4'h5, A}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=5a", "got: {o}");
}

#[test]
fn enum_label_comparison_unaffected() {
    // Byte-identity guard: a width-insensitive label use (equality) is unchanged.
    let (o, ok) = run(
        "module top; typedef enum logic [3:0] {A=4'hA, B=4'hB} e; e s; logic [7:0] y;\n\
         initial begin s = B; y = (s == B) ? 8'h01 : 8'h00; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=01", "got: {o}");
}

#[test]
fn signed_enum_label_negative_arithmetic() {
    // A NEGATIVE label of a signed-base enum keeps its SIGN in arithmetic. The
    // parser drops an `enum logic signed [N]` base's signedness, so signedness is
    // inferred per-value (`A=-2 < 0` ⇒ signed); narrowing only the width would
    // otherwise flip -2 to the unsigned 14 ⇒ `A + 1 = 15` instead of -1.
    let (o, ok) = run(
        "module top; typedef enum logic signed [3:0] {A=-2, B=1} e; logic signed [7:0] y;\n\
         initial begin y = A + 8'sd1; $display(\"y=%0d\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=-1", "got: {o}");
}

#[test]
fn signed_enum_label_negative_concat() {
    // The same negative label in a concat uses its 4-bit two's-complement (0xE),
    // not a 32-bit sign-extended blob: `{4'h5, A}` = 8'h5E.
    let (o, ok) = run(
        "module top; typedef enum logic signed [3:0] {A=-2, B=1} e; logic [7:0] y;\n\
         initial begin y = {4'h5, A}; $display(\"y=%h\", y); #1 $finish; end endmodule",
    );
    assert!(ok && o == "y=5e", "got: {o}");
}
