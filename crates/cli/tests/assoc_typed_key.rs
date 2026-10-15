//! §5.2 row 20: an associative-array index written as a TYPE — `[bit [31:0]]`,
//! `[int unsigned]`, `[logic signed [15:0]]`, `[addr_t]`, an enum — and the key
//! conversion every integral key now gets (IEEE 1800-2017 §7.8.4: the index
//! expression, self-determined, is cast to the index type).
//!
//! Oracles. Icarus Verilog 13.0 cannot declare any typed assoc (`t.sv:2: error:
//! Type names are not valid expressions here.` / `internal error: I do not know how
//! to elaborate this expression.`), and sv2v 0.0.13 has no associative arrays
//! (`Parse error: unexpected token '['`). Verilator 5.052 (`--binary`) is the one
//! tool, plus hand-IEEE where it splits from the LRM:
//!
//! * ORDER of a SIGNED key type: verilator walks every key in UNSIGNED order of its
//!   bits (`[int]`: 0, 4, 5, 259, -2147483648, -1), contradicting "smallest to
//!   largest" on the values it prints itself. §7.8.4 orders a signed index type by
//!   its signed value; vita did so before this row and still does.
//! * A NEGATIVE 32-bit index into an UNSIGNED key wider than 32 bits: verilator
//!   zero-extends `m[-1]` to `0ffffffff` but stores `m[33'(-1)]` and `m[t]` (`t = -1`)
//!   at `1ffffffff` — its own cast disagrees with its index. A cast extends by the
//!   OPERAND's sign (§6.24.1), so all three are `1ffffffff`.
//! * `prev(k)` from a key not in the array: verilator returns 0 (it looks the key
//!   up first); §7.9.7 finds the greatest smaller key.
//! * An x/z in the index: verilator is 2-state. §7.8.6: the index is invalid, the
//!   write is ignored with a warning (vita's rule before this row, kept).
//!
//! A key type the engine's signed 64-bit domain cannot hold exactly is refused by
//! name, as are the consumers a typed key would reach wrongly (a `foreach` whose
//! `integer` index cannot hold the key, an iteration variable of another type, an
//! index locator or `item.index`, a copy between two key types).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_atk_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The program's own lines (no `simulation ended` / summary lines).
fn lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
        .collect()
}

fn refused(src: &str, needle: &str) {
    let (_, err, code) = run(src);
    assert_eq!(code, 1, "must be refused:\n{err}");
    assert!(
        err.contains("VITA-E3009") && err.contains(needle),
        "expected a VITA-E3009 naming `{needle}`:\n{err}"
    );
}

