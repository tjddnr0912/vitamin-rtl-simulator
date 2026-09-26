//! A primitive cast is the context of its operand (IEEE 1800-2017 §6.24.1): `int'(e)` returns
//! what an `int` holds after being assigned `e`, so a context-determined operation inside it runs
//! at 32 bits.
//!
//! `lower_prim_cast` used to lower the operand at its own width and resize the result, so
//! `int'(u4 * u4)` with `u4 = 10` was the 4-bit product `00000004` where iverilog 13.0 and
//! verilator 5.052 both print `00000064`; `int'(-u4)` was `00000006` for `fffffff6`. The size
//! cast already took the context route (`size_ctx_route` + `lower_size_ctx_entry`); the prim cast
//! now takes the same one, guarded by `rhs_has_real_domain` so a real-domain operand
//! (`int'(r * 2)`, `int'(u4 * 2.0)`, `int'(u4 ** 0.5)`) keeps the real→int conversion — the route
//! alone answers a real VARIABLE by its net's sign and would refuse those.
//!
//! Every cell asserts the native value and that `--backend interp` and `--backend vm` print the
//! same, and every value is what both oracles print unless the cell says otherwise.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const DECLS: &str = "  logic [3:0] u4 = 4'b1010; logic [7:0] u8 = 8'hf0; logic [7:0] a = 8'hFF;
  logic signed [7:0] s8 = -8'sd3; logic signed [3:0] s4 = -4'sd3;
  real r = 2.5; parameter real RP = 1.5; bit c = 1;
  localparam logic [3:0] P = 4'd10;
  logic [3:0] arr [0:1];
  function real rf(); return 2.5; endfunction
  function logic [3:0] f4(); return 4'd10; endfunction
  sub u();
  initial begin arr[0] = 4'd1; arr[1] = 4'd10; end
";

fn run(src: &str, backend: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pccw_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "expected exit 0 on {backend}, got:\n{s}"
    );
    s
}

/// Display each expression with `%h` in one design and return the printed values, in order.
fn values(exprs: &[&str], backend: &str) -> Vec<String> {
    let mut body = String::new();
    for (i, e) in exprs.iter().enumerate() {
        body.push_str(&format!("    $display(\"V{i} %h\", {e});\n"));
    }
    let src = format!(
        "module sub; logic [7:0] v = 8'hFF; logic [3:0] v4 = 4'd10; endmodule\n\
         module t;\n{DECLS}  initial begin\n    #1;\n{body}    $finish;\n  end\nendmodule\n"
    );
    let out = run(&src, backend);
    (0..exprs.len())
        .map(|i| {
            let tag = format!("V{i} ");
            out.lines()
                .find_map(|l| l.strip_prefix(&tag))
                .unwrap_or_else(|| panic!("no line for {}:\n{out}", exprs[i]))
                .trim()
                .to_string()
        })
        .collect()
}

fn check(cells: &[(&str, &str)]) {
    let exprs: Vec<&str> = cells.iter().map(|c| c.0).collect();
    let want: Vec<String> = cells.iter().map(|c| c.1.to_string()).collect();
    let native = values(&exprs, "native");
    for (i, (e, w)) in cells.iter().enumerate() {
        assert_eq!(native[i], *w, "native {e}");
    }
    assert_eq!(native, want);
    for be in ["interp", "vm"] {
        assert_eq!(values(&exprs, be), native, "{be} disagrees with native");
    }
}

/// The operators that are context-determined (§11.6.1) run at the target width. PRE printed the
/// operand-width value on every line.
#[test]
fn a_prim_cast_is_the_context_of_its_operation() {
    check(&[
        ("int'(-u4)", "fffffff6"),                   // PRE 00000006
        ("int'(~u4)", "fffffff5"),                   // PRE 00000005
        ("int'(u4 - 4'd11)", "ffffffff"),            // PRE 0000000f
        ("int'(u8 << 4)", "00000f00"),               // PRE 00000000
        ("int'(u4 * u4)", "00000064"),               // PRE 00000004
        ("int'(u4 ** 2)", "00000064"),               // PRE 00000004
        ("int'(u8 + u8)", "000001e0"),               // PRE 000000e0
        ("int'({u4, u4} + 8'd255)", "000001a9"),     // PRE 000000a9
        ("int'(u4 ? u4 + 4'd8 : 4'd0)", "00000012"), // PRE 00000002
        ("int'(a * a)", "0000fe01"),                 // PRE 00000001
        ("int'(s8 * u8)", "0000ed30"),               // PRE 00000030
        ("int'(s4 + u4)", "00000017"),               // PRE 00000007
        ("int'(u4 * u4 + (r > 1.0))", "00000065"),   // PRE 00000005
        ("int'(u4 * $signed(u8))", "00000960"),      // PRE 00000060
        ("int'($unsigned(s8) * u4)", "000009e2"),    // PRE 000000e2
    ]);
}

