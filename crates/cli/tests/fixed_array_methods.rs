//! V34-4: the IEEE 1800 §7.12 array manipulation methods on a FIXED-SIZE
//! unpacked array — the reductions (§7.12.3 `.sum()`/`.product()`/`.and()`/
//! `.or()`/`.xor()`, with and without a `with` clause) and the ordering methods
//! (§7.12.2 `.sort()`/`.rsort()`/`.reverse()`).
//!
//! ## Which tool is the oracle, measured on this machine (2026-08-25)
//!
//! The report item claimed "iverilog supports these on fixed arrays". It does
//! not, and that was measured before anything was implemented:
//!
//! * `int a[4]; s = a.sum();` → iverilog 13:
//!   `error: Object tb.a has no method "sum(...)".`
//! * `s = a.sum() with (item);` → iverilog 13 does not even PARSE it
//!   (`syntax error` / `Malformed statement`), and neither does the QUEUE
//!   spelling `q.sum() with (item)` — iverilog 13 has no `with` clause at all.
//!   `.and()`/`.or()`/`.xor()` additionally collide with the operator keywords.
//! * `a.sort();` → iverilog 13: ``error: Enable of unknown task `a.sort'.``
//!
//! **verilator 5.050 is the sole oracle here**, and a legitimate one for this
//! axis: the values below are 2-state `int`/`byte`/`logic` arithmetic, not
//! 4-state x-propagation. Every value asserted in this file that verilator can
//! answer was compared against it; the exceptions are called out per test.
//!
//! ## Where vita deliberately does NOT follow verilator
//!
//! Two answers below differ from verilator, and in both the DYNAMIC-storage twin
//! that vita has shipped for many slices already gives the same answer, measured
//! side by side in `narrow_element_fold_matches_the_queue_twin` and
//! `signed_sort_matches_the_queue_twin`. Making the fixed array agree with the
//! queue is the point: this slice routes a second storage class into ONE piece of
//! machinery, so a divergence between the two spellings would be the defect.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fxarr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn assert_loud(src: &str, what: &str, needle: &str) {
    let (_, err, ok) = run_full(src);
    assert!(
        !ok,
        "{what}: must exit non-zero (loud reject); stderr:\n{err}"
    );
    assert!(
        err.contains(needle),
        "{what}: the refusal must say why ({needle:?}); got:\n{err}"
    );
}

// ── reductions with a `with` clause ──────────────────────────────────────────

#[test]
fn with_clause_reductions_on_a_fixed_array() {
    // verilator 5.050 on the identical source:
    //   SUM=10 SUM2=20 PROD=24 XOR=4 AND=0 OR=7 BARE=10
    // PRE (HEAD before V34-4) every one of these lines was
    //   E3009 "array reduction `with` applies to a dynamic array / queue / assoc handle".
    let out = run("module top; int a[4]; int s;\n\
         initial begin\n\
           a[0]=1; a[1]=2; a[2]=3; a[3]=4;\n\
           s = a.sum() with (item);     $display(\"SUM=%0d\", s);\n\
           s = a.sum() with (item*2);   $display(\"SUM2=%0d\", s);\n\
           s = a.product() with (item); $display(\"PROD=%0d\", s);\n\
           s = a.xor() with (item);     $display(\"XOR=%0d\", s);\n\
           s = a.and() with (item);     $display(\"AND=%0d\", s);\n\
           s = a.or() with (item);      $display(\"OR=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(
        out, "SUM=10\nSUM2=20\nPROD=24\nXOR=4\nAND=0\nOR=7\n",
        "must match verilator 5.050 value for value"
    );
}

