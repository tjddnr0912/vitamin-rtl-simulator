//! N3.4 follow-on: extend the multi-dim-packed part-select fix to the three
//! sibling cases the original N3.4 slice left as documented silent-wrong / lenient:
//!
//!   1. an ARRAY-OF-PACKED element part-select `qm[i][msb:lsb]` (read + write) —
//!      `qm[0][3:2]` selects whole outer sub-elements of array element 0, not flat
//!      bits (used to read `3` / no-op a write);
//!   2. a CONSTANT indexed part-select `x[c+:w]` / `x[c-:w]` on a multi-dim packed
//!      net (bare or an array element) — `x[2+:2]` ≡ `x[3:2]` (iverilog folds it to
//!      a range); a VARIABLE offset `x[i+:2]` is loud (iverilog 13.0 ABORTS on it —
//!      no oracle for the bit-vs-element unit);
//!   3. a NESTED range-then-index `x[3:2][0]` / `v[3:0][0]` — iverilog rejects this
//!      universally ("All but the final index ... must be a single value, not a
//!      range"); vita used to silently bit-select the narrowed result.
//!
//! Pure IR-0 (elaborate lowering only). Every value is pinned to LIVE iverilog 13.0,
//! EXCEPT the variable indexed part-select, where iverilog aborts and vita is loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_n34f_{}_{n}", std::process::id()));
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

