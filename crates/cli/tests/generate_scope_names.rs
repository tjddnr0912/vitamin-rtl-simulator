//! The name of a generate scope as `%m`, a hierarchical reference and the
//! diagnostic context see it — ROADMAP §2 🆕 N residue (§4.5.432).
//!
//! Two defects, both 2-oracle (iverilog 13.0 · verilator 5.050):
//!
//! * a `generate if/else` whose branches carry DIFFERENT labels named the taken
//!   ELSE scope with the IF label (`if (0) begin : n … end else begin : y … end` →
//!   `%m` = `top.n`, `n.v` resolved, `y.v` did not; both oracles `top.y` / `y.v`).
//!   The parser hoisted the then block's label into the `If` node's one slot and
//!   dropped the else's; it now keeps the else block as a `GenItem::Block`.
//! * an UNNAMED generate block contributed no scope segment at all. IEEE 1800
//!   §27.6 names it `genblk<N>`, N the number of the generate construct in its
//!   scope counting every construct, named or not; nested scopes restart at 1;
//!   an `else if` chain is one construct. `%m` printed `top`, an instance inside
//!   was `top.u` (both oracles `top.genblk1.u`), `genblk1.v` from the module was
//!   E3010 (both oracles read it), and a net declared inside was visible at module
//!   scope (both oracles refuse). An unnamed loop was `genblk[i]`.
//!
//! Every line below is the oracles' output, copied (22-cell census, 17 fixed).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gsn_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn top(body: &str) -> String {
    format!("`timescale 1ns/1ns\nmodule top;\n{body}\n  initial #5 $finish;\nendmodule\n")
}

fn d_lines(src: &str) -> Vec<String> {
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "exit\n{out}");
    out.lines()
        .filter(|l| l.starts_with("D="))
        .map(str::to_string)
        .collect()
}

fn prints(src: &str, want: &[&str]) {
    assert_eq!(d_lines(src), want);
}

fn m(tag: &str) -> String {
    format!("initial $display(\"D={tag} %m\");")
}