#[test]
fn bare_reductions_on_a_fixed_array() {
    // The `with`-less spelling. PRE it was not even recognised as an array
    // method: "unsupported hierarchical function call `a.sum` (the callee must
    // be a framed function … reached through an instance path)" — a message
    // about instance paths for an expression that names a local array.
    // verilator 5.050: SUM=10 PROD=24 AND=0 OR=7 XOR=4.
    let out = run("module top; int a[4]; int s;\n\
         initial begin\n\
           a[0]=1; a[1]=2; a[2]=3; a[3]=4;\n\
           s = a.sum();     $display(\"SUM=%0d\", s);\n\
           s = a.product(); $display(\"PROD=%0d\", s);\n\
           s = a.and();     $display(\"AND=%0d\", s);\n\
           s = a.or();      $display(\"OR=%0d\", s);\n\
           s = a.xor();     $display(\"XOR=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "SUM=10\nPROD=24\nAND=0\nOR=7\nXOR=4\n");
}

#[test]
fn a_one_element_array_is_still_an_array() {
    // `int a[1]` has `array_len == 1`, which `array_len > 1` cannot tell from a
    // scalar — the gate asks DECLARED array-ness instead (`net_is_static_array`).
    let out = run("module top; int a[1]; int s;\n\
         initial begin a[0]=42; s = a.sum() with (item); $display(\"S=%0d\", s); end\n\
         endmodule\n");
    assert_eq!(out, "S=42\n");
}

// ── `item.index` — the DECLARED index, not the flat slot ─────────────────────

#[test]
fn item_index_is_the_declared_index() {
    // §7.12.3's `index` is the array index. The engine iterates FLAT slots, so a
    // non-zero declared base has to be added back.
    //
    // ⚠️ NO TOOL ORACLE EXISTS FOR THIS ROW and one of them is actively wrong:
    // verilator 5.050 answers 0 for `item.index` over ANY fixed array (measured
    // on all three arrays below), while answering it CORRECTLY for a queue
    // (`int q[$]` of two elements → 1 = 0+1). Its own neighbouring answer is what
    // disqualifies it here. iverilog cannot parse a `with` clause at all. So this
    // row is hand-IEEE, and the values are hand-computable:
    //   int a[4]   → 0+1+2+3 = 6
    //   int b[3:0] → 3+2+1+0 = 6   (same index SET; every reduction is commutative)
    //   int c[-1:1] → -1+0+1 = 0
    //
    // ⚠️ The FIRST implementation of the rebase used `intern_const`, which
    // returns a CONST-POOL id, as if it were an ExprId — so the `Add`'s rhs was
    // whatever expression happened to sit at that index and `int c[-1:1]`
    // answered 93 (indices 30/31/32) and -1217425105 on a neighbouring design.
    // That is why this test asserts the c[-1:1] VALUE and not merely "it runs".
    let out = run("module top; int a[4]; int b[3:0]; int c[-1:1]; int s;\n\
         initial begin\n\
           a[0]=10; a[1]=20; a[2]=30; a[3]=40;\n\
           b[0]=10; b[1]=20; b[2]=30; b[3]=40;\n\
           c[-1]=1; c[0]=2; c[1]=3;\n\
           s = a.sum() with (item.index); $display(\"A=%0d\", s);\n\
           s = b.sum() with (item.index); $display(\"B=%0d\", s);\n\
           s = c.sum() with (item.index); $display(\"C=%0d\", s);\n\
           s = c.sum() with (item);       $display(\"CV=%0d\", s);\n\
           s = c.or()  with (item.index); $display(\"COR=%0d\", s);\n\
         end endmodule\n");
    // COR = (-1) | 0 | 1 = -1 (all bits set) — the negative index really is
    // negative, not a wrapped unsigned slot number.
    assert_eq!(out, "A=6\nB=6\nC=0\nCV=6\nCOR=-1\n");
}

#[test]
fn item_and_item_index_are_paired_element_wise() {
    // A sum of indices cannot tell a correct pairing from a rotated one — every
    // reduction is commutative, so the index SET alone is not a test. This one
    // multiplies the element by its index, which is not.
    // `int b[3:0]` = {b[0]=1, b[1]=2, b[2]=4, b[3]=8}
    //   → 1*0 + 2*1 + 4*2 + 8*3 = 34.
    let out = run("module top; int b[3:0]; int s;\n\
         initial begin\n\
           b[0]=1; b[1]=2; b[2]=4; b[3]=8;\n\
           s = b.sum() with (item * item.index); $display(\"P=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "P=34\n");
}

#[test]
fn a_queue_item_index_is_unchanged() {
    // The rebase is opt-in per receiver: a dynamic-storage handle has no declared
    // bounds and passes base 0, so its lowering is byte-for-byte what it was.
    // verilator 5.050 agrees here (1 = 0+1) — the only `item.index` row it can.
    let out = run("module top; int q[$]; int s;\n\
         initial begin\n\
           q.push_back(5); q.push_back(6);\n\
           s = q.sum() with (item.index); $display(\"Q=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "Q=1\n");
}

// ── element types and widths ─────────────────────────────────────────────────

#[test]
fn signed_and_unsigned_element_types() {
    // verilator 5.050 on the identical source: BSUM=2 BPROD=24 USUM=254 UXOR=0
    // UAND=0 ESUM=42 BIGSUM=1705032704.
    //   byte b[4]  = {-1, 2, -3, 4}   → sum 2, product 24
    //   logic [7:0] u[4] = {F0,0F,AA,55} → sum 254 (8-bit wrap of 0x1FE),
    //                                       xor 0, and 0
    //   int big[3] = 2e9 ×3           → 6e9 wrapped to 32 bits = 1705032704
    let out = run(
        "module top; byte b[4]; logic [7:0] u[4]; int e[1]; int big[3]; int s;\n\
         initial begin\n\
           b[0]=-8'sd1; b[1]=8'sd2; b[2]=-8'sd3; b[3]=8'sd4;\n\
           s = b.sum() with (item);     $display(\"BSUM=%0d\", s);\n\
           s = b.product() with (item); $display(\"BPROD=%0d\", s);\n\
           u[0]=8'hF0; u[1]=8'h0F; u[2]=8'hAA; u[3]=8'h55;\n\
           s = u.sum() with (item);     $display(\"USUM=%0d\", s);\n\
           s = u.xor() with (item);     $display(\"UXOR=%0d\", s);\n\
           s = u.and() with (item);     $display(\"UAND=%0d\", s);\n\
           e[0]=42; s = e.sum() with (item); $display(\"ESUM=%0d\", s);\n\
           big[0]=2000000000; big[1]=2000000000; big[2]=2000000000;\n\
           s = big.sum() with (item);   $display(\"BIGSUM=%0d\", s);\n\
         end endmodule\n",
    );
    assert_eq!(
        out,
        "BSUM=2\nBPROD=24\nUSUM=254\nUXOR=0\nUAND=0\nESUM=42\nBIGSUM=1705032704\n"
    );
}

#[test]
fn narrow_element_fold_matches_the_queue_twin() {
    // ⚠️ vita and verilator 5.050 DISAGREE on this row and it is NOT this
    // slice's doing: the accumulator width.
    //
    //   `logic [3:0]` elements 7, 8, 9 → 24, which does not fit 4 bits.
    //   vita:      8  (24 & 0xF — the result takes the element / with-expr type)
    //   verilator: 24 (accumulates in a wider type)
    //
    // IEEE 1800 §7.12.3's own example is what settles it: `count = bit_arr.sum
    // with (int'(item));  // count is the number of 1's` — the explicit `int'()`
    // cast on a 1-bit element only makes sense if the fold is otherwise done at
    // the ELEMENT width. vita implements that rule.
    //
    // The load-bearing part of this test is the SECOND assertion: the queue
    // spelling, shipped and reviewed long before V34-4, gives the same 8. Routing
    // the fixed array into that machinery must not make the two spellings of one
    // §7.12.3 rule disagree, and this is what would catch it if they ever did.
    let out = run("module top; logic [3:0] q[$]; logic [3:0] n[0:2]; int s;\n\
         initial begin\n\
           q.push_back(4'd7); q.push_back(4'd8); q.push_back(4'd9);\n\
           n[0]=4'd7; n[1]=4'd8; n[2]=4'd9;\n\
           s = q.sum(); $display(\"Q=%0d\", s);\n\
           s = n.sum(); $display(\"N=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(
        out, "Q=8\nN=8\n",
        "the fixed array must fold exactly as the queue does"
    );
}

#[test]
fn four_state_elements_match_the_queue_twin() {
    // x-propagation: verilator is NOT an oracle for 4-state, and iverilog cannot
    // reach these methods, so the check is parity with the dyn-storage twin plus
    // hand-IEEE. `8'h01 + 8'hxx + 8'h02` is x throughout; `01 | xx | 02` leaves
    // the bits an x could not decide as x (`xX` in vita's %h nibble spelling,
    // where an uppercase X is a partially-unknown nibble).
    let out = run("module top; logic [7:0] a[3]; logic [7:0] q[$];\n\
         initial begin\n\
           a[0]=8'h01; a[1]=8'hxx; a[2]=8'h02;\n\
           q.push_back(8'h01); q.push_back(8'hxx); q.push_back(8'h02);\n\
           $display(\"S=%h|%h\", a.sum() with (item), q.sum() with (item));\n\
           $display(\"O=%h|%h\", a.or()  with (item), q.or()  with (item));\n\
           $display(\"B=%h|%h\", a.sum(), q.sum());\n\
         end endmodule\n");
    assert_eq!(out, "S=xx|xx\nO=xX|xX\nB=xx|xx\n");
}

// ── ordering methods ─────────────────────────────────────────────────────────

#[test]
fn ordering_methods_on_a_fixed_array() {
    // verilator 5.050 on the identical source: SORT=1 2 3 4, RSORT=4 3 2 1,
    // REV=1 2 3 4 (reverse of the just-rsorted array). PRE all three lines were
    // "unsupported hierarchical task call `a.sort`".
    let out = run("module top; int a[4];\n\
         initial begin\n\
           a[0]=3; a[1]=1; a[2]=4; a[3]=2;\n\
           a.sort();    $display(\"SORT=%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]);\n\
           a.rsort();   $display(\"RSORT=%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]);\n\
           a.reverse(); $display(\"REV=%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]);\n\
         end endmodule\n");
    assert_eq!(out, "SORT=1 2 3 4\nRSORT=4 3 2 1\nREV=1 2 3 4\n");
}

#[test]
fn ordering_methods_honour_the_declared_index_space() {
    // verilator 5.050 on the identical source: B=1 2 3 4, C=5 7 9, CR=9 7 5.
    // A descending (`[3:0]`) and a negative-based (`[-1:1]`) array both store
    // slot k at declared index lo+k, and the sort is over the SLOTS, so reading
    // back through the declared indices must show ascending order.
    let out = run("module top; int b[3:0]; int c[-1:1];\n\
         initial begin\n\
           b[0]=3; b[1]=1; b[2]=4; b[3]=2;\n\
           b.sort(); $display(\"B=%0d %0d %0d %0d\", b[0],b[1],b[2],b[3]);\n\
           c[-1]=9; c[0]=5; c[1]=7;\n\
           c.sort();  $display(\"C=%0d %0d %0d\", c[-1],c[0],c[1]);\n\
           c.rsort(); $display(\"CR=%0d %0d %0d\", c[-1],c[0],c[1]);\n\
         end endmodule\n");
    assert_eq!(out, "B=1 2 3 4\nC=5 7 9\nCR=9 7 5\n");
}

#[test]
fn signed_sort_matches_the_queue_twin() {
    // ⚠️ A second vita-vs-verilator disagreement that predates this slice:
    // `byte` = {-5, 3, -1}
    //   vita:      -5 -1 3  (signed compare — the array's DECLARED element type)
    //   verilator:  3 -5 -1 (unsigned: 3, 251, 255)
    // IEEE §7.12.2 sorts "in ascending order" using the element type, and `byte`
    // is signed, so vita's order is the LRM one. As above, the load-bearing half
    // is that the queue spelling — `arr_cmp`'s long-standing rule — gives exactly
    // the same order, so the two receivers cannot drift apart.
    let out = run("module top; byte q[$]; byte n[3];\n\
         initial begin\n\
           q.push_back(-8'sd5); q.push_back(8'sd3); q.push_back(-8'sd1);\n\
           n[0]=-8'sd5; n[1]=8'sd3; n[2]=-8'sd1;\n\
           q.sort(); $display(\"Q=%0d %0d %0d\", q[0],q[1],q[2]);\n\
           n.sort(); $display(\"N=%0d %0d %0d\", n[0],n[1],n[2]);\n\
         end endmodule\n");
    assert_eq!(out, "Q=-5 -1 3\nN=-5 -1 3\n");
}

// ── the must-stay-loud set ───────────────────────────────────────────────────

#[test]
fn a_multi_dimensional_fixed_array_stays_loud() {
    // No oracle: iverilog has no fixed-array reduction, and verilator 5.050 emits
    // C++ that does not compile for `int a[2][3]`
    // ("assigning to 'IData' … from 'VlUnpacked<unsigned int, 3>'"). §7.12.3 does
    // not say whether the fold is over the rows or over the leaf elements, which
    // is exactly the ambiguity, so vita refuses instead of guessing.
    assert_loud(
        "module top; int a[2][3]; int s;\n\
         initial begin a[0][0]=1; s = a.sum() with (item); $display(\"%0d\", s); end\n\
         endmodule\n",
        "2-D reduction",
        "MULTI-dimensional",
    );
    assert_loud(
        "module top; int a[2][3];\n initial begin a.sort(); end\nendmodule\n",
        "2-D sort",
        "MULTI-dimensional",
    );
}

#[test]
fn a_real_or_string_element_array_stays_loud() {
    // The fold is 4-state INTEGER arithmetic. Over an f64 net it would answer
    // with the IEEE-754 bit pattern at exit 0 — a silent-wrong, not a gap.
    assert_loud(
        "module top; real r[3]; int s;\n\
         initial begin s = r.sum() with (item); $display(\"%0d\", s); end\nendmodule\n",
        "real reduction",
        "integral elements",
    );
    assert_loud(
        "module top; real r[3];\n initial begin r.sort(); end\nendmodule\n",
        "real sort",
        "integral elements",
    );
}

#[test]
fn a_packed_vector_is_not_an_array_method_receiver() {
    // `logic [3:0] v; v.sum()` — verilator refuses it outright ("Unsupported:
    // Member call on object 'VARREF 'v'' which is a 'BASICDTYPE 'logic''"), and
    // vita must too: the receiver gate asks for DECLARED unpacked dims, which a
    // packed vector has none of.
    assert_loud(
        "module top; logic [3:0] v; int s;\n\
         initial begin v = 4'b1011; s = v.sum() with (item); $display(\"%0d\", s); end\n\
         endmodule\n",
        "packed vector reduction",
        "E3009",
    );
}

#[test]
fn an_ordering_method_on_a_wire_array_is_a_procedural_net_write() {
    // ⚠️ Found by probe, not by review: an ordering method WRITES its receiver,
    // so it is a procedural assignment, and `wire` is illegal there (§6.5). Left
    // ungated, `wire [7:0] w[3]; w.sort();` was accepted at exit 0 and the sort
    // silently vanished under the continuous drivers — while verilator refuses
    // the design (%Error-CONTASSINIT) and vita's own `w[0] = 8'd9;` is E3018.
    // The gate asks `check_lvalue_kind`, the same rule the scalar write asks.
    assert_loud(
        "module top; wire [7:0] w[3];\n\
         assign w[0]=1; assign w[1]=2; assign w[2]=3;\n\
         initial begin #1; w.sort(); end\nendmodule\n",
        "wire array sort",
        "E3018",
    );
}

#[test]
fn a_wire_array_reduction_is_a_pure_read_and_is_allowed() {
    // The twin of the row above: `.sum()` only READS, so the E3018 gate must not
    // fire for it. verilator cannot be asked (it refuses the whole design once a
    // `.sort()` appears), so this is hand-IEEE: 1|2|4 summed is 7.
    let out = run("module top; wire [7:0] w[3]; int s;\n\
         assign w[0]=1; assign w[1]=2; assign w[2]=4;\n\
         initial begin #1; s = w.sum() with (item); $display(\"S=%0d\", s); end\n\
         endmodule\n");
    assert_eq!(out, "S=7\n");
}

#[test]
fn a_module_port_array_reduction_reads_the_connected_value() {
    // A fixed unpacked array arriving through a module port is an ordinary net to
    // the receiver gate. verilator 5.050 on the identical source: PORTSUM=7.
    let out = run("module sub(input logic [7:0] pin [3], output int o);\n\
           always_comb o = pin.sum() with (item);\n\
         endmodule\n\
         module top; logic [7:0] q[3]; int o; sub u(.pin(q), .o(o));\n\
         initial begin q[0]=1; q[1]=2; q[2]=4; #1; $display(\"PORTSUM=%0d\", o); end\n\
         endmodule\n");
    assert_eq!(out, "PORTSUM=7\n");
}

#[test]
fn a_subroutine_local_array_is_still_loud() {
    // NOT shipped, and pinned so the gap is visible rather than assumed closed:
    // a frame-local array does not resolve through `lookup_net_scoped`, so the
    // receiver gate declines and the pre-slice hierarchical-call refusal stands.
    // verilator runs it (FL=6), so this is a real remaining gap — it needs the
    // frame-local storage path, not this slice's receiver resolution.
    assert_loud(
        "module top;\n\
           function automatic int fl();\n\
             int loc[3];\n\
             loc[0]=1; loc[1]=2; loc[2]=3;\n\
             return loc.sum();\n\
           endfunction\n\
           initial $display(\"%0d\", fl());\n\
         endmodule\n",
        "subroutine-local array reduction",
        "E3009",
    );
}

// ── backend parity ───────────────────────────────────────────────────────────

#[test]
fn all_three_backends_agree() {
    // These designs run on the tier-3 NATIVE backend by default (`run.json`:
    // backend "native", eligible+buildable). The engine reads elements through
    // the `NetReader` the evaluation already holds and writes the sort back
    // through the task-write funnel, so both stores must give one answer.
    let src = "module top; int a[4]; int c[-1:1]; int s;\n\
         initial begin\n\
           a[0]=3; a[1]=1; a[2]=4; a[3]=2;\n\
           s = a.sum() with (item.index); $display(\"IDX=%0d\", s);\n\
           a.sort(); $display(\"S=%0d %0d %0d %0d\", a[0],a[1],a[2],a[3]);\n\
           c[-1]=1; c[0]=2; c[1]=3;\n\
           s = c.sum() with (item.index); $display(\"C=%0d\", s);\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fxarr_be_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut seen: Vec<String> = Vec::new();
    for backend in ["interp", "vm", "native"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg("--backend")
            .arg(backend)
            .arg(&path)
            .output()
            .expect("failed to run vita");
        assert!(out.status.success(), "{backend}: vita must succeed");
        let raw = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut s = String::new();
        for l in raw.lines().filter(|l| !l.starts_with("simulation ended")) {
            s.push_str(l);
            s.push('\n');
        }
        assert_eq!(s, "IDX=6\nS=1 2 3 4\nC=0\n", "{backend} value");
        seen.push(s);
    }
    let _ = std::fs::remove_file(&path);
    assert!(seen.windows(2).all(|w| w[0] == w[1]));
}