#[test]
fn array_of_packed_part_select_read() {
    // reg [3:0][7:0] qm [0:1]: qm[0]={aa,bb,cc,dd}, qm[1]={11,22,33,44}.
    // qm[0][3:2] selects outer elements 3,2 of element 0 = aabb (16 bits), NOT
    // flat bits `3`. A VARIABLE array index still works (qm[k][3:2]).
    let (out, _c) = run("module t;\n\
         reg [3:0][7:0] qm [0:1];\n\
         integer k;\n\
         initial begin\n\
           qm[0][3]=8'haa; qm[0][2]=8'hbb; qm[0][1]=8'hcc; qm[0][0]=8'hdd;\n\
           qm[1][3]=8'h11; qm[1][2]=8'h22; qm[1][1]=8'h33; qm[1][0]=8'h44;\n\
           $display(\"R0 %h\", qm[0][3:2]);\n\
           $display(\"R1 %h\", qm[1][1:0]);\n\
           k=0;\n\
           $display(\"RK %h\", qm[k][3:2]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("R0 aabb"), "qm[0][3:2] = elements 3,2:\n{out}");
    assert!(out.contains("R1 3344"), "qm[1][1:0] = elements 1,0:\n{out}");
    assert!(
        out.contains("RK aabb"),
        "variable array index, const part-select:\n{out}"
    );
}

#[test]
fn array_of_packed_part_select_write() {
    // qm[0][3:2] = 16'h1234 writes whole outer elements 3,2 of element 0
    // (=> qm[0] = 12340000), not 2 flat bits (which was a silent no-op).
    let (out, _c) = run("module t;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           qm[0]=32'h0; qm[1]=32'h0;\n\
           qm[0][3:2]=16'h1234;\n\
           qm[1][1:0]=16'h5678;\n\
           $display(\"W0 %h\", qm[0]);\n\
           $display(\"W1 %h\", qm[1]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("W0 12340000"), "{out}");
    assert!(out.contains("W1 00005678"), "{out}");
}

#[test]
fn bare_const_indexed_part_selects_outer_elements() {
    // x[2+:2] starts at element 2 and spans 2 elements UP => {x[3],x[2]} = aabb,
    // identical to the range form x[3:2]. x[3-:2] spans 2 DOWN from element 3 =>
    // the same {x[3],x[2]} = aabb. iverilog folds both const forms to the range.
    let (out, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin\n\
           x[3]=8'haa; x[2]=8'hbb; x[1]=8'hcc; x[0]=8'hdd;\n\
           $display(\"UP %h\", x[2+:2]);\n\
           $display(\"DN %h\", x[3-:2]);\n\
           $display(\"LO %h\", x[0+:2]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("UP aabb"), "x[2+:2]:\n{out}");
    assert!(out.contains("DN aabb"), "x[3-:2]:\n{out}");
    assert!(out.contains("LO ccdd"), "x[0+:2] = elements 1,0:\n{out}");
}

#[test]
fn array_of_packed_const_indexed_part() {
    // qm[0][2+:2] on the array element = aabb (read); writing it = 12340000.
    let (rd, _c) = run("module t;\n\
         reg [3:0][7:0] qm [0:1];\n\
         integer k;\n\
         initial begin\n\
           qm[0][3]=8'haa; qm[0][2]=8'hbb; qm[0][1]=8'hcc; qm[0][0]=8'hdd;\n\
           k=0;\n\
           $display(\"AR %h\", qm[k][2+:2]);\n\
         end\n\
         endmodule\n");
    assert!(rd.contains("AR aabb"), "qm[k][2+:2] read:\n{rd}");
    let (wr, _c) = run("module t;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin qm[0]=0; qm[0][2+:2]=16'h1234; $display(\"AW %h\", qm[0]); end\n\
         endmodule\n");
    assert!(wr.contains("AW 12340000"), "qm[0][2+:2] write:\n{wr}");
}

#[test]
fn const_indexed_part_write_outer_elements() {
    // x[2+:2] = 16'h1234 => 12340000; x[3-:2] = 16'h1234 => 12340000 (same range).
    let (a, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; x[2+:2]=16'h1234; $display(\"A %h\", x); end\n\
         endmodule\n");
    assert!(a.contains("A 12340000"), "x[2+:2]=…:\n{a}");
    let (b, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; x[3-:2]=16'h1234; $display(\"B %h\", x); end\n\
         endmodule\n");
    assert!(b.contains("B 12340000"), "x[3-:2]=…:\n{b}");
}

#[test]
fn minus_colon_low_boundary_does_not_underflow() {
    // Review M1 (HIGH): a `-:` select whose LOWEST element index is 0 used to
    // underflow u32 (`c - w + 1` evaluated `c - w` first) → debug panic / release
    // wrap. iverilog accepts and returns the whole-element slice. Read, array
    // element, AND write. reg [3:0][7:0] x = aabbccdd ⇒ x[3]=aa,x[2]=bb,x[1]=cc,x[0]=dd.
    let (rd, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           x = 32'haabbccdd; qm[0] = 32'haabbccdd;\n\
           $display(\"L1 %h\", x[0-:1]);\n\
           $display(\"L2 %h\", x[1-:2]);\n\
           $display(\"L3 %h\", x[2-:3]);\n\
           $display(\"LA %h\", qm[0][1-:2]);\n\
         end\n\
         endmodule\n");
    assert!(rd.contains("L1 dd"), "x[0-:1] = element 0:\n{rd}");
    assert!(rd.contains("L2 ccdd"), "x[1-:2] = elements 1,0:\n{rd}");
    assert!(rd.contains("L3 bbccdd"), "x[2-:3] = elements 2,1,0:\n{rd}");
    assert!(rd.contains("LA ccdd"), "qm[0][1-:2] = elements 1,0:\n{rd}");
    let (wr, _c) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; x[1-:2]=16'hface; $display(\"LW %h\", x); end\n\
         endmodule\n");
    assert!(
        wr.contains("LW 0000face"),
        "x[1-:2]=… writes elements 1,0:\n{wr}"
    );
}

#[test]
fn variable_indexed_part_is_loud() {
    // x[i+:2] with a RUNTIME offset on a multi-dim packed array: iverilog 13.0
    // aborts (internal assertion) — no defined bit-vs-element semantics — so vita
    // is loud (E3009), NOT a silent flat-bit read. Both read and write.
    let (rd, rc) = run("module t;\n\
         reg [3:0][7:0] x; integer i;\n\
         initial begin x[3]=8'haa; i=2; $display(\"%h\", x[i+:2]); end\n\
         endmodule\n");
    assert!(
        rd.contains("VITA-E3009") || rc == Some(1),
        "variable indexed part-select read must be loud: {rd} code={rc:?}"
    );
    let (_w, wc) = run("module t;\n\
         reg [3:0][7:0] x; integer i;\n\
         initial begin x=0; i=2; x[i+:2]=16'h1234; end\n\
         endmodule\n");
    assert_eq!(
        wc,
        Some(1),
        "variable indexed part-select write must be loud"
    );
    // ... but on a PLAIN vector a variable indexed part-select is legal flat-bits.
    let (pv, _c) = run("module t;\n\
         reg [7:0] v; integer i;\n\
         initial begin v=8'h5a; i=2; $display(\"PV %h\", v[i+:4]); end\n\
         endmodule\n");
    assert!(
        pv.contains("PV 6"),
        "plain vector v[i+:4] stays flat-bits:\n{pv}"
    );
}

#[test]
fn nested_range_then_index_is_loud() {
    // A range select cannot be further indexed. iverilog rejects this universally
    // (on multi-dim packed AND plain vectors); vita used to silently bit-select the
    // narrowed result (`x[3:2][0]` => 0, `v[3:0][0]` => 0).
    for (label, src) in [
        (
            "x[3:2][0]",
            "reg [3:0][7:0] x; initial begin x[3]=8'haa; $display(\"%h\", x[3:2][0]); end",
        ),
        (
            "x[2+:2][0]",
            "reg [3:0][7:0] x; initial begin x[3]=8'haa; $display(\"%h\", x[2+:2][0]); end",
        ),
        (
            "v[3:0][0]",
            "reg [7:0] v; initial begin v=8'h5a; $display(\"%h\", v[3:0][0]); end",
        ),
    ] {
        let (out, code) = run(&format!("module t;\n  {src}\nendmodule\n"));
        assert!(
            out.contains("VITA-E3009") || code == Some(1),
            "range-then-index {label} must be loud: {out} code={code:?}"
        );
    }
}

#[test]
fn const_indexed_part_out_of_range_is_loud() {
    // x[3+:2] reaches elements 3,4 but the outer dim is [3:0] — element 4 is out of
    // range. iverilog rejects ("Part-select ... exceeds the declared bounds"); vita
    // is loud (E3009), NOT a silent read past the net. Likewise x[0-:2] underflows.
    let (a, ac) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; $display(\"%h\", x[3+:2]); end\n\
         endmodule\n");
    assert!(
        a.contains("VITA-E3009") || ac == Some(1),
        "over-range x[3+:2] must be loud: {a} code={ac:?}"
    );
    let (_b, bc) = run("module t;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=0; x[0-:2]=16'h1234; end\n\
         endmodule\n");
    assert_eq!(bc, Some(1), "underflow x[0-:2] must be loud");
}
