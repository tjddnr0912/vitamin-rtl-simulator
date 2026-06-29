//! SYS-INTRO array-query / dimension introspection const-folds (Medium bundle
//! rank 2): $size/$left/$right/$low/$high/$increment/$dimensions/
//! $unpacked_dimensions/$isunbounded. Pure IR-0 elaborate const-folds (the
//! argument is a TYPE/net reference, not evaluated). Previously these system
//! FUNCTIONS were E3009-LOUD, which killed the WHOLE design — so folding them is
//! both a feature and a robustness fix.
//!
//! Oracle: iverilog 13.0 (every value verified live). $typename is DEFERRED
//! (stays loud); iverilog rejects $isunbounded/$typename so those are hand-IEEE.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(body: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_si_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, body).unwrap();
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

fn disp(decls: &str, fmt: &str, args: &str) -> String {
    run(&format!(
        "module t;\n{decls}\ninitial $display(\"R {fmt}\", {args});\nendmodule\n"
    ))
    .0
}

#[test]
fn packed_vector_descending() {
    // reg [7:0] v: bits=8 size=8 left=7 right=0 low=0 high=7 incr=1 dims=1.
    let out = disp(
        "reg [7:0] v;",
        "%0d %0d %0d %0d %0d %0d %0d %0d",
        "$bits(v),$size(v),$left(v),$right(v),$low(v),$high(v),$increment(v),$dimensions(v)",
    );
    assert!(out.contains("R 8 8 7 0 0 7 1 1"), "{out}");
}

#[test]
fn packed_vector_ascending_increment_is_negative() {
    // reg [0:7] a: ascending => left=0 right=7 low=0 high=7 increment=-1.
    let out = disp(
        "reg [0:7] a;",
        "%0d %0d %0d %0d %0d %0d",
        "$size(a),$left(a),$right(a),$low(a),$high(a),$increment(a)",
    );
    assert!(out.contains("R 8 0 7 0 7 -1"), "{out}");
}

#[test]
fn unpacked_array_dimensions() {
    // reg [3:0] mem [0:7]: dim1 = unpacked [0:7], dim2 = packed [3:0].
    let out = disp(
        "reg [3:0] mem [0:7];",
        "%0d %0d %0d %0d %0d %0d",
        "$size(mem),$left(mem),$right(mem),$dimensions(mem),$unpacked_dimensions(mem),$size(mem,2)",
    );
    assert!(out.contains("R 8 0 7 2 1 4"), "{out}");
}

#[test]
fn multidim_array_dimension_order() {
    // reg [7:0] g [0:3][0:1]: dim1=[0:3]=4, dim2=[0:1]=2, dim3=packed[7:0]=8.
    let out = disp(
        "reg [7:0] g [0:3][0:1];",
        "%0d %0d %0d %0d %0d",
        "$dimensions(g),$unpacked_dimensions(g),$size(g,1),$size(g,2),$size(g,3)",
    );
    assert!(out.contains("R 3 2 4 2 8"), "{out}");
}

#[test]
fn scalar_has_zero_dimensions_and_x_query() {
    // A true scalar `reg s` has 0 dimensions; a dim query (e.g. $size) => X.
    let out = disp(
        "reg s;",
        "%0d %0d %0d %0d",
        "$dimensions(s),$bits(s),$size(s),$left(s)",
    );
    assert!(out.contains("R 0 1 x x"), "{out}");
}

#[test]
fn one_bit_vector_is_distinct_from_scalar() {
    // reg [0:0] z is a 1-DIMENSION vector (dims=1, size=1) — net metadata alone
    // cannot tell it from a scalar, but the decl-time descriptor can.
    let out = disp(
        "reg [0:0] z;",
        "%0d %0d %0d",
        "$dimensions(z),$size(z),$left(z)",
    );
    assert!(out.contains("R 1 1 0"), "{out}");
}

#[test]
fn isunbounded_folds_to_zero() {
    let out = disp("reg [7:0] v;", "%0d", "$isunbounded(v)");
    assert!(out.contains("R 0"), "{out}");
}

#[test]
fn size_out_of_range_dimension_is_x() {
    let out = disp("reg [3:0] mem [0:7];", "%0d", "$size(mem,3)");
    assert!(out.contains("R x"), "{out}");
}

#[test]
fn integer_has_implicit_32bit_dimension() {
    // integer carries an implicit [31:0] packed dim (adversarial-review regression
    // fix: was folding to 0-dim X). iverilog: $size=32 $left=31 $right=0 $dims=1.
    let out = disp(
        "integer ii;",
        "%0d %0d %0d %0d",
        "$size(ii),$left(ii),$right(ii),$dimensions(ii)",
    );
    assert!(out.contains("R 32 31 0 1"), "{out}");
}

#[test]
fn time_has_implicit_64bit_dimension() {
    let out = disp(
        "time tt;",
        "%0d %0d %0d",
        "$size(tt),$left(tt),$dimensions(tt)",
    );
    assert!(out.contains("R 64 63 1"), "{out}");
}

#[test]
fn integer_array_dimensions() {
    // integer mem [0:3]: dim1 = unpacked [0:3] (4), dim2 = implicit [31:0] (32).
    let out = disp(
        "integer mem [0:3];",
        "%0d %0d %0d",
        "$dimensions(mem),$size(mem,1),$size(mem,2)",
    );
    assert!(out.contains("R 2 4 32"), "{out}");
}

