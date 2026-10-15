//! §5.2 row 67b: every variable-keyed struct, record and enum binding, and every nested
//! struct member, stores the KEY of the type declaration it bound where it was declared
//! (`hdl-parser/src/decl_key.rs`), and every access reads that declaration's layout. Before
//! this row a binding stored the type NAME and each access looked the name up again where
//! it stood, so a routine, block, generate block, class method or import that re-declared
//! the name between the binding and the access read the inner type's layout, at exit 0.
//!
//! Each cell records PRE (vita 4858082a) and the oracles' raw `A` lines: Icarus Verilog
//! 13.0 (`iverilog -g2012`), sv2v 0.0.13 → iverilog 13.0 where recorded, verilator 5.052
//! (`--binary`). "refuses" = the tool rejects or crashes on the design. A cell pinned at a
//! value no oracle prints says why (hand-IEEE, or KNOWN-WRONG held for its ROADMAP line).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_declkeys_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// The `A …` lines, in order, joined by `|`.
fn lines(out: &str) -> String {
    out.lines()
        .filter(|l| l.starts_with("A "))
        .collect::<Vec<_>>()
        .join("|")
}

fn value(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0:\n{out}");
    assert_eq!(lines(&out), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "{name}: expected exit 1:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

/// §5.2 row 67b targets.
#[test]
fn the_row_67_target_cells_read_the_declaration_their_binding_named() {
    // rv.c571_fn2_lit: PRE A c571f2l fx=1 q=0X; iverilog A c571f2l fx=1 q=10; sv2v refuses; verilator A c571f2l fx=1 q=10.
    value(
        "rv.c571_fn2_lit",
        r##"// §4.5.571 CLASS: a nested member key re-read INSIDE a function after a function-local typedef of the same name; literal part-select
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    return {o.f.x, 2'b00, o.f.x[2:1]};
  endfunction
  initial begin o = 16'h1205; #1 $display("A c571f2l fx=%h q=%h", o.f.x, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571f2l fx=1 q=10",
    );
    // r3.c571_fnvar_lit: PRE A fnvar sa=5 q=63; iverilog A fnvar sa=5 q=52; sv2v refuses; verilator A fnvar sa=5 q=52.
    value(
        "r3.c571_fnvar_lit",
        r##"// row 67 scope: a MODULE variable `st s` read inside a function that declares its own `typedef ... st` (top-level var->type key, not a nested key)
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {s.a, 2'b00, s.a[2:1]};
  endfunction
  initial begin s = 8'h56; #1 $display("A fnvar sa=%h q=%h", s.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A fnvar sa=5 q=52",
    );
    // q2.mod_base_import_between: PRE loud VITA-E3010; iverilog refuses; sv2v refuses; verilator A q2mib ga=0d.
    // hand-IEEE ga=16 (declaration-point binding, IEEE 1800-2017 §26.3; §4.5.592): `st_t s` is
    // declared before `import p::*`, so it is the `$unit` `st_t`; verilator binds scope-wide
    // (§4.5.592: disqualified on this axis)
    value(
        "q2.mod_base_import_between",
        r##"package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
endpackage
typedef struct packed { logic [1:0] b; logic [5:0] a; } st_t;
module m;
  class B; st_t s; endclass
  import p::*;
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display("A q2mib ga=%h", d.ga()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A q2mib ga=16",
    );
}

/// `var_struct`: a member read, write or `'{…}` through a variable of an outer type, inside a routine, block or generate block that re-declares the type name (`struct_field_select`, `build_struct_pattern_concat`).
#[test]
fn a_member_read_or_write_through_a_variable_reads_its_own_declaration() {
    // f1.fn.S.rd: PRE A f1.fn.S.rd w=16 s=56; iverilog A f1.fn.S.rd w=05 s=56; sv2v refuses; verilator A f1.fn.S.rd w=05 s=56.
    value(
        "f1.fn.S.rd",
        r##"// row 67 f1.fn.S.rd: module var `st s` (a[7:4] b[3:0]); fn declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; rd of s inside fn
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b00, s.a};
  endfunction
  initial begin s = 8'h56; #1 w = q(); #1 $display("A f1.fn.S.rd w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.fn.S.rd w=05 s=56",
    );
    // f1.fn.S.wr: PRE A f1.fn.S.wr w=00 s=96; iverilog A f1.fn.S.wr w=00 s=5a; sv2v refuses; verilator A f1.fn.S.wr w=00 s=5a.
    value(
        "f1.fn.S.wr",
        r##"// row 67 f1.fn.S.wr: module var `st s` (a[7:4] b[3:0]); fn declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; wr of s inside fn
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    s.b = 4'hA; return 8'h00;
  endfunction
  initial begin s = 8'h56; #1 w = q(); #1 $display("A f1.fn.S.wr w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.fn.S.wr w=00 s=5a",
    );
    // f1.fn.S.pat: PRE A f1.fn.S.pat w=00 s=c4; iverilog A f1.fn.S.pat w=00 s=34; sv2v refuses; verilator A f1.fn.S.pat w=00 s=34.
    value(
        "f1.fn.S.pat",
        r##"// row 67 f1.fn.S.pat: module var `st s` (a[7:4] b[3:0]); fn declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; pat of s inside fn
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    s = '{4'h3, 4'h4}; return 8'h00;
  endfunction
  initial begin s = 8'h56; #1 w = q(); #1 $display("A f1.fn.S.pat w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.fn.S.pat w=00 s=34",
    );
    // f1.task.U.rd: PRE A f1.task.U.rd w=56 s=56; iverilog A f1.task.U.rd w=05 s=56; sv2v refuses; verilator A f1.task.U.rd w=05 s=56.
    value(
        "f1.task.U.rd",
        r##"// row 67 f1.task.U.rd: module var `st s` (a[7:4] b[3:0]); task declares `typedef union packed { logic [7:0] a; logic [7:0] b; } st;`; rd of s inside task
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  task automatic q(output logic [7:0] o);
    typedef union packed { logic [7:0] a; logic [7:0] b; } st;
    o = {2'b00, s.a};
  endtask
  initial begin s = 8'h56; #1 q(w); #1 $display("A f1.task.U.rd w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.task.U.rd w=05 s=56",
    );
    // f1.blk.S.rd: PRE A f1.blk.S.rd w=16 s=56; iverilog A f1.blk.S.rd w=05 s=56; sv2v refuses; verilator A f1.blk.S.rd w=05 s=56.
    value(
        "f1.blk.S.rd",
        r##"// row 67 f1.blk.S.rd: module var `st s` (a[7:4] b[3:0]); blk declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; rd of s inside blk
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  initial begin : nb
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    #1 w = {2'b00, s.a};
  end
  initial begin s = 8'h56; w = 8'h00; #2 $display("A f1.blk.S.rd w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.blk.S.rd w=05 s=56",
    );
    // f1.gif.S.rd: PRE A f1.gif.S.rd w=16 s=56; iverilog A f1.gif.S.rd w=05 s=56; sv2v A f1.gif.S.rd w=05 s=56; verilator A f1.gif.S.rd w=05 s=56.
    value(
        "f1.gif.S.rd",
        r##"// row 67 f1.gif.S.rd: module var `st s` (a[7:4] b[3:0]); gif declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; rd of s inside gif
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    assign w = {2'b00, s.a};
  end
  initial begin s = 8'h56; #2 $display("A f1.gif.S.rd w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.gif.S.rd w=05 s=56",
    );
    // f1.gfor.U.wr: PRE A f1.gfor.U.wr w=00 s=0a; iverilog A f1.gfor.U.wr w=00 s=5a; sv2v A f1.gfor.U.wr w=00 s=5a; verilator A f1.gfor.U.wr w=00 s=5a.
    value(
        "f1.gfor.U.wr",
        r##"// row 67 f1.gfor.U.wr: module var `st s` (a[7:4] b[3:0]); gfor declares `typedef union packed { logic [7:0] a; logic [7:0] b; } st;`; wr of s inside gfor
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  for (genvar i = 0; i < 1; i++) begin : g
    typedef union packed { logic [7:0] a; logic [7:0] b; } st;
    initial #1 s.b = 4'hA;
  end
  initial begin s = 8'h56; w = 8'h00; #2 $display("A f1.gfor.U.wr w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.gfor.U.wr w=00 s=5a",
    );
    // f1.gcase.S.pat: PRE A f1.gcase.S.pat w=00 s=c4; iverilog A f1.gcase.S.pat w=00 s=34; sv2v A f1.gcase.S.pat w=00 s=34; verilator A f1.gcase.S.pat w=00 s=34.
    value(
        "f1.gcase.S.pat",
        r##"// row 67 f1.gcase.S.pat: module var `st s` (a[7:4] b[3:0]); gcase declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; pat of s inside gcase
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  case (1)
    1: begin : g
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      initial #1 s = '{4'h3, 4'h4};
    end
  endcase
  initial begin s = 8'h56; w = 8'h00; #2 $display("A f1.gcase.S.pat w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.gcase.S.pat w=00 s=34",
    );
    // f1.gif.S.port: PRE A f1.gif.S.port w=16; iverilog A f1.gif.S.port w=05; sv2v A f1.gif.S.port w=05; verilator A f1.gif.S.port w=05.
    value(
        "f1.gif.S.port",
        r##"// row 67 f1.gif.S.port: module var st s; generate block re-declares st (S); s.a as an input port actual
module sub(input logic [5:0] i, output logic [7:0] o); assign o = {2'b00, i}; endmodule
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    sub u(.i(s.a), .o(w));
  end
  initial begin s = 8'h56; #1 $display("A f1.gif.S.port w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.gif.S.port w=05",
    );
    // f1.fn.S.bitsm: PRE A f1.fn.S.bitsm q=6; iverilog A f1.fn.S.bitsm q=4; sv2v refuses; verilator A f1.fn.S.bitsm q=4.
    value(
        "f1.fn.S.bitsm",
        r##"// row 67 f1.fn.S.bitsm: module var st s; function re-declares st (S); returns $bits(s.a)
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s;
  function automatic int q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return $bits(s.a);
  endfunction
  initial begin s = 8'h56; #1 $display("A f1.fn.S.bitsm q=%0d", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f1.fn.S.bitsm q=4",
    );
    // f1.meth.S.rd: PRE loud VITA-E3010; iverilog A f1.meth.S.rd w=05 s=56; sv2v refuses; verilator A f1.meth.S.rd w=05 s=56.
    // loud on both sides (a method of a class reading a module variable; E3010), unchanged
    loud(
        "f1.meth.S.rd",
        r##"// row 67 f1.meth.S.rd: module var `st s` (a[7:4] b[3:0]); meth declares `typedef struct packed { logic [1:0] b; logic [5:0] a; } st;`; rd of s inside meth
module m #(parameter int W = 2);
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st s; logic [7:0] w;
  class K;
    function logic [7:0] q();
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      return {2'b00, s.a};
    endfunction
  endclass
  K k;
  initial begin s = 8'h56; #1 k = new; w = k.q(); #1 $display("A f1.meth.S.rd w=%h s=%h", w, s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
}

/// A nested member (`StructFieldLayout` slot 8): a chain `o.f.x`, an element chain `arr[i].f.x`, a nested `'{…}` and a whole-member `'{…}` (`struct_field_chain`, `extend_member_chain`, `member_pattern_ty`, the pattern recursion), and the `endpackage` twin pass.
#[test]
fn a_nested_member_reads_the_declaration_its_type_named() {
    // f3.fn.S.rd: PRE A f3.fn.S.rd w=0000 o=1205; iverilog A f3.fn.S.rd w=0001 o=1205; sv2v refuses; verilator A f3.fn.S.rd w=0001 o=1205.
    value(
        "f3.fn.S.rd",
        r##"// row 67 f3.fn.S.rd: module var `out_t o` with nested member f:in_t; fn declares `typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;`; rd in fn
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o; logic [15:0] w;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    w = {12'h000, o.f.x};
    return 8'h00;
  endfunction
  initial begin o = 16'h1205; w = 16'h0; #1 void'(q()); #1 $display("A f3.fn.S.rd w=%h o=%h", w, o); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.fn.S.rd w=0001 o=1205",
    );
    // f3.gif.U.wr: PRE A f3.gif.U.wr w=0000 o=0a05; iverilog A f3.gif.U.wr w=0000 o=a205; sv2v A f3.gif.U.wr w=0000 o=a205; verilator A f3.gif.U.wr w=0000 o=a205.
    value(
        "f3.gif.U.wr",
        r##"// row 67 f3.gif.U.wr: module var `out_t o` with nested member f:in_t; gif declares `typedef union packed { logic [7:0] x; logic [7:0] y; } in_t;`; wr in gif
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o; logic [15:0] w;
  if (1) begin : g
    typedef union packed { logic [7:0] x; logic [7:0] y; } in_t;
    initial #1 o.f.x = 4'hA;
  end
  initial begin o = 16'h1205; w = 16'h0; #2 $display("A f3.gif.U.wr w=%h o=%h", w, o); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.gif.U.wr w=0000 o=a205",
    );
    // f3.task.S.npat: PRE A f3.task.S.npat w=0000 o=c405; iverilog A f3.task.S.npat w=0000 o=3405; sv2v refuses; verilator A f3.task.S.npat w=0000 o=3405.
    value(
        "f3.task.S.npat",
        r##"// row 67 f3.task.S.npat: module var `out_t o` with nested member f:in_t; task declares `typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;`; npat in task
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o; logic [15:0] w;
  task automatic q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    o = '{'{4'h3, 4'h4}, 8'h05};
  endtask
  initial begin o = 16'h1205; w = 16'h0; #1 q(); #1 $display("A f3.task.S.npat w=%h o=%h", w, o); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.task.S.npat w=0000 o=3405",
    );
    // f3.blk.U.kpat: PRE A f3.blk.U.kpat w=0000 o=0405; iverilog refuses; sv2v refuses; verilator A f3.blk.U.kpat w=0000 o=3405.
    value(
        "f3.blk.U.kpat",
        r##"// row 67 f3.blk.U.kpat: module var `out_t o` with nested member f:in_t; blk declares `typedef union packed { logic [7:0] x; logic [7:0] y; } in_t;`; kpat in blk
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o; logic [15:0] w;
  initial begin : nb
    typedef union packed { logic [7:0] x; logic [7:0] y; } in_t;
    #1 o = '{f: '{x: 4'h3, y: 4'h4}, g: 8'h05};
  end
  initial begin o = 16'h1205; w = 16'h0; #2 $display("A f3.blk.U.kpat w=%h o=%h", w, o); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.blk.U.kpat w=0000 o=3405",
    );
    // f3.gfor.S.mpat: PRE A f3.gfor.S.mpat w=0000 o=c405; iverilog A f3.gfor.S.mpat w=0000 o=3405; sv2v A f3.gfor.S.mpat w=0000 o=3405; verilator A f3.gfor.S.mpat w=0000 o=3405.
    value(
        "f3.gfor.S.mpat",
        r##"// row 67 f3.gfor.S.mpat: module var `out_t o` with nested member f:in_t; gfor declares `typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;`; mpat in gfor
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o; logic [15:0] w;
  for (genvar i = 0; i < 1; i++) begin : g
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    initial #1 o.f = '{4'h3, 4'h4};
  end
  initial begin o = 16'h1205; w = 16'h0; #2 $display("A f3.gfor.S.mpat w=%h o=%h", w, o); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.gfor.S.mpat w=0000 o=3405",
    );
    // f3.fn.S.depth2: PRE A f3.fn.S.depth2 q=00; iverilog A f3.fn.S.depth2 q=01; sv2v refuses; verilator A f3.fn.S.depth2 q=01.
    value(
        "f3.fn.S.depth2",
        r##"// row 67 f3.fn.S.depth2: nested depth 2: o.f.i.x where the innermost type is re-declared in a function
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } i2_t;
  typedef struct packed { i2_t i; logic [3:0] z; } in_t;
  typedef struct packed { in_t f; logic [3:0] g; } out_t;
  out_t o;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } i2_t;
    return {4'h0, o.f.i.x};
  endfunction
  initial begin o = 16'h1234; #1 $display("A f3.fn.S.depth2 q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.fn.S.depth2 q=01",
    );
    // f3.gif.S.arrelem: PRE A f3.gif.S.arrelem w=00; iverilog refuses; sv2v A f3.gif.S.arrelem w=01; verilator A f3.gif.S.arrelem w=01.
    value(
        "f3.gif.S.arrelem",
        r##"// row 67 f3.gif.S.arrelem: module `out_t arr[2]`; generate block re-declares in_t; reads arr[1].f.x
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t arr [2]; logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    assign w = {4'h0, arr[1].f.x};
  end
  initial begin arr[1] = 16'h1205; #1 $display("A f3.gif.S.arrelem w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.gif.S.arrelem w=01",
    );
    // f3.fn.S.decl_after: PRE A f3.fn.S.decl_after q=00; iverilog A f3.fn.S.decl_after q=01; sv2v refuses; verilator A f3.fn.S.decl_after q=01.
    value(
        "f3.fn.S.decl_after",
        r##"// row 67 f3.fn.S.decl_after: function re-declares in_t, then declares its own `out_t v` (module out_t) and reads v.f.x after v = 16'h1205
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    out_t v;
    v = 16'h1205;
    return {4'h0, v.f.x};
  endfunction
  initial begin #1 $display("A f3.fn.S.decl_after q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.fn.S.decl_after q=01",
    );
    // f3.gif.S.arrwr: PRE A f3.gif.S.arrwr a1=c405; iverilog A f3.gif.S.arrwr a1=0004; sv2v A f3.gif.S.arrwr a1=3405; verilator A f3.gif.S.arrwr a1=3405.
    // iverilog `a1=0004` is its own (the untyped element pattern; sv2v -> iverilog and
    // verilator agree)
    value(
        "f3.gif.S.arrwr",
        r##"// row 67 f3.gif.S.arrwr: module `out_t arr[2]`; generate block re-declares in_t; writes arr[1].f = '{..}
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t arr [2];
  if (1) begin : g
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    initial #1 arr[1].f = '{4'h3, 4'h4};
  end
  initial begin arr[1] = 16'h1205; #2 $display("A f3.gif.S.arrwr a1=%h", arr[1]); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f3.gif.S.arrwr a1=3405",
    );
    // m01: PRE A w=01; iverilog A w=10; sv2v A w=10; verilator A w=10.
    value(
        "m01",
        r##"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  s_t r = 7'b1011001;
  logic [1:0] w;
  if (1) begin : g
    typedef struct packed { logic l; logic [1:0] h; } in_t;
    assign w = r.i.h;
  end
  initial begin #1 $display("A w=%b", w); $finish; end
endmodule
"##,
        "A w=10",
    );
    // m09: PRE A w=0111001; iverilog A w=1011001; sv2v A w=1011001; verilator A w=1011001.
    value(
        "m09",
        r##"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  s_t r; logic [6:0] w;
  if (1) begin : g
    typedef struct packed { logic l; logic [1:0] h; } in_t;
    assign w = {r.i.h, r.i.l, r.z};
  end
  initial begin r = 7'b1011001; #1 $display("A w=%b", w); end
endmodule
"##,
        "A w=1011001",
    );
    // a01u: PRE A r=0011001; iverilog refuses; sv2v A r=1011001; verilator A r=1011001.
    value(
        "a01u",
        r##"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  s_t r;
  if (1) begin : g
    typedef struct packed { logic [3:0] h; logic [1:0] l; } in_t;
    assign r = '{i: '{h: 2'b10, l: 1'b1}, z: 4'h9};
  end
  initial begin #1 $display("A r=%b", r); $finish; end
endmodule
"##,
        "A r=1011001",
    );
    // n04_u: PRE A f=0011001; iverilog A f=1011001; sv2v refuses; verilator A f=1011001.
    value(
        "n04_u",
        r##"module t;
  typedef struct packed { logic [1:0] h; logic l; } in_t;
  typedef struct packed { in_t i; logic [3:0] z; } s_t;
  s_t g;
  function automatic logic [6:0] f();
    typedef struct packed { logic l; logic [1:0] h; } in_t;
    s_t v;
    v = '{'{2'b10, 1'b1}, 4'h9};
    return v;
  endfunction
  initial begin #1 $display("A f=%b", f()); end
endmodule
"##,
        "A f=1011001",
    );
    // g22: PRE A f=12; iverilog A f=02; sv2v refuses; verilator A f=02.
    value(
        "g22",
        r##"module t;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t;
  s_t r;
  function automatic logic [7:0] f();
    typedef struct packed { logic [1:0] a; logic [5:0] b; } s_t;
    return r.b;
  endfunction
  initial begin r = 8'h12; #1 $display("A f=%h", f()); end
endmodule
"##,
        "A f=02",
    );
    // g05.arr_chain_fn: PRE A g05 x=1 q=00; iverilog refuses; sv2v refuses; verilator A g05 x=1 q=01.
    // iverilog asserts (elab_expr.cc:2677) on an array of a struct in a function
    value(
        "g05.arr_chain_fn",
        r##"// 67b grounding: extend_member_chain — `arr[i].f.x` (nested member key) inside a function that declares its own `in_t`
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t arr [0:1];
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    return {4'h0, arr[1].f.x};
  endfunction
  initial begin arr[1] = 16'h1205; #1 $display("A g05 x=%h q=%h", arr[1].f.x, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g05 x=1 q=01",
    );
    // g21.pkg_nested_twin: PRE A g21 x=1; iverilog A g21 x=1; sv2v refuses; verilator A g21 x=1.
    value(
        "g21.pkg_nested_twin",
        r##"// 67b grounding (row 67 "first:"): the endpackage twin pass for a nested key — `p::out_t` imported explicitly into a module that declares its own `in_t`
package p;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
endpackage
module m;
  import p::out_t;
  typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
  out_t o;
  initial begin o = 16'h1205; #1 $display("A g21 x=%h", o.f.x); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g21 x=1",
    );
    // g21b.pkg_nested_twin_fn: PRE A g21b x=1 q=01; iverilog A g21b x=1 q=01; sv2v refuses; verilator A g21b x=1 q=01.
    value(
        "g21b.pkg_nested_twin_fn",
        r##"// 67b grounding: the endpackage twin pass for a nested key — `p::out_t o` read `o.f.x` in a function declaring its own bare `in_t`
package p;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
endpackage
module m;
  import p::*;
  out_t o;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    return {4'h0, o.f.x};
  endfunction
  initial begin o = 16'h1205; #1 $display("A g21b x=%h q=%h", o.f.x, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g21b x=1 q=01",
    );
    // g31.typed_union_nested_redecl: PRE loud VITA-E2002; iverilog refuses; sv2v A g31 w=67; verilator A g31 w=67.
    // a typed pattern's nested member keeps its declared union type (refused as a union member,
    // like the control g32); before row 67b the whole pattern was refused (row 36's nested
    // re-declaration check)
    loud(
        "g31.typed_union_nested_redecl",
        r##"// 67b grounding: a typed pattern whose nested member type is a union, re-declared as a packed struct in a generate block before the pattern
module m;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] a; logic [1:0] b; } u_t;
    assign w = s_t'{u: '{4'h5, 4'h6}, c: 4'h7};
  end
  initial begin #1 $display("A g31 w=%h", w); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E2002]",
    );
    // g32.typed_union_nested_ctl: PRE loud VITA-E2002; iverilog refuses; sv2v A g32 w=67; verilator A g32 w=67.
    // control: nothing re-declared; the union-member refusal of row 36
    loud(
        "g32.typed_union_nested_ctl",
        r##"// 67b grounding control: the same typed pattern with the generate block's struct renamed (nothing re-declared)
module m;
  typedef union packed { logic [3:0] a; logic [3:0] b; } u_t;
  typedef struct packed { u_t u; logic [3:0] c; } s_t;
  logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] a; logic [1:0] b; } v_t;
    assign w = s_t'{u: '{4'h5, 4'h6}, c: 4'h7};
  end
  initial begin #1 $display("A g32 w=%h", w); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E2002]",
    );
}

/// An element of a struct array or queue (`struct_array_field_geom`, the element and whole-array `'{…}`, `push_back('{…})`).
#[test]
fn an_array_element_member_reads_its_own_declaration() {
    // g04.arr_fn: PRE A g04 a=5 q=58; iverilog refuses; sv2v refuses; verilator A g04 a=5 q=14.
    // iverilog asserts (elab_expr.cc:2677) on an array of a struct in a function
    value(
        "g04.arr_fn",
        r##"// 67b grounding: struct_array_field_geom — a module `st arr[2]` read `arr[i].a` inside a function that declares its own `st`
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st arr [0:1];
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b00, arr[1].a, 2'b00};
  endfunction
  initial begin arr[1] = 8'h56; #1 $display("A g04 a=%h q=%h", arr[1].a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g04 a=5 q=14",
    );
    // x.arr_fn: PRE A x.arr_fn q=16; iverilog refuses; sv2v refuses; verilator A x.arr_fn q=05.
    value(
        "x.arr_fn",
        r##"// row 67 x.arr_fn: module `st arr[2]`; function declares its own st and reads arr[1].a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st arr [2];
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, arr[1].a};
  endfunction
  initial begin arr[1] = 8'h56; #1 $display("A x.arr_fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.arr_fn q=05",
    );
    // x.arr_pat_fn: PRE A x.arr_pat_fn a1=c4; iverilog A x.arr_pat_fn a1=34; sv2v refuses; verilator A x.arr_pat_fn a1=34.
    value(
        "x.arr_pat_fn",
        r##"// row 67 x.arr_pat_fn: module `st arr[2]`; function declares its own st and writes arr[1] = '{..}
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st arr [2];
  function automatic void q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    arr[1] = '{4'h3, 4'h4};
  endfunction
  initial begin q(); #1 $display("A x.arr_pat_fn a1=%h", arr[1]); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.arr_pat_fn a1=34",
    );
    // x.parr_fn: PRE A x.parr_fn q=16; iverilog A x.parr_fn q=05; sv2v refuses; verilator A x.parr_fn q=05.
    value(
        "x.parr_fn",
        r##"// row 67 x.parr_fn: module `st [1:0] pa`; function declares its own st and reads pa[1].a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st [1:0] pa;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, pa[1].a};
  endfunction
  initial begin pa = 16'h5600; #1 $display("A x.parr_fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.parr_fn q=05",
    );
    // x.queue_fn: PRE A x.queue_fn q0=c4; iverilog refuses; sv2v refuses; verilator A x.queue_fn q0=34.
    value(
        "x.queue_fn",
        r##"// row 67 x.queue_fn: module queue `st q[$]`; function declares its own st and pushes '{..}
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st qq [$];
  function automatic void q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    qq.push_back('{4'h3, 4'h4});
  endfunction
  initial begin q(); #1 $display("A x.queue_fn q0=%h", qq[0]); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.queue_fn q0=34",
    );
    // g15.push_fn: PRE A g15 q0=c4; iverilog refuses; sv2v refuses; verilator A g15 q0=34.
    value(
        "g15.push_fn",
        r##"// 67b grounding: desugar_container_pattern_args — `q.push_back('{…})` on a module struct queue inside a function declaring its own `st`
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  st qu [$];
  function automatic void p();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    qu.push_back('{4'h3, 4'h4});
  endfunction
  initial begin p(); #1 $display("A g15 q0=%h", qu[0]); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g15 q0=34",
    );
}

/// A per-instance struct (`sym_struct_field_select`, `sym_struct_array_field_geom`).
#[test]
fn a_per_instance_struct_member_reads_its_own_declaration() {
    // g06.sym_fn: PRE A g06 a=5 q=16; iverilog A g06 a=5 q=05; sv2v refuses; verilator A g06 a=5 q=05.
    value(
        "g06.sym_fn",
        r##"// 67b grounding: sym_struct_field_select — a per-instance struct variable read inside a function declaring its own `st`
module m #(parameter W = 4);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  st s;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {4'h0, s.a};
  endfunction
  initial begin s = 8'h56; #1 $display("A g06 a=%h q=%h", s.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g06 a=5 q=05",
    );
    // g07.sym_arr_fn: PRE A g07 a=5 q=16; iverilog refuses; sv2v refuses; verilator A g07 a=5 q=05.
    value(
        "g07.sym_arr_fn",
        r##"// 67b grounding: sym_struct_array_field_geom — `arr[i].a` of a per-instance struct array inside a function declaring its own `st`
module m #(parameter W = 4);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  st arr [0:1];
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {4'h0, arr[1].a};
  endfunction
  initial begin arr[1] = 8'h56; #1 $display("A g07 a=%h q=%h", arr[1].a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g07 a=5 q=05",
    );
    // x.sym_fn: PRE A x.sym_fn q=16; iverilog A x.sym_fn q=05; sv2v refuses; verilator A x.sym_fn q=05.
    value(
        "x.sym_fn",
        r##"// row 67 x.sym_fn: module per-instance struct `st` (W header); module var `st s`; function declares a flat st; reads s.a
module m #(parameter int W = 4);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  st s;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, s.a};
  endfunction
  initial begin s = 8'h56; #1 $display("A x.sym_fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.sym_fn q=05",
    );
    // g34.sym_over_sym_fn: PRE A g34 a=5 q=16; iverilog A g34 a=5 q=05; sv2v refuses; verilator A g34 a=5 q=05.
    // the inner `st` is laid out per instance too, so the name re-read reaches the per-instance
    // map (g06's inner packed `st` leaves it alone)
    value(
        "g34.sym_over_sym_fn",
        r##"// 67b grounding: a per-instance struct variable read in a function that declares its own per-instance `st`
module m #(parameter W = 4, parameter V = 2);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  st s;
  function automatic logic [7:0] q();
    typedef struct packed { logic [V-1:0] b; logic [5:0] a; } st;
    return {4'h0, s.a};
  endfunction
  initial begin s = 8'h56; #1 $display("A g34 a=%h q=%h", s.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g34 a=5 q=05",
    );
    // g35.sym_arr_over_sym_fn: PRE A g35 a=5 q=16; iverilog refuses; sv2v refuses; verilator A g35 a=5 q=05.
    // iverilog asserts (elab_expr.cc:2677) on an array of a struct in a function
    value(
        "g35.sym_arr_over_sym_fn",
        r##"// 67b grounding: `arr[i].a` of a per-instance struct array read in a function that declares its own per-instance `st`
module m #(parameter W = 4, parameter V = 2);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  st arr [0:1];
  function automatic logic [7:0] q();
    typedef struct packed { logic [V-1:0] b; logic [5:0] a; } st;
    return {4'h0, arr[1].a};
  endfunction
  initial begin arr[1] = 8'h56; #1 $display("A g35 a=%h q=%h", arr[1].a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g35 a=5 q=05",
    );
}

/// Unpacked records: a packable record through `var_struct`, `record_array_vars`, `record_soa_vars`, `var_unpacked_struct` (member nets, call-argument expansion, `foreach`, `pop_front`).
#[test]
fn a_record_variable_reads_its_own_declaration() {
    // g08.rec_fn: PRE A g08 a=5 q=16; iverilog refuses; sv2v refuses; verilator A g08 a=5 q=05.
    value(
        "g08.rec_fn",
        r##"// 67b grounding: struct_field_select through a packable record (`var_struct` -> record) inside a function declaring its own record `rt`
module m;
  typedef struct { logic [3:0] a; logic [3:0] b; } rt;
  rt r;
  function automatic logic [7:0] q();
    typedef struct { logic [1:0] b; logic [5:0] a; } rt;
    return {4'h0, r.a};
  endfunction
  initial begin r.a = 4'h5; r.b = 4'h6; #1 $display("A g08 a=%h q=%h", r.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g08 a=5 q=05",
    );
    // x.rec_fn: PRE A x.rec_fn q=16; iverilog refuses; sv2v refuses; verilator A x.rec_fn q=05.
    value(
        "x.rec_fn",
        r##"// row 67 x.rec_fn: module packable record `st` and fn-local `st r` (record); function declares a packed st and reads... record var in module + function packed st reading r.a
module m;
  typedef struct { logic [3:0] a; logic [3:0] b; } st;
  st r;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {4'h0, r.a};
  endfunction
  initial begin r.a = 4'h5; r.b = 4'h6; #1 $display("A x.rec_fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.rec_fn q=05",
    );
    // g09.recarr_fn: PRE A g09 a=5 q=16; iverilog refuses; sv2v refuses; verilator A g09 a=5 q=05.
    value(
        "g09.recarr_fn",
        r##"// 67b grounding: record_array_vars — `arr[i].a` of a dynamic array of a packable record inside a function declaring its own `rt`
module m;
  typedef struct { logic [3:0] a; logic [3:0] b; } rt;
  rt arr [];
  function automatic logic [7:0] q();
    typedef struct { logic [1:0] b; logic [5:0] a; } rt;
    return {4'h0, arr[0].a};
  endfunction
  initial begin arr = new[1]; arr[0].a = 4'h5; arr[0].b = 4'h6; #1 $display("A g09 a=%h q=%h", arr[0].a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g09 a=5 q=05",
    );
    // g26.recarr_wr_fn: PRE A g26 a=4 b=9; iverilog refuses; sv2v refuses; verilator A g26 a=9 b=6.
    value(
        "g26.recarr_wr_fn",
        r##"// 67b grounding: parse_record_array_member_lval — `arr[i].a = …` on a dyn array of a packable record inside a function declaring its own `rt`
module m;
  typedef struct { logic [3:0] a; logic [3:0] b; } rt;
  rt arr [];
  function automatic void p();
    typedef struct { logic [1:0] b; logic [5:0] a; } rt;
    arr[0].a = 4'h9;
  endfunction
  initial begin arr = new[1]; arr[0].a = 4'h5; arr[0].b = 4'h6; p(); #1 $display("A g26 a=%h b=%h", arr[0].a, arr[0].b); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g26 a=9 b=6",
    );
    // g10.soa_fn: PRE A g10 a=5 q=05; iverilog refuses; sv2v refuses; verilator A g10 a=5 q=05.
    value(
        "g10.soa_fn",
        r##"// 67b grounding: record_soa_vars — `arr[i].a` of a queue of a non-packable record inside a function declaring its own `rt`
module m;
  typedef struct { string s; logic [3:0] a; } rt;
  rt arr [$];
  function automatic logic [7:0] q();
    typedef struct { logic [3:0] a; string s; logic [3:0] c; } rt;
    return {4'h0, arr[0].a};
  endfunction
  initial begin rt e; e.s = "x"; e.a = 4'h5; arr.push_back(e); #1 $display("A g10 a=%h q=%h", arr[0].a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g10 a=5 q=05",
    );
    // x.soa_fn_rec: PRE loud VITA-E2002; iverilog refuses; sv2v refuses; verilator A x.soa_fn_rec f=05.
    value(
        "x.soa_fn_rec",
        r##"// row 67 x.soa_fn_rec: module SoA record dyn array `st q[]`; a function re-declares st as a record WITHOUT member a; reads q[0].a
module m;
  typedef struct { logic [3:0] a; string s; } st;
  st q [];
  function automatic logic [7:0] f();
    typedef struct { logic [7:0] z; string s; } st;
    return {4'h0, q[0].a};
  endfunction
  initial begin q = new[1]; q[0].a = 4'h5; q[0].s = "x"; #1 $display("A x.soa_fn_rec f=%h", f()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.soa_fn_rec f=05",
    );
    // x.recnp_fn_rec: PRE loud VITA-E3010; iverilog refuses; sv2v refuses; verilator A x.recnp_fn_rec q=05.
    value(
        "x.recnp_fn_rec",
        r##"// row 67 x.recnp_fn_rec: module NON-packable record `st` scalar; a function re-declares st as a record with a different member set and reads r.a
module m;
  typedef struct { logic [3:0] a; string s; } st;
  st r;
  function automatic logic [7:0] q();
    typedef struct { logic [7:0] z; string s; } st;
    return {4'h0, r.a};
  endfunction
  initial begin r.a = 4'h5; r.s = "x"; #1 $display("A x.recnp_fn_rec q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.recnp_fn_rec q=05",
    );
    // g11.recnp_call_fn: PRE A g11 f=56 q=65; iverilog refuses; sv2v refuses; verilator A g11 f=56 q=56.
    value(
        "g11.recnp_call_fn",
        r##"// 67b grounding: var_unpacked_struct call-argument expansion — `f(k)` of a non-packable record inside a function declaring its own `rt` (members reordered)
module m;
  typedef struct { string s; logic [3:0] a; logic [3:0] b; } rt;
  rt k;
  function automatic logic [7:0] f(input rt v); return {v.a, v.b}; endfunction
  function automatic logic [7:0] q();
    typedef struct { string s; logic [3:0] b; logic [3:0] a; } rt;
    return f(k);
  endfunction
  initial begin k.s = "x"; k.a = 4'h5; k.b = 4'h6; #1 $display("A g11 f=%h q=%h", f(k), q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g11 f=56 q=56",
    );
    // g11b.recnp_field_fn: PRE loud VITA-E3010; iverilog refuses; sv2v refuses; verilator A g11b a=5 q=05.
    value(
        "g11b.recnp_field_fn",
        r##"// 67b grounding: var_unpacked_struct `k.f` — a non-packable record member read inside a function whose own `rt` lacks the member
module m;
  typedef struct { string s; logic [3:0] a; } rt;
  rt k;
  function automatic logic [7:0] q();
    typedef struct { string s; logic [3:0] c; } rt;
    return {4'h0, k.a};
  endfunction
  initial begin k.s = "x"; k.a = 4'h5; #1 $display("A g11b a=%h q=%h", k.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g11b a=5 q=05",
    );
    // g22.foreach_soa_fn: PRE A g22 n=10; iverilog refuses; sv2v refuses; verilator A g22 n=10.
    value(
        "g22.foreach_soa_fn",
        r##"// 67b grounding: parse_foreach over a SoA record queue inside a function declaring its own `rt` (members reordered)
module m;
  typedef struct { string s; logic [3:0] a; } rt;
  rt arr [$];
  function automatic int q();
    typedef struct { logic [3:0] a; string s; } rt;
    int n = 0;
    foreach (arr[i]) n += arr[i].a;
    return n;
  endfunction
  initial begin rt e; e.s = "x"; e.a = 4'h5; arr.push_back(e); arr.push_back(e); #1 $display("A g22 n=%0d", q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g22 n=10",
    );
    // g23.soa_pop_fn: PRE loud VITA-E3009,VITA-E3010; iverilog refuses; sv2v refuses; verilator A g23 a=5.
    value(
        "g23.soa_pop_fn",
        r##"// 67b grounding: try_soa_assign — `k = arr.pop_front()` (record identity by type) inside a function declaring its own `rt`
module m;
  typedef struct { string s; logic [3:0] a; } rt;
  rt arr [$];
  rt k;
  function automatic void p();
    typedef struct { logic [3:0] a; string s; logic [3:0] c; } rt;
    k = arr.pop_front();
  endfunction
  initial begin rt e; e.s = "x"; e.a = 4'h5; arr.push_back(e); p(); #1 $display("A g23 a=%h", k.a); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g23 a=5",
    );
}

/// `var_enum`: `.next()`, `.name()`, `.next(N)` on a variable of an outer enum type.
#[test]
fn an_enum_method_reads_the_labels_its_variable_bound() {
    // n.fn: PRE A n.fn q=00; iverilog A n.fn q=02; sv2v refuses; verilator A n.fn q=02.
    value(
        "n.fn",
        r##"// row 67 n.fn: module enum var `et e`; a function declares its own enum `et` (other labels); e.name() and e.next() inside the function
module m;
  typedef enum logic [1:0] {A0, A1, A2} et;
  et e;
  function automatic logic [7:0] q();
    typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} et;
    return {6'h0, e.next()};
  endfunction
  initial begin e = A1; #1 $display("A n.fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A n.fn q=02",
    );
    // n.fn_name: PRE A n.fn_name q=B1; iverilog A n.fn_name q=A1; sv2v refuses; verilator A n.fn_name q=A1.
    value(
        "n.fn_name",
        r##"// row 67 n.fn_name: module enum var `et e`; a function declares its own enum `et`; returns e.name()
module m;
  typedef enum logic [1:0] {A0, A1, A2} et;
  et e;
  function automatic string q();
    typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} et;
    return e.name();
  endfunction
  initial begin e = A1; #1 $display("A n.fn_name q=%s", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A n.fn_name q=A1",
    );
    // n.gen: PRE A n.gen w=0; iverilog refuses; sv2v refuses; verilator A n.gen w=2.
    value(
        "n.gen",
        r##"// row 67 n.gen: module enum var `et e`; a generate block declares its own enum `et`; assign w = e.next()
module m;
  typedef enum logic [1:0] {A0, A1, A2} et;
  et e; logic [1:0] w;
  if (1) begin : g
    typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} et;
    assign w = e.next();
  end
  initial begin e = A1; #1 $display("A n.gen w=%0d", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A n.gen w=2",
    );
    // n.param_fn: PRE A n.param_fn q=0; iverilog refuses; sv2v refuses; verilator A n.param_fn q=2.
    value(
        "n.param_fn",
        r##"// row 67 n.param_fn: module `localparam et P = A1`; a function re-declares et; returns P.next()
module m;
  typedef enum logic [1:0] {A0, A1, A2} et;
  localparam et P = A1;
  function automatic logic [1:0] q();
    typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} et;
    return P.next();
  endfunction
  initial begin #1 $display("A n.param_fn q=%0d", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A n.param_fn q=2",
    );
    // g17.enum_nextn_fn: PRE A g17 n=3 q=0; iverilog A g17 n=3 q=3; sv2v refuses; verilator A g17 n=3 q=3.
    value(
        "g17.enum_nextn_fn",
        r##"// 67b grounding: enum_step_n_expr — `e.next(2)` on a module enum variable inside a function declaring its own `et`
module m;
  typedef enum logic [1:0] { A0, A1, A2, A3 } et;
  et e;
  function automatic logic [1:0] q();
    typedef enum logic [1:0] { B0 = 1, B1 = 2, B2 = 0, B3 = 3 } et;
    return e.next(2);
  endfunction
  initial begin e = A1; #1 $display("A g17 n=%0d q=%0d", e.next(2), q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g17 n=3 q=3",
    );
}

/// Ports, parameters and formals bound to an outer type, read inside a scope that re-declares it.
#[test]
fn a_port_parameter_or_formal_binding_reads_its_own_declaration() {
    // x.port_fn: PRE A x.port_fn w=16; iverilog A x.port_fn w=05; sv2v refuses; verilator A x.port_fn w=05.
    value(
        "x.port_fn",
        r##"// row 67 x.port_fn: unit struct st; module ANSI `input st pi`; a function declares its own st and reads pi.a
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module c(input st pi, output logic [7:0] w);
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, pi.a};
  endfunction
  assign w = q();
endmodule
module top;
  logic [7:0] w;
  c u(.pi(8'h56), .w(w));
  initial begin #1 $display("A x.port_fn w=%h", w); #1 $finish; end
endmodule
"##,
        "A x.port_fn w=05",
    );
    // g33.sym_outer_packed_inner: PRE A g33 q=16; iverilog A g33 q=16; sv2v refuses; verilator A g33 q=16.
    // a per-instance module `st` and a function-local packed `st` both stand under the name:
    // the binding takes the packed entry, the one a member select reads first
    // (`var_struct_ref`); which entry the name should denote is ROADMAP §2
    // struct-binding-by-name
    value(
        "g33.sym_outer_packed_inner",
        r##"// 67b grounding: a per-instance module `st` and a function-local packed `st`; a function-local `st v` binds the packed one (the reader order where both entries stand)
module m #(parameter W = 4);
  typedef struct packed { logic [W-1:0] a; logic [3:0] b; } st;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    st v;
    v = 8'h56;
    return {2'b00, v.a};
  endfunction
  initial begin #1 $display("A g33 q=%h", q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g33 q=16",
    );
    // x.port_gen: PRE A x.port_gen w=16; iverilog A x.port_gen w=05; sv2v A x.port_gen w=05; verilator A x.port_gen w=05.
    value(
        "x.port_gen",
        r##"// row 67 x.port_gen: unit struct st; module ANSI `input st pi`; a generate block declares its own st and reads pi.a
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module c(input st pi, output logic [7:0] w);
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    assign w = {2'b0, pi.a};
  end
endmodule
module top;
  logic [7:0] w;
  c u(.pi(8'h56), .w(w));
  initial begin #1 $display("A x.port_gen w=%h", w); #1 $finish; end
endmodule
"##,
        "A x.port_gen w=05",
    );
    // x.nonansi_port_gen: PRE A x.nonansi_port_gen w=16; iverilog A x.nonansi_port_gen w=05; sv2v A x.nonansi_port_gen w=05; verilator A x.nonansi_port_gen w=05.
    value(
        "x.nonansi_port_gen",
        r##"// row 67 x.nonansi_port_gen: unit struct st; non-ANSI `input st pi;` port; a generate block re-declares st and reads pi.a
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module c(pi, w);
  input st pi;
  output logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    assign w = {2'b0, pi.a};
  end
endmodule
module top;
  logic [7:0] w;
  c u(.pi(8'h56), .w(w));
  initial begin #1 $display("A x.nonansi_port_gen w=%h", w); #1 $finish; end
endmodule
"##,
        "A x.nonansi_port_gen w=05",
    );
    // x.param_fn: PRE A x.param_fn q=16; iverilog refuses; sv2v refuses; verilator A x.param_fn q=05.
    value(
        "x.param_fn",
        r##"// row 67 x.param_fn: module `localparam st P = 8'h56`; a function re-declares st and returns P.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  localparam st P = 8'h56;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, P.a};
  endfunction
  initial begin #1 $display("A x.param_fn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.param_fn q=05",
    );
    // x.param_gen: PRE A x.param_gen w=16; iverilog refuses; sv2v A x.param_gen w=05; verilator A x.param_gen w=05.
    value(
        "x.param_gen",
        r##"// row 67 x.param_gen: module `localparam st P = 8'h56`; a generate block re-declares st and reads P.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  localparam st P = 8'h56;
  logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    assign w = {2'b0, P.a};
  end
  initial begin #1 $display("A x.param_gen w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.param_gen w=05",
    );
    // x.param_arr_gen: PRE A x.param_arr_gen w=16; iverilog refuses; sv2v A x.param_arr_gen w=05; verilator A x.param_arr_gen w=05.
    value(
        "x.param_arr_gen",
        r##"// row 67 x.param_arr_gen: module `localparam st P [2] = '{8'h12, 8'h56}`; a generate block re-declares st and reads P[1].a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  localparam st P [2] = '{8'h12, 8'h56};
  logic [7:0] w;
  if (1) begin : g
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    assign w = {2'b0, P[1].a};
  end
  initial begin #1 $display("A x.param_arr_gen w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.param_arr_gen w=05",
    );
    // x.fn_formal_blk: PRE A x.fn_formal_blk q=16; iverilog A x.fn_formal_blk q=05; sv2v refuses; verilator A x.fn_formal_blk q=05.
    value(
        "x.fn_formal_blk",
        r##"// row 67 x.fn_formal_blk: function formal `st x`; a begin block INSIDE the body re-declares st (nested scope) and reads x.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  function automatic logic [7:0] q(st x);
    logic [7:0] r;
    begin : nb
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      r = {2'b0, x.a};
    end
    return r;
  endfunction
  initial begin #1 $display("A x.fn_formal_blk q=%h", q(8'h56)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.fn_formal_blk q=05",
    );
    // x.task_formal_blk: PRE A x.task_formal_blk w=16; iverilog A x.task_formal_blk w=05; sv2v refuses; verilator A x.task_formal_blk w=05.
    value(
        "x.task_formal_blk",
        r##"// row 67 x.task_formal_blk: task formal `input st x`; a begin block inside re-declares st and writes o = x.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  logic [7:0] w;
  task automatic q(input st x, output logic [7:0] o);
    begin : nb
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      o = {2'b0, x.a};
    end
  endtask
  initial begin #1 q(8'h56, w); $display("A x.task_formal_blk w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.task_formal_blk w=05",
    );
}

/// Package variable bindings replayed by an import (`PkgBindings`).
#[test]
fn a_package_binding_reads_its_own_declaration() {
    // p.imp_var_fn: PRE A p.imp_var_fn q=16 a=5; iverilog A p.imp_var_fn q=05 a=5; sv2v refuses; verilator A p.imp_var_fn q=05 a=5.
    value(
        "p.imp_var_fn",
        r##"// row 67 p.imp_var_fn: module imports p::* (p exports struct st); module var `st s`; a function declares its own `st` and reads s.a
package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
endpackage
module m;
  import p::*;
  st s;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, s.a};
  endfunction
  initial begin s = 8'h56; #1 $display("A p.imp_var_fn q=%h a=%h", q(), s.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A p.imp_var_fn q=05 a=5",
    );
    // p.nested_pkgfn: PRE A p.nested_pkgfn q=00; iverilog A p.nested_pkgfn q=01; sv2v refuses; verilator A p.nested_pkgfn q=01.
    value(
        "p.nested_pkgfn",
        r##"// row 67 p.nested_pkgfn: inside the package: package var `out_t po`; a package function declares its own in_t and reads po.f.x
package p;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t po;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    return {4'h0, po.f.x};
  endfunction
endpackage
module m;
  import p::*;
  initial begin po = 16'h1205; #1 $display("A p.nested_pkgfn q=%h", q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A p.nested_pkgfn q=01",
    );
    // g29.pkgvar_fn: PRE A g29 a=5 q=05; iverilog refuses; sv2v refuses; verilator A g29 a=5 q=05.
    value(
        "g29.pkgvar_fn",
        r##"// 67b grounding: an imported package struct variable (PkgBindings replay) read in a function declaring its own `st`
package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  parameter st P = 8'h56;
endpackage
module m;
  import p::*;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {4'h0, P.a};
  endfunction
  initial begin #1 $display("A g29 a=%h q=%h", P.a, q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g29 a=5 q=05",
    );
}

/// Class records (`ClassRec`) and class methods: an inherited property keeps the type its class declared it with.
#[test]
fn a_class_property_reads_the_declaration_its_class_named() {
    // g20.cls_meth_fnlocal: PRE A g20 q=16; iverilog `q=00` after its own "sorry: Nested member
    // path not yet supported for class properties" (not an oracle answer); sv2v refuses;
    // verilator A g20 q=05 = hand-IEEE (`s` is the property of `K`, declared with the module `st`).
    value(
        "g20.cls_meth_fnlocal",
        r##"// 67b grounding (row 67 "first:"): a class method with a function-local typedef of its property's type name reads the property
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  class K;
    st s;
    function logic [7:0] q();
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      return {4'h0, s.a};
    endfunction
  endclass
  K k;
  initial begin k = new; k.s = 8'h56; #1 $display("A g20 q=%h", k.q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g20 q=05",
    );
    // g28.cls_extends_fnlocal: PRE A g28 q=16; iverilog `q=00` after its own "sorry: Nested member
    // path not yet supported for class properties" (not an oracle answer); sv2v refuses;
    // verilator A g28 q=05 = hand-IEEE.
    value(
        "g28.cls_extends_fnlocal",
        r##"// 67b grounding: an `extends` replay of a base record into a derived class whose method declares its own `st`
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  class B; st s; endclass
  class D extends B;
    function logic [7:0] q();
      typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
      return {4'h0, s.a};
    endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'h56; #1 $display("A g28 q=%h", d.q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g28 q=05",
    );
    // g30.cls_enum_fnlocal: PRE A g30 q=0; iverilog refuses; sv2v refuses; verilator A g30 q=2.
    value(
        "g30.cls_enum_fnlocal",
        r##"// 67b grounding: a class enum property's `.next()` in a method declaring its own `et`
module m;
  typedef enum logic [1:0] { A0, A1, A2, A3 } et;
  class K;
    et e;
    function logic [1:0] q();
      typedef enum logic [1:0] { B0 = 3, B1 = 2, B2 = 1, B3 = 0 } et;
      return e.next();
    endfunction
  endclass
  K k;
  initial begin k = new; k.e = A1; #1 $display("A g30 q=%0d", k.q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A g30 q=2",
    );
    // fp.base_mod_import_between: PRE loud VITA-E3010; iverilog refuses; sv2v refuses; verilator A fp.base_mod_import_between ga=0d.
    // hand-IEEE ga=16 (declaration-point binding, §4.5.592); verilator binds scope-wide
    value(
        "fp.base_mod_import_between",
        r##"typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
package q; typedef struct packed { logic [3:0] a; logic [3:0] b; } st2; endpackage
module m;
  class B; st2 s; endclass
  import q::*;
  class D extends B; function logic [5:0] ga(); return s.a; endfunction endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display("A fp.base_mod_import_between ga=%h", d.ga()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A fp.base_mod_import_between ga=16",
    );
    // fp.base_mod_import_between_pat: PRE loud VITA-E3009; iverilog A fp.base_mod_import_between_pat ds=42; sv2v refuses; verilator A fp.base_mod_import_between_pat ds=12.
    // declaration-point binding (§4.5.592): iverilog = hand-IEEE; verilator binds scope-wide
    value(
        "fp.base_mod_import_between_pat",
        r##"typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
package q; typedef struct packed { logic [3:0] a; logic [3:0] b; } st2; endpackage
module m;
  class B; st2 s; endclass
  import q::*;
  class D extends B; function void sp(); s = '{2'h1, 6'h2}; endfunction endclass
  D d;
  initial begin d = new; d.sp(); #1 $display("A fp.base_mod_import_between_pat ds=%h", d.s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A fp.base_mod_import_between_pat ds=42",
    );
    // fp.two_state_member_diff: PRE loud VITA-E3009; iverilog A fp.two_state_member_diff ds=x1000010; sv2v refuses; verilator A fp.two_state_member_diff ds=01000010.
    // a 4-state member keeps `x` (IEEE 1800-2017 §7.2.1): iverilog; verilator is 2-state
    value(
        "fp.two_state_member_diff",
        r##"typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
class B; st2 s; endclass
module m;
  typedef struct packed { bit [1:0] b; bit [5:0] a; } st2;
  class D extends B; function void sp(); s = '{2'bx1, 6'h2}; endfunction endclass
  D d;
  initial begin d = new; d.sp(); #1 $display("A fp.two_state_member_diff ds=%b", d.s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A fp.two_state_member_diff ds=x1000010",
    );
    // q2.top_base_mod_typedef_pat: PRE loud VITA-E3009; iverilog A q2tp s=42; sv2v refuses; verilator A q2tp s=42.
    value(
        "q2.top_base_mod_typedef_pat",
        r##"typedef struct packed { logic [1:0] b; logic [5:0] a; } st_t;
class B; st_t s; endclass
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
  class D extends B;
    function void sp(); s = '{2'h1, 6'h2}; endfunction
  endclass
  D d;
  initial begin d = new; d.sp(); #1 $display("A q2tp s=%h", d.s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A q2tp s=42",
    );
}

/// A variable declared before the same scope re-declares or imports its type name keeps the type it was declared with: declaration-point binding (IEEE 1800-2017 §26.3; ROADMAP §4.5.592 — Icarus Verilog and the standard decide forward references; verilator and sv2v bind scope-wide and are not oracles on this axis).
#[test]
fn a_variable_declared_before_a_redeclaration_keeps_its_declaration() {
    // u.mod_redecl_after: PRE A u.mod_redecl_after a=16; iverilog A u.mod_redecl_after a=5; sv2v A u.mod_redecl_after a=16; verilator refuses.
    // declaration-point binding (§4.5.592): `st s` precedes the module `st`; iverilog =
    // hand-IEEE, sv2v binds scope-wide, verilator refuses
    value(
        "u.mod_redecl_after",
        r##"// row 67 u.mod_redecl_after: unit struct `st`; module var `st s` then module-level `typedef struct ... st` (other layout) AFTER the var; s.a
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module m;
  st s;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
  initial begin s = 8'h56; #1 $display("A u.mod_redecl_after a=%h", s.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A u.mod_redecl_after a=5",
    );
    // u.mod_redecl_after_pat: PRE A u.mod_redecl_after_pat s=c4; iverilog A u.mod_redecl_after_pat s=34; sv2v A u.mod_redecl_after_pat s=34; verilator refuses.
    // declaration-point binding (§4.5.592); verilator refuses
    value(
        "u.mod_redecl_after_pat",
        r##"// row 67 u.mod_redecl_after_pat: unit struct st; module var `st s` then module-level struct st (other layout); s = '{4'h3,4'h4}
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module m;
  st s;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
  initial begin s = '{4'h3, 4'h4}; #1 $display("A u.mod_redecl_after_pat s=%h", s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A u.mod_redecl_after_pat s=34",
    );
    // u.mod_redecl_after_kpat: PRE A u.mod_redecl_after_kpat s=03; iverilog refuses; sv2v A u.mod_redecl_after_kpat s=03; verilator refuses.
    // declaration-point binding (§4.5.592): hand-IEEE `s=34` (the unit `st`); iverilog has no
    // keyed-pattern syntax here, sv2v binds scope-wide (`03`), verilator refuses
    value(
        "u.mod_redecl_after_kpat",
        r##"// row 67 u.mod_redecl_after_kpat: unit struct st; module var `st s` then module-level struct st (other layout); s = '{a:4'h3, b:4'h4}
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
module m;
  st s;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
  initial begin s = '{a: 4'h3, b: 4'h4}; #1 $display("A u.mod_redecl_after_kpat s=%h", s); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A u.mod_redecl_after_kpat s=34",
    );
    // i.mod_after_var: PRE A i.mod_after_var a=16; iverilog A i.mod_after_var a=5; sv2v A i.mod_after_var a=5; verilator A i.mod_after_var a=16.
    // declaration-point binding (IEEE §26.3, §4.5.592): `st s` precedes `import p::*`;
    // verilator binds scope-wide
    value(
        "i.mod_after_var",
        r##"// row 67 i.mod_after_var: unit `st`; module var `st s` declared BEFORE `import p::*` (p exports st); reads s.a after the import
typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
package p;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
endpackage
module m;
  st s;
  import p::*;
  initial begin s = 8'h56; #1 $display("A i.mod_after_var a=%h", s.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A i.mod_after_var a=5",
    );
    // q4.imp_order_var: PRE A q4iov a=d; iverilog A q4iov a=16; sv2v A q4iov a=16; verilator A q4iov a=d.
    // declaration-point binding (§4.5.592); verilator binds scope-wide
    value(
        "q4.imp_order_var",
        r##"package p;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
endpackage
typedef struct packed { logic [1:0] b; logic [5:0] a; } st_t;
module m;
  st_t v;
  import p::*;
  initial begin v = 8'hD6; #1 $display("A q4iov a=%h", v.a); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A q4iov a=16",
    );
    // fp.base_mod_import_between_ctl: PRE A fp.base_mod_import_between_ctl v=12 w=12 bv=8; iverilog A fp.base_mod_import_between_ctl v=42 w=12 bv=8; sv2v A fp.base_mod_import_between_ctl v=42 w=12 bv=8; verilator A fp.base_mod_import_between_ctl v=12 w=12 bv=8.
    // declaration-point binding (§4.5.592); verilator binds scope-wide
    value(
        "fp.base_mod_import_between_ctl",
        r##"typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
package q; typedef struct packed { logic [3:0] a; logic [3:0] b; } st2; endpackage
module m;
  st2 v;
  import q::*;
  st2 w;
  initial begin v = '{2'h1, 6'h2}; w = '{4'h1, 4'h2}; #1 $display("A fp.base_mod_import_between_ctl v=%h w=%h bv=%0d", v, w, $bits(v)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A fp.base_mod_import_between_ctl v=42 w=12 bv=8",
    );
    // rv.q3c: PRE A q3c w=16; iverilog A q3c w=05; sv2v A q3c w=16; verilator refuses.
    // declaration-point binding (§4.5.592); sv2v binds scope-wide, verilator refuses
    value(
        "rv.q3c",
        r##"// review q3c: Q3 in a GENERATE block: a variable of the module type, then the block re-declares the type; v.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  logic [7:0] w;
  if (1) begin : g
    st v;
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    initial v = 8'h56;
    assign w = {2'b0, v.a};
  end
  initial begin #2 $display("A q3c w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A q3c w=05",
    );
    // x.fn_formal: PRE A x.fn_formal q=16; iverilog A x.fn_formal q=05; sv2v refuses; verilator refuses.
    // declaration-point binding (§4.5.592); verilator refuses
    value(
        "x.fn_formal",
        r##"// row 67 x.fn_formal: function formal `st x`; the body declares its own st; reads x.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  function automatic logic [7:0] q(st x);
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return {2'b0, x.a};
  endfunction
  initial begin #1 $display("A x.fn_formal q=%h", q(8'h56)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.fn_formal q=05",
    );
    // x.task_formal: PRE A x.task_formal w=16; iverilog A x.task_formal w=05; sv2v refuses; verilator refuses.
    // declaration-point binding (§4.5.592); verilator refuses
    value(
        "x.task_formal",
        r##"// row 67 x.task_formal: task formal `input st x`; the body declares its own st; reads x.a
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  logic [7:0] w;
  task automatic q(input st x, output logic [7:0] o);
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    o = {2'b0, x.a};
  endtask
  initial begin #1 q(8'h56, w); $display("A x.task_formal w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.task_formal w=05",
    );
    // n.formal: PRE A n.formal q=0; iverilog A n.formal q=2; sv2v refuses; verilator refuses.
    // declaration-point binding (§4.5.592); verilator refuses
    value(
        "n.formal",
        r##"// row 67 n.formal: an enum formal `et x` of the module enum; the function body re-declares et (other labels); returns x.next()
module m;
  typedef enum logic [1:0] {A0, A1, A2} et;
  function automatic logic [1:0] q(et x);
    typedef enum logic [1:0] {B0 = 2'd2, B1 = 2'd1, B2 = 2'd0} et;
    return x.next();
  endfunction
  initial begin #1 $display("A n.formal q=%0d", q(A1)); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A n.formal q=2",
    );
}

/// Must not move under row 67b (values as on 4858082a).
#[test]
fn cells_that_must_not_move() {
    // r2.cls_inherit: PRE A clsinh ga=16; iverilog refuses; sv2v refuses; verilator A clsinh ga=16.
    // iverilog "sorry"; sv2v has no classes
    value(
        "r2.cls_inherit",
        r##"// row 67 risk: a DERIVED class reads an inherited struct property `s.a` (base property st2 s) while the module declares st1 s
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
  st1 s;
  class B; st2 s; endclass
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display("A clsinh ga=%h", d.ga()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A clsinh ga=16",
    );
    // r2.cls_inherit_nomod: PRE A clsinh2 ga=16; iverilog refuses; sv2v refuses; verilator A clsinh2 ga=16.
    // iverilog "sorry"; sv2v has no classes
    value(
        "r2.cls_inherit_nomod",
        r##"// the same without a module variable named s (the inherited binding alone)
module m;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
  class B; st2 s; endclass
  class D extends B;
    function logic [5:0] ga(); return s.a; endfunction
  endclass
  D d;
  initial begin d = new; d.s = 8'hD6; #1 $display("A clsinh2 ga=%h", d.ga()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A clsinh2 ga=16",
    );
    // rv.c571_cls_lit: PRE A c571cl a=5 p=2; iverilog A c571cl a=5 p=2; sv2v refuses; verilator A c571cl a=5 p=2.
    value(
        "rv.c571_cls_lit",
        r##"// §4.5.571 CLASS member-read shape: a class property `st2 s` leaks past endclass onto a module `st1 s`; LITERAL member part-select (open today)
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
  class K; st2 s; endclass
  st1 s;
  initial begin s = 8'h56; #1 $display("A c571cl a=%h p=%h", s.a, s.a[2:1]); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571cl a=5 p=2",
    );
    // rv.c571_fnloc_lit: PRE A c571fl fx=1 p=0 q=00; iverilog A c571fl fx=1 p=0 q=00; sv2v refuses; verilator A c571fl fx=1 p=0 q=00.
    value(
        "rv.c571_fnloc_lit",
        r##"// §4.5.571 CLASS: a nested member key re-read under a function-local typedef of the same name; literal member part-select
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  out_t o;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
    return 8'h0;
  endfunction
  initial begin o = 16'h1205; #1 $display("A c571fl fx=%h p=%h q=%h", o.f.x, o.f.x[2:1], q()); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571fl fx=1 p=0 q=00",
    );
    // rv.c571_named: PRE A c571l kl=3; iverilog A c571l kl=3; sv2v A c571l kl=3; verilator A c571l kl=3.
    value(
        "rv.c571_named",
        r##"// row 24 loud->value check (§4.5.571 class): a function formal s of struct type st2 beside a module variable s of struct type st
// literal-bound member part-select (open today) and its named-bound twin (R05 today)
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
  localparam int H = 2;
  st s;
  function automatic logic [1:0] kl(st2 s); return s.a[2:1]; endfunction
  initial begin s = 8'h12; #1 $display("A c571l kl=%h", kl(8'hD6)); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571l kl=3",
    );
    // r2.cls_handle: PRE loud VITA-E3010; iverilog A clsh hsa=20 sa=5; sv2v refuses; verilator A clsh hsa=16 sa=5.
    // stays loud (`h.s.a` through a class handle, E3010)
    loud(
        "r2.cls_handle",
        r##"// row 67 risk: a struct property read through a class handle from the module (`h.s.a`), with a module st1 s beside the class st2 s
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st1;
  typedef struct packed { logic [1:0] b; logic [5:0] a; } st2;
  st1 s;
  class K; st2 s; endclass
  K h;
  initial begin h = new; h.s = 8'hD6; s = 8'h56; #1 $display("A clsh hsa=%h sa=%h", h.s.a, s.a); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "error[VITA-E3010]",
    );
    // x.cat.c571alias: PRE A c571alias w=42; iverilog A c571alias w=12; sv2v A c571alias w=12; verilator A c571alias w=12.
    // KNOWN-WRONG, held (ROADMAP §2 struct-binding-by-name: the vector alias `st` binds the
    // imported struct's layout by NAME where the variable is declared; 67b keeps that binding)
    value(
        "x.cat.c571alias",
        r##"// catalog §4.5.571 name-keyed struct binding: a vector alias st over an imported struct st
package p; typedef struct packed {logic [1:0] a; logic [5:0] b;} st; endpackage
module m;
  import p::*;
  typedef logic [1:0][3:0] st;
  st w;
  initial begin w = '{4'h1, 4'h2}; #1 $display("A c571alias w=%h", w); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571alias w=42",
    );
    // x.cat.c571formal: PRE A c571formal g=42; iverilog A c571formal g=12; sv2v A c571formal g=12; verilator A c571formal g=12.
    // KNOWN-WRONG, held (ROADMAP §2 struct-binding-by-name: a plain formal `s` does not unbind
    // the module struct variable `s`)
    value(
        "x.cat.c571formal",
        r##"// catalog §4.5.571 name-keyed struct binding: a formal s of vector type beside a module st s
module m;
  typedef struct packed {logic [1:0] a; logic [5:0] b;} st;
  st s;
  function automatic logic [7:0] g(input logic [1:0][3:0] s); s = '{4'h1, 4'h2}; return s; endfunction
  initial #1 $display("A c571formal g=%h", g(8'h0));
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c571formal g=42",
    );
    // x.cat.c573: PRE A c573 b=10 v=3ff; iverilog refuses; sv2v A c573 b=8 v=ff; verilator A c573 b=8 v=ff.
    // KNOWN-WRONG, held (ROADMAP §2 struct-binding-by-name: the unit record `st` is bound where
    // the module's vector typedef `st` is newer)
    value(
        "x.cat.c573",
        r##"// catalog §4.5.573 a module typedef st under a unit UNPACKED struct st
typedef struct {logic [4:0] a; logic [4:0] b;} st;
module m;
  typedef logic [7:0] st;
  st v;
  initial begin v = '1; #1 $display("A c573 b=%0d v=%h", $bits(v), v); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"##,
        "A c573 b=10 v=3ff",
    );
    // t3.ctl: PRE A t3.ctl w=3405; iverilog refuses; sv2v A t3.ctl w=3405; verilator A t3.ctl w=3405.
    value(
        "t3.ctl",
        r##"// row 67 t3.ctl: typed control, no re-declaration
module m;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
  logic [15:0] w;
  function automatic logic [15:0] q(); return out_t'{'{4'h3, 4'h4}, 8'h05}; endfunction
  initial begin #1 w = q(); $display("A t3.ctl w=%h", w); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A t3.ctl w=3405",
    );
    // rv.xp1: PRE A xp1 w=1 o2=3405; iverilog A xp1 w=1 o2=3405; sv2v A xp1 w=1 o2=3405; verilator A xp1 w=1 o2=3405.
    value(
        "rv.xp1",
        r##"// review xp1: package q imports p's in_t and declares out_t {in_t f}; the module declares its own in_t; o.f.x and a nested pattern
package p; typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t; endpackage
package q; import p::*; typedef struct packed { in_t f; logic [7:0] g; } out_t; endpackage
module m;
  import q::*;
  typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
  out_t o, o2; logic [3:0] w;
  initial begin o = 16'h1205; w = o.f.x; o2 = '{'{4'h3, 4'h4}, 8'h05}; #1 $display("A xp1 w=%h o2=%h", w, o2); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A xp1 w=1 o2=3405",
    );
    // p.nested_scoped: PRE A p.nested_scoped x=1 o2=3405; iverilog A p.nested_scoped x=1 o2=3405; sv2v A p.nested_scoped x=1 o2=3405; verilator A p.nested_scoped x=1 o2=3405.
    value(
        "p.nested_scoped",
        r##"// row 67 p.nested_scoped: package out_t{in_t f}; module `p::out_t o` with its own `in_t`; o.f.x and o = '{'{..},..}
package p;
  typedef struct packed { logic [3:0] x; logic [3:0] y; } in_t;
  typedef struct packed { in_t f; logic [7:0] g; } out_t;
endpackage
module m;
  typedef struct packed { logic [1:0] x; logic [5:0] y; } in_t;
  p::out_t o, o2;
  initial begin o = 16'h1205; o2 = '{'{4'h3, 4'h4}, 8'h05}; #1 $display("A p.nested_scoped x=%h o2=%h", o.f.x, o2); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A p.nested_scoped x=1 o2=3405",
    );
    // f2.fn.S.decl_after: PRE A f2.fn.S.decl_after a=5 s3=34 q=00; iverilog A f2.fn.S.decl_after a=5 s3=34 q=00; sv2v refuses; verilator A f2.fn.S.decl_after a=5 s3=34 q=00.
    value(
        "f2.fn.S.decl_after",
        r##"// row 67 f2.fn.S.decl_after: a function re-declares st; a module variable `st s2` declared AFTER the function binds the module st; s2.a and s2 = '{..}
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return 8'h0;
  endfunction
  st s2, s3;
  initial begin s2 = 8'h56; s3 = '{4'h3, 4'h4}; #1 $display("A f2.fn.S.decl_after a=%h s3=%h q=%h", s2.a, s3, q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A f2.fn.S.decl_after a=5 s3=34 q=00",
    );
    // x.decl_init_fn: PRE A x.decl_init_fn s=34 q=00; iverilog A x.decl_init_fn s=34 q=00; sv2v refuses; verilator A x.decl_init_fn s=34 q=00.
    value(
        "x.decl_init_fn",
        r##"// row 67 x.decl_init_fn: function-local inner st; a function-local var of the OUTER... n/a: module `st s = '{..}` decl-init after a function with inner st
module m;
  typedef struct packed { logic [3:0] a; logic [3:0] b; } st;
  function automatic logic [7:0] q();
    typedef struct packed { logic [1:0] b; logic [5:0] a; } st;
    return 8'h0;
  endfunction
  st s = '{4'h3, 4'h4};
  initial begin #1 $display("A x.decl_init_fn s=%h q=%h", s, q()); end
endmodule
module top; m u(); initial #20 $finish; endmodule
"##,
        "A x.decl_init_fn s=34 q=00",
    );
}