#[test]
fn the_taken_else_branch_is_named_by_its_own_label() {
    // n01 · PRE `top.n`
    prints(
        &top(&format!(
            "  generate if (0) begin : n {} end else begin : y {} end endgenerate",
            m("n"),
            m("y")
        )),
        &["D=y top.y"],
    );
    // n02 · the common same-label idiom, unchanged
    prints(
        &top(&format!(
            "  generate if (0) begin : g {} end else begin : g {} end endgenerate",
            m("n"),
            m("y")
        )),
        &["D=y top.g"],
    );
    // n11 · a hierarchical read INTO the else scope resolves by the else label
    // (PRE E3010) and no longer by the if label (PRE read it; both oracles refuse)
    let src = |lbl: &str| {
        top(&format!(
            "  generate if (0) begin : n logic [3:0] v = 4'h1; end else begin : y logic [3:0] v \
             = 4'h2; end endgenerate\n  initial #1 $display(\"D=%h\", {lbl}.v);"
        ))
    };
    prints(&src("y"), &["D=2"]);
    let (out, code) = run(&src("n"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("E-ELAB-UNRESOLVED-NAME"), "{out}");
    // n16 · a named THEN with an unnamed ELSE: the else is `genblk1`, not `n`
    prints(
        &top(
            "  generate if (0) begin : n logic [3:0] v = 4'h1; end else begin logic [3:0] v = \
             4'h2; initial $display(\"D=e %h %m\", v); end endgenerate",
        ),
        &["D=e 2 top.genblk1"],
    );
}

#[test]
fn an_unnamed_block_is_genblk_n_counting_every_construct_in_its_scope() {
    // n04 · a named construct takes a number too; a construct outside a
    // `generate` region counts in the same sequence
    prints(
        &top(&format!(
            "  generate if (1) begin {} end endgenerate\n  generate if (1) begin : nm {} end \
             endgenerate\n  generate if (1) begin {} end endgenerate\n  if (1) begin {} end",
            m("a"),
            m("nm"),
            m("c"),
            m("bare")
        )),
        &[
            "D=a top.genblk1",
            "D=nm top.nm",
            "D=c top.genblk3",
            "D=bare top.genblk4",
        ],
    );
    // n03 · an unnamed else after a named then
    prints(
        &top(&format!(
            "  generate if (0) begin : c {} end else begin {} end endgenerate",
            m("c"),
            m("e")
        )),
        &["D=e top.genblk1"],
    );
    // n12 · an un-blocked branch is an implicit block
    prints(
        &top(&format!(
            "  generate if (1) {} endgenerate\n  generate if (0) {} else {} endgenerate",
            m("bare"),
            m("x"),
            m("bareelse")
        )),
        &["D=bare top.genblk1", "D=bareelse top.genblk2"],
    );
    // n05 / n18 · an unnamed loop is `genblk<N>[i]` (PRE `genblk[i]`)
    prints(
        &top(&format!(
            "  genvar i; generate if (1) begin : q end for (i=0;i<2;i++) begin {} end endgenerate",
            m("f")
        )),
        &["D=f top.genblk2[0]", "D=f top.genblk2[1]"],
    );
    prints(
        &top(
            "  genvar i; generate for (i=0;i<2;i++) begin logic [3:0] v; assign v = i + 1; \
             initial #1 $display(\"D=v %h %m\", v); end endgenerate",
        ),
        &["D=v 1 top.genblk1[0]", "D=v 2 top.genblk1[1]"],
    );
    // n06 · case arms: a labelled arm, an unnamed block arm, an un-blocked arm
    prints(
        &top(&format!(
            "  generate case (1) 0: begin : z {} end 1: begin : o {} end endcase endgenerate\n  \
             generate case (1) 1: begin {} end endcase endgenerate\n  generate case (1) 1: {} \
             endcase endgenerate",
            m("z"),
            m("o"),
            m("u"),
            m("bare")
        )),
        &["D=o top.o", "D=u top.genblk2", "D=bare top.genblk3"],
    );
    // n20 · a bare `begin … end` in a region is transparent (anachronistic surround)
    prints(
        &top(&format!(
            "  generate begin if (1) begin {} end end endgenerate",
            m("bb")
        )),
        &["D=bb top.genblk1"],
    );
}

#[test]
fn an_else_if_chain_is_one_construct() {
    // n07 · PRE `top.a.b` / `top`
    prints(
        &top(&format!(
            "  generate if (0) begin : a {} end else if (1) begin : b {} end else begin : c {} end \
             endgenerate\n  generate if (0) begin {} end else if (1) begin {} end endgenerate",
            m("a"),
            m("b"),
            m("c"),
            m("a2"),
            m("b2")
        )),
        &["D=b top.b", "D=b2 top.genblk2"],
    );
}

#[test]
fn numbering_restarts_in_every_nested_scope() {
    // n08
    prints(
        &top(&format!(
            "  generate if (1) begin : g if (1) begin {} end if (1) begin : h if (1) begin {} \
             end end end endgenerate\n  generate if (1) begin if (1) begin {} end end endgenerate",
            m("gu"),
            m("ghu"),
            m("uu")
        )),
        &[
            "D=gu top.g.genblk1",
            "D=ghu top.g.h.genblk1",
            "D=uu top.genblk2.genblk1",
        ],
    );
    // n13 / n19 · a labelled loop inside an unnamed block, and an unnamed `if`
    // inside each iteration of a labelled loop (numbering restarts per iteration)
    prints(
        &top(&format!(
            "  genvar i; generate if (1) begin for (i=0;i<2;i++) begin : L {} end end endgenerate",
            m("L")
        )),
        &["D=L top.genblk1.L[0]", "D=L top.genblk1.L[1]"],
    );
    prints(
        &top(&format!(
            "  genvar i; generate for (i=0;i<2;i++) begin : L if (i==1) begin {} end end \
             endgenerate",
            m("in")
        )),
        &["D=in top.L[1].genblk1"],
    );
    // n15 · each module numbers its own
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule ch; generate if (1) begin {} end if (1) begin {} end \
         endgenerate endmodule\nmodule top;\n  ch u();\n  generate if (1) begin {} end \
         endgenerate\n  initial #5 $finish;\nendmodule\n",
        m("ch1"),
        m("ch2"),
        m("top")
    ));
    assert_eq!(code, Some(0), "{out}");
    let mut got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
    got.sort();
    assert_eq!(
        got,
        [
            "D=ch1 top.u.genblk1",
            "D=ch2 top.u.genblk2",
            "D=top top.genblk1"
        ]
    );
}