#[test]
fn real_is_dimensionless() {
    // real/realtime are genuinely 0-dimensional (NOT a 64-bit vector) — iverilog
    // parity: $dimensions=0 even though $bits=64. (Guards the integer/time fix
    // from over-reaching to real.)
    let out = disp("real rr;", "%0d %0d", "$dimensions(rr),$bits(rr)");
    assert!(out.contains("R 0 64"), "{out}");
}

#[test]
fn packed_multidim_port_dimensions() {
    // ANSI packed multi-dim port (adversarial-review regression fix: was folding
    // to a single 32-bit dim). input [3:0][7:0] mp => dim1=[3:0]=4, dim2=[7:0]=8.
    let out = run("module t(input [3:0][7:0] mp);\n\
         initial $display(\"R %0d %0d %0d %0d %0d\",\
         $dimensions(mp),$size(mp,1),$size(mp,2),$left(mp,1),$left(mp,2));\n\
         endmodule\n")
    .0;
    assert!(out.contains("R 2 4 8 3 7"), "{out}");
}

// ── SYS-INTRO잔여 (non-libm): $countbits, $typename, $exit ─────────────────────
// All three are PURE IR-0 (no format_version bump): $countbits desugars to a
// per-bit case-eq sum, $typename const-folds to a packed-ASCII string from the
// net's type metadata, $exit reuses SysTaskId::Finish. iverilog 13.0 crashes on
// $countbits and rejects $typename ⇒ HAND-IEEE.

#[test]
fn countbits_counts_ones_and_zeros() {
    // v = 8'b1010_0011 ⇒ four 1s, four 0s; both ⇒ all 8 known bits.
    let (out, _c) = run("module t; reg [7:0] v;\n\
        initial begin v=8'b1010_0011;\n\
          $display(\"ones=%0d zeros=%0d both=%0d\", $countbits(v,1), $countbits(v,0), $countbits(v,0,1));\n\
        end endmodule\n");
    assert!(
        out.contains("ones=4 zeros=4 both=8"),
        "countbits 0/1:\n{out}"
    );
}

#[test]
fn countbits_counts_x_and_z() {
    // v = 4'b10xz ⇒ bit3=1, bit2=0, bit1=x, bit0=z.
    let (out, _c) = run("module t; reg [3:0] v;\n\
        initial begin v=4'b10xz;\n\
          $display(\"one=%0d zero=%0d x=%0d z=%0d\", $countbits(v,1), $countbits(v,0), $countbits(v,1'bx), $countbits(v,1'bz));\n\
        end endmodule\n");
    assert!(
        out.contains("one=1 zero=1 x=1 z=1"),
        "countbits x/z:\n{out}"
    );
}

#[test]
fn countbits_needs_a_control_bit() {
    // $countbits(expr) with no control bit is illegal (≥1 control required).
    let (out, code) =
        run("module t; reg [3:0] v; initial $display(\"%0d\", $countbits(v)); endmodule\n");
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "countbits without control should be loud: {out} code={code:?}"
    );
}

#[test]
fn typename_folds_to_type_string() {
    // $typename const-folds to vita's canonical type string (hand-IEEE: reg≡logic).
    let (out, _c) = run(
        "module t; reg [7:0] v; reg s; integer i; reg [7:0] m [0:3];\n\
        initial begin\n\
          $display(\"vec=<%s>\", $typename(v));\n\
          $display(\"scl=<%s>\", $typename(s));\n\
          $display(\"int=<%s>\", $typename(i));\n\
          $display(\"arr=<%s>\", $typename(m));\n\
        end endmodule\n",
    );
    assert!(out.contains("vec=<logic[7:0]>"), "vector typename:\n{out}");
    assert!(out.contains("scl=<logic>"), "scalar typename:\n{out}");
    assert!(out.contains("int=<integer>"), "integer typename:\n{out}");
    assert!(
        out.contains("arr=<logic[7:0]$[0:3]>"),
        "array typename:\n{out}"
    );
}

#[test]
fn exit_terminates_like_finish() {
    // $exit ends simulation like $finish: "A" prints, "B" does not.
    let (out, _c) =
        run("module t; initial begin $display(\"A\"); $exit; $display(\"B\"); end endmodule\n");
    assert!(out.contains("A"), "$exit should run prior stmts:\n{out}");
    assert!(!out.contains("B"), "$exit should terminate (no B):\n{out}");
}

// Adversarial-found fixes (workflow wyrhi3ukf):

#[test]
fn typename_string_var_is_string_not_logic() {
    // HIGH: the `string` decl branch skips intro_kind, so $typename used to default
    // to "logic". It must report "string".
    let (out, _c) = run(
        "module t; string str; initial begin $display(\"<%s>\", $typename(str)); end endmodule\n",
    );
    assert!(out.contains("<string>"), "string typename:\n{out}");
}

#[test]
fn typename_carries_signed_qualifier() {
    // MEDIUM: a signed vector must carry the `signed` qualifier (was silently dropped).
    let (out, _c) = run(
        "module t; logic signed [7:0] s8; initial begin $display(\"<%s>\", $typename(s8)); end endmodule\n",
    );
    assert!(
        out.contains("<logic signed[7:0]>"),
        "signed typename:\n{out}"
    );
}

#[test]
fn countbits_rejects_real_operand() {
    // MEDIUM: a real operand must be loud (like the sibling $countones), not a silent
    // popcount of the IEEE-754 storage.
    let (out, code) = run(
        "module t; real r; initial begin r=3.5; $display(\"%0d\", $countbits(r,1)); end endmodule\n",
    );
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "countbits(real) must be loud: {out} code={code:?}"
    );
}