/// Every primitive type's width is the context: `byte`, `shortint`, `longint`, `integer`,
/// `time`. PRE printed the operand-width value.
#[test]
fn every_primitive_width_is_the_context() {
    check(&[
        ("byte'(u4 * u4)", "64"),                                 // PRE 04
        ("shortint'(u4 + u4 + u4 + u4 + u4)", "0032"),            // PRE 0002
        ("shortint'(a * a)", "fe01"),                             // PRE 0001
        ("shortint'(~a)", "ff00"),                                // PRE 0000
        ("longint'(u8 * u8 * u8 * u8 * u8)", "000000b964f00000"), // PRE 0000000000000000
        ("integer'(-u4)", "fffffff6"),                            // PRE 00000006
        ("integer'(u8 * u8)", "0000e100"),                        // PRE 00000000
        ("time'(u8 * u8)", "000000000000e100"),                   // PRE 0000000000000000
    ]);
}

/// A 4-state cast keeps an unknown operand whole at the target width (iverilog `xxxxxxxx`; PRE
/// `0000000x`, the 4-bit sum zero-extended). The 2-state `int'` coerces it to 0 either way.
/// verilator is a 2-state simulator and prints `0000000a` for both — not an oracle here.
#[test]
fn a_four_state_cast_keeps_the_unknown_at_the_target_width() {
    check(&[
        ("integer'(u4 + 4'bx)", "xxxxxxxx"),
        ("int'(u4 + 4'bx)", "00000000"),
    ]);
}

/// The leaves the route reaches: a constant, a function return, an array element, a child
/// instance's net. PRE printed the 4- or 8-bit product.
#[test]
fn a_named_leaf_takes_the_context_too() {
    check(&[
        ("int'(u4 * P)", "00000064"),
        ("int'(f4() * u4)", "00000064"),
        ("int'(arr[1] * u4)", "00000064"),
        ("int'(u.v * u.v)", "0000fe01"),
        ("int'(u.v4 * u.v4)", "00000064"),
    ]);
}

/// A real-domain operand is not a bit-width context (§11.8.1): it keeps the real→int conversion
/// (round half away from zero). These were right before and must stay right; the route alone
/// refused them with E3009.
#[test]
fn a_real_domain_operand_keeps_the_conversion() {
    check(&[
        ("int'(r)", "00000003"),
        ("int'(r * 2)", "00000005"),
        ("int'(u4 * 2.0)", "00000014"),
        ("int'(-r)", "fffffffd"),
        ("int'(-rf())", "fffffffd"),
        ("int'($itor(u4) + 0.5)", "0000000b"),
        ("int'(c ? r : u4)", "00000003"),
        ("int'(RP + u4)", "0000000c"),
        ("int'(r ** 2)", "00000006"),
        ("int'(u4 ** 0.5)", "00000003"),
        ("byte'(-r)", "fd"),
        ("integer'(r + u4)", "0000000d"),
        ("longint'(r * 1e10)", "00000005d21dba00"),
        // the integral sub-expression of a real one is self-determined: (10*10 mod 16) * 1.0
        ("int'(u4 * u4 * 1.0)", "00000004"),
        ("int'(-u4 * r)", "0000000f"),
    ]);
}

/// Unchanged: the self-determined positions and the signed operands were already right, and
/// the size cast took the context route before this change.
#[test]
fn the_size_cast_twins_and_signed_operands_are_unchanged() {
    check(&[
        ("16'(-u4)", "fff6"),
        ("16'(u4 * u4)", "0064"),
        ("int'(-s8)", "00000003"),
        ("int'(s8 * s8)", "00000009"),
        ("int'(s8 >>> 1)", "fffffffe"),
        ("int'(u4 == 4'd10)", "00000001"),
        ("int'({u4} * 2)", "00000014"),
        ("int'(16'(u4) * u4)", "00000064"),
        ("bit'(u4 + u4)", "0"),
    ]);
}