/// The row-9 repro (VeeR EH1 / EL2 tb `ahb_sif.sv:45`), verbatim.
/// verilator: `mem=5a m2=a5 n=1`.
#[test]
fn row9_repro_bit32_and_int_unsigned_keys() {
    let (out, err, code) = run("module top;
  bit [7:0] mem [bit [31:0]];
  bit [7:0] m2  [int unsigned];
  initial begin
    mem[32'hF000_0000] = 8'h5A;
    m2[32'hF000_0001]  = 8'hA5;
    $display(\"mem=%h m2=%h n=%0d\", mem[32'hF000_0000], m2[32'hF000_0001], mem.num());
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["mem=5a m2=a5 n=1"]);
}

/// `[bit [7:0]]`: every index wraps to 8 bits — a negative literal, a 32- and a
/// 64-bit literal, an 8-bit carry — and the methods see the same keys. Verilator
/// prints these eleven lines exactly.
#[test]
fn bit8_key_wraps_every_index_to_the_key_width() {
    let (out, err, code) = run("module top;
  bit [7:0] m [bit [7:0]];
  bit [7:0] k;
  bit [7:0] a;
  int st;
  initial begin
    a = '1;
    m[5] = 8'h5a;
    m[-1] = 8'h11;
    m[0] = 8'h22;
    m[32'h103] = 8'h33;
    m[64'h1_0000_0004] = 8'h44;
    m[a + 1'b1] = 8'h55;
    m[8'h80] = 8'h66;
    $display(\"num=%0d\", m.num());
    st = m.first(k); while (st) begin $display(\"it k=%0d h=%h\", k, k); st = m.next(k); end
    st = m.last(k); $display(\"last=%0d st=%0d\", k, st);
    k = 3; st = m.prev(k); $display(\"prev=%0d k=%0d\", st, k);
    $display(\"rd5=%h rd3=%h rdm1=%h rd4=%h rd259=%h\", m[5], m[3], m[-1], m[4], m[259]);
    $display(\"ex=%0d %0d %0d\", m.exists(5), m.exists(259), m.exists(-1));
    m.delete(32'h105); $display(\"num2=%0d\", m.num());
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "num=6",
            "it k=0 h=00",
            "it k=3 h=03",
            "it k=4 h=04",
            "it k=5 h=05",
            "it k=128 h=80",
            "it k=255 h=ff",
            "last=255 st=1",
            "prev=1 k=0",
            "rd5=5a rd3=33 rdm1=11 rd4=44 rd259=33",
            "ex=1 1 1",
            "num2=5",
        ]
    );
}

/// KNOWN-WRONG, kept on purpose (round-1 soundness F1): the keyword spellings that
/// predate this row keep their pre-row-20 index — the expression widened to 64 bits,
/// never cast to the key type — because casting it breaks the engine's by-value
/// iteration fit test for an unsigned iteration variable (see
/// `keyword_key_walked_by_an_unsigned_variable_keeps_every_key`). ROADMAP §2 has the
/// line. verilator 5.052: `p i=-2 v=1` / `p i=-1 v=3` / `n i=1 v=2`; vita prints
/// `p i=254`, `p i=2147483647`, and the `foreach` over `n` stops on a key its
/// `integer` index cannot hold (W4020). The typed twin (`bit signed [7:0]` keys and
/// variables) is cast and agrees with verilator.
#[test]
fn keyword_keys_keep_the_uncast_index_known_wrong() {
    let src = "module top;
  bit [7:0] p [int];
  bit [7:0] n [byte];
  byte c, d; int e, f;
  initial begin
    c = 127; d = 127; e = 32'h7FFF_FFFF; f = 32'h7FFF_FFFF;
    p[c + d] = 1;
    n[e + f + 3] = 2;
    p[(e + f) >>> 1] = 3;
    foreach (p[i]) $display(\"p i=%0d v=%0d\", i, p[i]);
    foreach (n[i]) $display(\"n i=%0d v=%0d\", i, n[i]);
    $finish;
  end
endmodule
";
    let (out, err, code) = run(src);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("W4020"), "{err}");
    assert_eq!(lines(&out), ["p i=254 v=1", "p i=2147483647 v=3"]);
    // The typed twin: signed 32- and 8-bit keys, which a `foreach` cannot walk
    // (it is refused for an 8-bit key), so they are walked by hand.
    let (out, err, code) = run("module top;
  bit [7:0] p [bit signed [31:0]];
  bit [7:0] n [bit signed [7:0]];
  bit signed [31:0] k; bit signed [7:0] k8; int st;
  byte c, d; int e, f;
  initial begin
    c = 127; d = 127; e = 32'h7FFF_FFFF; f = 32'h7FFF_FFFF;
    p[c + d] = 1;
    n[e + f + 3] = 2;
    p[(e + f) >>> 1] = 3;
    st = p.first(k); while (st == 1) begin $display(\"p i=%0d v=%0d\", k, p[k]); st = p.next(k); end
    st = n.first(k8); while (st == 1) begin $display(\"n i=%0d v=%0d\", k8, n[k8]); st = n.next(k8); end
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["p i=-2 v=1", "p i=-1 v=3", "n i=1 v=2"]);
}

/// Round-1 soundness F1, the reason the keyword spellings keep the uncast index: an
/// unsigned iteration variable walks every key of an `[integer]` / `[int]` array
/// whose keys were written through unsigned indexes. verilator 5.052 and vita before
/// row 20: `A k=fffffff4 v=2 st=1`, `A k=fffffff0 v=1 st=1`, `A k=00000100 v=3 st=1`,
/// `A walked=3`; and, for a `bit [31:0]` variable over `[int]` (the lens's
/// q2_b32_iter), `A walked=4`. A cast index stored
/// these keys negative, and `prev` then never ended.
#[test]
fn keyword_key_walked_by_an_unsigned_variable_keeps_every_key() {
    let (out, err, code) = run("module top;
  logic [31:0] mem [integer];
  logic [31:0] k;
  int st, n;
  initial begin
    mem[32'hffff_fff0] = 1; mem[32'hffff_fff4] = 2; mem[32'h0000_0100] = 3;
    n = 0; st = mem.last(k);
    while (st != 0) begin $display(\"A k=%h v=%0d st=%0d\", k, mem[k], st); n++; st = mem.prev(k); end
    $display(\"A walked=%0d\", n);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "A k=fffffff4 v=2 st=1",
            "A k=fffffff0 v=1 st=1",
            "A k=00000100 v=3 st=1",
            "A walked=3"
        ]
    );
    let (out, err, code) = run("module top;
  logic [31:0] mem [int];
  bit [31:0] k;
  logic [31:0] a;
  int st, n;
  initial begin
    for (int i = 0; i < 3; i++) begin a = 32'h8000_0000 + 4*i; mem[a] = i + 10; end
    mem[32'h10] = 99;
    n = 0;
    st = mem.first(k);
    while (st != 0) begin $display(\"A it k=%h v=%0d st=%0d\", k, mem[k], st); n++; st = mem.next(k); end
    $display(\"A walked=%0d\", n);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "A it k=00000010 v=99 st=1",
            "A it k=80000000 v=10 st=1",
            "A it k=80000004 v=11 st=1",
            "A it k=80000008 v=12 st=1",
            "A walked=4"
        ]
    );
}

/// Unsigned atoms. verilator: `k=0` `k=2147483648` `k=4294967295` / `s=2 4 5`.
#[test]
fn unsigned_atom_keys_order_unsigned() {
    let (out, err, code) = run("module top;
  bit [7:0] m [int unsigned];
  bit [7:0] s [shortint unsigned];
  int st; int unsigned k;
  initial begin
    m[-1] = 1; m[0] = 2; m[32'h8000_0000] = 3;
    s[-1] = 4; s[70000] = 5;
    st = m.first(k); while (st == 1) begin $display(\"k=%0d\", k); st = m.next(k); end
    $display(\"s=%0d %0d %0d\", s.num(), s[65535], s[4464]);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        ["k=0", "k=2147483648", "k=4294967295", "s=2 4 5"]
    );
}

/// A signed packed key orders by value (hand-IEEE §7.8.4; verilator prints
/// `m k=3` first — unsigned bit order — and the same `n` lines).
#[test]
fn signed_typedef_key_orders_by_signed_value() {
    let (out, err, code) = run("module top;
  typedef logic signed [15:0] k_t;
  typedef int unsigned u_t;
  bit [7:0] m [k_t];
  bit [7:0] n [u_t];
  k_t k; u_t j; int st;
  initial begin
    m[-2] = 1; m[32'h0001_8000] = 2; m[3] = 3;
    n[-2] = 4; n[3] = 5;
    st = m.first(k); while (st == 1) begin $display(\"m k=%0d\", k); st = m.next(k); end
    st = n.first(j); while (st == 1) begin $display(\"n j=%0d\", j); st = n.next(j); end
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        ["m k=-32768", "m k=-2", "m k=3", "n j=3", "n j=4294967294"]
    );
}

/// `m[-1]`, `m[t]` with `t = -1` and `m[33'(-1)]` name one key of a `bit [32:0]`
/// array: a cast extends by the operand's sign (§6.24.1). verilator stores
/// `m[-1]` apart (`k=0ffffffff v=1`) while agreeing that the other two are
/// `1ffffffff` — its index and its own cast disagree.
#[test]
fn negative_index_sign_extends_into_a_wider_unsigned_key() {
    let (out, err, code) = run("module top;
  bit [7:0] m [bit [32:0]];
  bit [32:0] t;
  bit [32:0] k; int st;
  initial begin
    t = -1;
    m[-1] = 1;
    m[t] = 2;
    m[33'(-1)] = 3;
    st = m.first(k); while (st == 1) begin $display(\"k=%h v=%0d\", k, m[k]); st = m.next(k); end
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["k=1ffffffff v=3"]);
}

/// A typedef key — plain, package-scoped, imported — and an enum key with its
/// own iteration variable. verilator: `m1=1 n2=2`; `first=EA 1` `next=EB 6 st=1`
/// `n=2 e=1`.
#[test]
fn typedef_and_enum_keys() {
    let (out, err, code) = run("package p;
  typedef bit [11:0] addr_t;
endpackage
module top;
  import p::*;
  bit [7:0] m [addr_t];
  bit [7:0] n [p::addr_t];
  initial begin
    m[4097] = 1; n[4098] = 2;
    $display(\"m1=%0d n2=%0d\", m[1], n[2]);
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["m1=1 n2=2"]);
    let (out, err, code) = run("module top;
  typedef enum logic [2:0] {EA = 3'd1, EB = 3'd6} e_t;
  int m [e_t];
  e_t k; int st;
  initial begin
    m[EB] = 2; m[EA] = 1;
    st = m.first(k); $display(\"first=%s %0d\", k.name(), k);
    st = m.next(k); $display(\"next=%s %0d st=%0d\", k.name(), k, st);
    $display(\"n=%0d e=%0d\", m.num(), m.exists(EB));
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["first=EA 1", "next=EB 6 st=1", "n=2 e=1"]);
}

/// A key width from a parameter, per instance. verilator:
/// `W=4 n=2 m3=2 mmax=1` and `W=12 n=2 m3=2 mmax=1`.
#[test]
fn parameter_sized_key_folds_per_instance() {
    let (out, err, code) = run("module sub #(parameter int W = 4);
  bit [7:0] m [bit [W-1:0]];
  initial begin
    m[W'(0) - 1] = 1; m[(1 << W) + 3] = 2;
    #1 $display(\"W=%0d n=%0d m3=%0d mmax=%0d\", W, m.num(), m[3], m[(1<<W)-1]);
  end
endmodule
module top;
  sub #(4) a();
  sub #(12) b();
  initial #2 $finish;
endmodule
");
    assert_eq!(code, 0, "{err}");
    let mut got = lines(&out);
    got.sort_unstable();
    assert_eq!(got, ["W=12 n=2 m3=2 mmax=1", "W=4 n=2 m3=2 mmax=1"]);
}

/// The child's key type converts a hierarchical index (read and write), and a
/// function or task body indexes a module array the same way.
/// verilator: `h=3c m=3c`, `h=3c`, `r=42 n=2 e=1`.
#[test]
fn hierarchical_and_subroutine_indexes_convert() {
    let (out, err, code) = run("module mem;
  bit [7:0] mem [bit [31:0]];
  initial mem[32'hF000_0004] = 8'h3c;
endmodule
module top;
  mem u();
  bit [31:0] addr;
  initial begin
    #1 addr = 32'hF000_0000;
    $display(\"h=%h m=%h\", u.mem[addr + 4], u.mem[64'h1_F000_0004]);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["h=3c m=3c"]);
    let (out, err, code) = run("module mem;
  bit [7:0] mem [bit [31:0]];
endmodule
module top;
  mem u();
  initial begin
    u.mem[64'h1_F000_0004] = 8'h3c;
    #1 $display(\"h=%h\", u.mem[32'hF000_0004]);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["h=3c"]);
    let (out, err, code) = run("module top;
  bit [7:0] mem [bit [31:0]];
  function automatic bit [7:0] rd(bit [31:0] a);
    return mem[a + 32'd1];
  endfunction
  task automatic wr(bit [31:0] a, bit [7:0] v);
    mem[a] = v;
  endtask
  initial begin
    wr(32'hFFFF_FFFF, 8'h21);
    mem[0] = 8'h42;
    $display(\"r=%h n=%0d e=%0d\", rd(32'hFFFF_FFFF), mem.num(), mem.exists(-1));
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["r=42 n=2 e=1"]);
}

/// An x/z anywhere in the index makes it invalid (§7.8.6), even above the key's
/// width: the write is ignored with `VITA-W4020`. No tool oracle (verilator is
/// 2-state and stores `mn=2 nn=1`).
#[test]
fn an_unknown_index_bit_is_an_invalid_key_even_above_the_key_width() {
    let (out, err, code) = run("module top;
  bit [7:0] m [logic [7:0]];
  bit [7:0] n [int];
  logic [15:0] x16;
  initial begin
    x16 = 16'hxx05;
    m[x16] = 1;
    n[x16] = 2;
    m[8'b0000_010x] = 3;
    $display(\"mn=%0d nn=%0d\", m.num(), n.num());
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("W4020"), "{err}");
    assert_eq!(lines(&out), ["mn=0 nn=0"]);
}

/// Same key type: the whole-handle copy is a deep copy. verilator:
/// `b5=1 b255=2 n=2 a5=9`.
#[test]
fn copy_between_two_arrays_of_one_key_type() {
    let (out, err, code) = run("module top;
  bit [7:0] a [bit [7:0]];
  bit [7:0] b [bit [7:0]];
  initial begin
    a[5] = 1; a[255] = 2;
    b = a; a[5] = 9;
    $display(\"b5=%0d b255=%0d n=%0d a5=%0d\", b[5], b[255], b.num(), a[5]);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["b5=1 b255=2 n=2 a5=9"]);
}

// ── refused by name ────────────────────────────────────────────────────────────

#[test]
fn a_64_bit_unsigned_key_is_refused() {
    for key in [
        "bit [63:0]",
        "longint unsigned",
        "logic [0:63]",
        "bit [7:0][7:0]",
    ] {
        refused(
            &format!("module t; bit [7:0] m [{key}]; initial m[1] = 1; endmodule\n"),
            "64-bit UNSIGNED",
        );
    }
}

#[test]
fn a_key_wider_than_64_bits_is_refused() {
    refused(
        "module t; bit [7:0] m [bit [64:0]]; initial m[1] = 1; endmodule\n",
        "65 bits is wider than",
    );
}

/// The widths around the domain's edge that DO fit: 63-bit unsigned, 64-bit
/// signed. verilator prints `m k=00000000ffffffff`, `m k=4000000000000005`,
/// `n j=5`, `n j=-9223372036854775808` — both module-doc splits at once: `m[-1]`
/// zero-extended, and the signed key walked in unsigned bit order.
#[test]
fn the_widest_keys_that_fit() {
    let (out, err, code) = run("module top;
  bit [7:0] m [bit [62:0]];
  bit [7:0] n [bit signed [63:0]];
  bit [62:0] k; bit signed [63:0] j; int st;
  initial begin
    m[64'hC000_0000_0000_0005] = 1; m[-1] = 2;
    n[64'h8000_0000_0000_0000] = 3; n[5] = 4;
    st = m.first(k); while (st == 1) begin $display(\"m k=%h\", k); st = m.next(k); end
    st = n.first(j); while (st == 1) begin $display(\"n j=%0d\", j); st = n.next(j); end
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "m k=4000000000000005",
            "m k=7fffffffffffffff",
            "n j=-9223372036854775808",
            "n j=5"
        ]
    );
}

#[test]
fn foreach_over_a_key_its_integer_index_cannot_hold_is_refused() {
    refused(
        "module t; bit [7:0] m [bit [31:0]]; initial begin m[1] = 1; \
         foreach (m[i]) $display(i); end endmodule\n",
        "`foreach` over an associative array keyed by a 32-bit unsigned type",
    );
    // A 32-bit signed typed key walks: the `integer` index holds it.
    let (out, err, code) = run("module t; bit [7:0] m [bit signed [31:0]]; initial begin \
         m[-3] = 1; m[32'hFFFF_FFFE] = 2; foreach (m[i]) $display(\"i=%0d v=%0d\", i, m[i]); \
         end endmodule\n");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["i=-3 v=1", "i=-2 v=2"]);
}

#[test]
fn an_iteration_variable_of_another_type_is_refused() {
    refused(
        "module t; bit [7:0] m [bit [7:0]]; int k, st; initial begin m[1] = 1; \
         st = m.first(k); end endmodule\n",
        "must have that type (it is 32-bit signed)",
    );
    refused(
        "module t; bit [7:0] m [bit [7:0]]; byte k; int st; initial begin m[1] = 1; \
         st = m.next(k); end endmodule\n",
        "keyed by a 8-bit unsigned type must have that type (it is 8-bit signed)",
    );
}

#[test]
fn an_index_locator_or_item_index_is_refused() {
    refused(
        "module t; bit [7:0] m [bit [7:0]]; bit [7:0] q[$]; initial begin m[5] = 1; \
         q = m.find_index() with (item == 1); end endmodule\n",
        "index-returning locator",
    );
    refused(
        "module t; bit [7:0] m [bit [7:0]]; int s; initial begin m[5] = 1; \
         s = m.sum() with (int'(item.index)); end endmodule\n",
        "`item.index`",
    );
    // A value locator and a value reduction read no key. verilator: `v='{'h2, 'h2}`
    // and `t=5`.
    let (out, err, code) = run("module t; bit [7:0] m [bit [7:0]]; bit [7:0] v[$]; int t; \
         initial begin m[5] = 1; m[200] = 2; m[7] = 2; v = m.find() with (item == 2); \
         t = m.sum() with (int'(item)); $display(\"v=%p t=%0d\", v, t); end endmodule\n");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["v='{'h2, 'h2} t=5"]);
}

#[test]
fn a_copy_between_two_key_types_is_refused() {
    // verilator rejects `[int]` <- `[bit [7:0]]` at compile; it accepts
    // `[int]` <- `[int unsigned]` by reinterpreting the bits. Both pair a typed key
    // with a keyword one; two typed keys of different types are refused too.
    for src in [
        "module t; bit [7:0] a [bit [7:0]]; bit [7:0] b [int]; \
         initial begin a[5] = 1; b = a; end endmodule\n",
        "module t; bit [7:0] a [int unsigned]; bit [7:0] b [int]; \
         initial begin a[5] = 1; b = a; end endmodule\n",
    ] {
        refused(src, "vita never copies between the two");
    }
    refused(
        "module t; bit [7:0] a [bit [7:0]]; bit [7:0] b [bit signed [7:0]]; \
         initial begin a[5] = 1; b = a; end endmodule\n",
        "same key type on both sides (width and sign)",
    );
}

/// Round-2 soundness R2-1: a keyword atom written with an explicit `signed` was E2002
/// before row 20, so it is a TYPED key — cast, like every typed key — and not the
/// uncast keyword spelling. The lens's s_atom_signed without its `%p` line;
/// verilator 5.052:
/// `A b44=1 b300ex=1 s4464=2 s70000ex=1 i5=3 g6=4 bm1=5 b255ex=1`. Its `%p` of the
/// `[byte signed]` array (verilator `'{'h2c:'h1, 'hff:'h5}`) is refused as every typed
/// signed key's is.
#[test]
fn an_explicitly_signed_atom_is_a_typed_key() {
    let (out, err, code) = run("module top;
  bit [7:0] b [byte signed];
  bit [7:0] s [shortint signed];
  bit [7:0] i [int signed];
  bit [7:0] g [integer signed];
  bit [7:0] l [longint signed];
  bit [7:0] u8;
  initial begin
    b[300] = 1; s[70000] = 2; i[64'h1_0000_0005] = 3; g[64'h1_0000_0006] = 4;
    u8 = 8'hff; b[u8] = 5;
    $display(\"A b44=%0d b300ex=%0d s4464=%0d s70000ex=%0d i5=%0d g6=%0d bm1=%0d b255ex=%0d\", b[44], b.exists(300), s[4464], s.exists(70000), i[5], g[6], b[-1], b.exists(255));
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        ["A b44=1 b300ex=1 s4464=2 s70000ex=1 i5=3 g6=4 bm1=5 b255ex=1"]
    );
    refused(
        "module t; bit [7:0] b [byte signed]; initial begin b[-1] = 5; \
         $display(\"%p\", b); end endmodule\n",
        "`%p` of an associative array keyed by a 8-bit signed typed key",
    );
}

/// A typedef key the parser cannot lay out where it was declared, or whose name the
/// unit also uses otherwise, stays loud. An `enum bit [N]` key (recorded 4-state) is
/// refused at parse by name. A struct key (its typedef statement holds a member's
/// `;`) and a type parameter (`type T = …`) fail the name scan closed and keep the
/// pre-row-20 size reading, refused at elaborate as before (`undefined name`).
/// verilator runs all three.
#[test]
fn a_typedef_key_without_a_fixed_layout_stays_loud() {
    let (_, err, code) =
        run("module t; typedef enum bit [1:0] {EA, EB} e_t; int m [e_t]; endmodule\n");
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("VITA-E2002") && err.contains("associative-array key type"),
        "{err}"
    );
    for src in [
        "module t; typedef struct packed { bit [3:0] a; bit [3:0] b; } s_t; \
         bit [7:0] m [s_t]; endmodule\n",
        "module t #(parameter type T = bit [3:0]); bit [7:0] m [T]; endmodule\n",
    ] {
        let (_, err, code) = run(src);
        assert_eq!(code, 1, "{err}");
        assert!(
            err.contains("VITA-E3009") && err.contains("undefined name"),
            "{err}"
        );
    }
}

/// A key the array does not hold reads the ELEMENT type's default (IEEE 1800-2017
/// §7.8.6, Table 7-1): `0` for a 2-state element, so the counter idiom counts, and
/// `x` for a 4-state one. verilator: `a=3 c5=7 b3=1 l1=01 k=0 n=1` — 2-state, so
/// its `l1=01` cannot speak for a 4-state element (hand-IEEE: `x + 1` is `xx`).
/// Before row 20 every element read `x` (`a=x c5=x`).
#[test]
fn a_missing_key_reads_the_element_types_default() {
    let (out, err, code) = run("module top;
  int cnt [string];
  int c2 [int];
  bit [7:0] b [bit [7:0]];
  logic [7:0] l [int];
  int k;
  initial begin
    cnt[\"a\"]++;
    cnt[\"a\"] += 2;
    c2[5] += 7;
    b[3] = b[3] + 1;
    l[1] = l[1] + 1;
    k = c2[9];
    $display(\"a=%0d c5=%0d b3=%0d l1=%h k=%0d n=%0d\", cnt[\"a\"], c2[5], b[3], l[1], k, c2.num());
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("assoc key not found (read 0)"), "{err}");
    assert!(err.contains("assoc key not found (read X)"), "{err}");
    assert_eq!(lines(&out), ["a=3 c5=7 b3=1 l1=xx k=0 n=1"]);
}

/// Round-1 soundness F2: `%p` renders an integral key as its stored signed i64, so
/// a negative key of a typed SIGNED key type would print 64 bits wide
/// (`'hfffffffffffffffe` where verilator 5.052 prints `'hffe`). Refused by name. A
/// typed UNSIGNED key is stored non-negative and prints as verilator does:
/// `A pu '{'hfffffffe:'h5}`, `A pe '{'hfffffffe:'h6}`, `A pb '{'h5:'h1, 'hff:'h2}`.
/// The keyword atoms keep their pre-row-20 render, which verilator matches for a key
/// written through an unsigned index: `A pm '{'h3:'h2, 'hfffffff0:'h1}`,
/// `A pb '{'hf0:'h3}`, `A ps '{'hfff0:'h4}`.
#[test]
fn pattern_render_of_a_typed_signed_key_is_refused() {
    for key in [
        "bit signed [11:0]",
        "logic signed [15:0]",
        "bit signed [31:0]",
    ] {
        refused(
            &format!(
                "module t; bit [7:0] a [{key}]; initial begin a[-2] = 1; \
                 $display(\"%p\", a); end endmodule\n"
            ),
            "`%p` of an associative array keyed by",
        );
    }
    refused(
        "module c; typedef bit signed [11:0] k_t; bit [7:0] a [k_t]; initial a[-2] = 1; endmodule\n\
         module t; c u(); initial #1 $display(\"%p\", u.a); endmodule\n",
        "`%p` of an associative array keyed by a 12-bit signed",
    );
    let (out, err, code) = run("module top;
  bit [7:0] u [int unsigned];
  bit [7:0] e [integer unsigned];
  bit [7:0] b [bit [7:0]];
  initial begin
    u[-2] = 5;
    e[-2] = 6;
    b[5] = 1; b[-1] = 2;
    $display(\"A pu %p\", u);
    $display(\"A pe %p\", e);
    $display(\"A pb %p\", b);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "A pu '{'hfffffffe:'h5}",
            "A pe '{'hfffffffe:'h6}",
            "A pb '{'h5:'h1, 'hff:'h2}"
        ]
    );
    let (out, err, code) = run("module top;
  bit [7:0] m [int];
  bit [7:0] b [byte];
  bit [7:0] s [shortint];
  bit [31:0] a;
  bit [7:0] a8;
  bit [15:0] a16;
  initial begin
    a = 32'hffff_fff0; m[a] = 1; m[3] = 2;
    a8 = 8'hf0; b[a8] = 3;
    a16 = 16'hfff0; s[a16] = 4;
    $display(\"A pm %p\", m);
    $display(\"A pb %p\", b);
    $display(\"A ps %p\", s);
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        lines(&out),
        [
            "A pm '{'h3:'h2, 'hfffffff0:'h1}",
            "A pb '{'hf0:'h3}",
            "A ps '{'hfff0:'h4}"
        ]
    );
}

/// Round-2 differential B2: an unbased unsized fill literal as a typed-key index
/// has two readings and no oracle to settle them — self-determined, `'1` is one bit
/// (key 1); sized by the cast to the key type (§6.24.1), all ones (verilator 5.052
/// `m k=ffffffff`, and vita's own `32'('1)` is `ffffffff`). Refused by name. A fill
/// inside a larger index (`'1 + 0`, self-determined at 32 bits) has one reading
/// and runs.
#[test]
fn a_bare_fill_literal_index_of_a_typed_key_is_refused() {
    for idx in ["'1", "'0", "('1)", "'x"] {
        refused(
            &format!(
                "module t; bit [7:0] m [bit [15:0]]; initial begin m[{idx}] = 2; end endmodule\n"
            ),
            "unbased unsized fill literal",
        );
    }
    refused(
        "module t; bit [7:0] m [bit [15:0]]; initial begin m[5] = 2; \
         $display(\"%0d\", m.exists('1)); end endmodule\n",
        "unbased unsized fill literal",
    );
    // verilator: `A u k=ffff v=3`.
    let (out, err, code) = run("module top;
  bit [7:0] u [bit [15:0]];
  bit [15:0] k; int st;
  initial begin
    u['1 + 0] = 3;
    st = u.first(k); while (st == 1) begin $display(\"A u k=%h v=%0d\", k, u[k]); st = u.next(k); end
    $finish;
  end
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["A u k=ffff v=3"]);
}

/// Round-2 differential B1: a name in brackets is a key TYPE only when no value of
/// that name appears in the unit; a parameter, localparam or generate localparam
/// that shadows an imported, `$unit` or outer typedef keeps the array's size, as it
/// did before row 20. Each source is the lens's cell; vita before row 20 and
/// verilator 5.052 agree on every printed value.
#[test]
fn a_value_shadowing_a_typedef_keeps_the_size_reading() {
    for (src, want) in [
        (
            "package p;
  typedef int N;
endpackage
module sub #(parameter int N = 4) ();
  import p::*;
  bit [7:0] m [N];
  int cnt;
  initial begin
    m[1] = 8'h5A;
    foreach (m[i]) cnt++;
    $display(\"cnt=%0d\", cnt);
  end
endmodule
module top;
  sub #(.N(6)) u();
  initial #1 $finish;
endmodule
",
            vec!["cnt=6"],
        ),
        (
            "package p;
  typedef int N;
endpackage
module sub import p::*; #(parameter int N = 4) ();
  bit [7:0] m [N];
  int cnt;
  initial begin
    m[1] = 8'h5A;
    foreach (m[i]) cnt++;
    $display(\"cnt=%0d\", cnt);
  end
endmodule
module top;
  sub #(.N(6)) u();
  initial #1 $finish;
endmodule
",
            vec!["cnt=6"],
        ),
        (
            "typedef bit [7:0] DEPTH;
module sub #(parameter int DEPTH = 4) ();
  int mem [DEPTH];
  int cnt;
  initial begin
    mem[1] = 7;
    foreach (mem[i]) cnt++;
    $display(\"cnt=%0d\", cnt);
  end
endmodule
module top;
  sub #(.DEPTH(6)) u();
  initial #1 $finish;
endmodule
",
            vec!["cnt=6"],
        ),
        (
            "package p;
  typedef int N;
endpackage
module top;
  import p::*;
  localparam int N = 4;
  bit [7:0] m [N];
  initial begin
    m[3] = 8'h5A;
    $display(\"size=%0d m3=%h\", $size(m), m[3]);
    $finish;
  end
endmodule
",
            vec!["size=4 m3=5a"],
        ),
        (
            "package p;
  typedef int N;
endpackage
module top;
  import p::*;
  localparam int N = 4;
  bit [7:0] m [N];
  int cnt;
  initial begin
    m[1] = 8'h5A;
    foreach (m[i]) cnt++;
    $display(\"cnt=%0d m1=%h m2=%h\", cnt, m[1], m[2]);
    $finish;
  end
endmodule
",
            vec!["cnt=4 m1=5a m2=00"],
        ),
        (
            "module top;
  typedef bit [7:0] N;
  for (genvar g = 0; g < 1; g++) begin : gb
    localparam int N = 4;
    bit [7:0] m [N];
    initial begin
      m[3] = 8'h5A;
      $display(\"size=%0d m3=%h\", $size(m), m[3]);
    end
  end
  initial #1 $finish;
endmodule
",
            vec!["size=4 m3=5a"],
        ),
    ] {
        let (out, err, code) = run(src);
        assert_eq!(code, 0, "{err}");
        assert_eq!(lines(&out), want, "{src}");
    }
    // i_td_gen_shadow_fe: a fixed `[4]` array (vita `m44=xx` with E4002 for the
    // out-of-range `m[300]`, verilator `m44=77`: out-of-range writes are §2's).
    let (out, err, code) = run("module top;
  typedef bit [7:0] N;
  for (genvar g = 0; g < 1; g++) begin : gb
    localparam int N = 4;
    bit [7:0] m [N];
    int cnt;
    initial begin
      m[1] = 8'h5A;
      m[300] = 8'h77;
      foreach (m[i]) cnt++;
      $display(\"cnt=%0d m1=%h m44=%h\", cnt, m[1], m[44]);
    end
  end
  initial #1 $finish;
endmodule
");
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("E4002"), "{err}");
    assert_eq!(lines(&out), ["cnt=4 m1=5a m44=xx"]);
}

/// A keyword-atom array holds UNCAST keys, so it never copies to or from a typed
/// one, even of the same width and sign; two keyword atoms keep the pre-row-20 copy
/// (`assoc_key_spellings::whole_copy_across_spellings_of_the_same_domain`).
#[test]
fn a_copy_between_a_keyword_key_and_a_typed_key_is_refused() {
    refused(
        "module t; bit [7:0] a [int]; bit [7:0] b [bit signed [31:0]]; \
         initial begin a[5] = 1; b = a; end endmodule\n",
        "vita never copies between the two",
    );
}