#[test]
fn a_concurrent_assertion_inside_a_generate_block_sees_the_blocks_nets() {
    // review B (regression): a pending `assert property` was materialised at module
    // scope, so an unnamed block's own net was `top.v` (E3010) once the block had a
    // scope of its own — the old flattening had made it work by accident; a NAMED
    // block was loud on PRE too. Verilator's lines (iverilog: `sorry` on the shape).
    let (out, code) = run(&top(
        "  logic clk = 0;\n  always #1 clk = ~clk;\n  generate\n    if (1) begin\n      logic [3:0] v; \
         initial v = 4'h3;\n      ap: assert property (@(posedge clk) v == 4'h3) else \
         $display(\"FAILCONC %m\");\n    end\n    if (1) begin : nb\n      logic [3:0] w; initial w = \
         4'h9;\n      always @(posedge clk) $display(\"D=tick %m w=%h\", w);\n    end\n  endgenerate",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(!out.contains("FAILCONC"), "{out}");
    assert!(out.lines().any(|l| l == "D=tick top.nb w=9"), "{out}");
    // a cover property inside an unnamed block counts the block's own net
    let (out, code) = run(&top(
        "  logic clk = 0; always #1 clk = ~clk;\n  generate if (1) begin\n    logic [3:0] v = 4'h3;\n    \
         cp: cover property (@(posedge clk) v == 4'h3);\n  end endgenerate",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("Cover property hits: 2"), "{out}");
}

#[test]
fn an_unnamed_block_is_a_scope_for_its_nets_and_instances() {
    // n09 · `genblk1.v` from the module reads the block's net (PRE E3010)
    prints(
        &top(
            "  generate if (1) begin logic [3:0] v = 4'hA; initial $display(\"D=in %h\", v); end \
             endgenerate\n  initial #1 $display(\"D=hier %h\", genblk1.v);",
        ),
        &["D=in a", "D=hier a"],
    );
    // n09b · the bare name is NOT visible at module scope (both oracles refuse;
    // PRE read it — the flatten leniency §2 🆕 L ⓜ recorded)
    let (out, code) = run(&top(
        "  generate if (1) begin logic [3:0] v = 4'hA; end endgenerate\n  initial #1 \
         $display(\"D=leak %h\", v);",
    ));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("E-ELAB-UNRESOLVED-NAME"), "{out}");
    // n10 · an instance inside is `top.genblk1.u` (PRE `top.u`)
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule ch; {} endmodule\nmodule top;\n  generate if (1) begin ch \
         u(); end endgenerate\n  initial #5 $finish;\nendmodule\n",
        m("ch")
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "D=ch top.genblk1.u"), "{out}");
    // n17 · an assign inside an unnamed block reading a module net, unchanged
    prints(
        &top(
            "  logic [3:0] a = 4'h3, b;\n  generate if (1) begin assign b = a + 1; end \
             endgenerate\n  initial #1 $display(\"D=b %h\", b);",
        ),
        &["D=b 4"],
    );
}
