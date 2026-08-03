//! External report round-28 (2026-08-03) — IEEE 1364-2005 §3.5 implicit net declaration.
//!
//! The reporter analysed 97 `E3010` + 3 `E3009` sites in a commercial ASIC tree and
//! classified each by "is this legal standard Verilog". Seven unique roots; six were
//! vita gaps and only one was a real RTL defect. The headline is §3.5:
//!
//! > An undeclared identifier in the TERMINAL LIST of a gate/module instance, or on the
//! > LHS of a CONTINUOUS ASSIGNMENT, is implicitly declared as a scalar net of the
//! > current `` `default_nettype `` — which defaults to `wire`.
//!
//! vita rejected both positions, i.e. it behaved as `` `default_nettype none `` always,
//! and doc-15 recorded that as deliberate policy: "오타가 조용히 wire가 되는 사고
//! 클래스가 원천 차단되는 보수적 선택". Conservative, non-conforming, and — this is what
//! changed the verdict — UNFIXABLE from the user side: two of the seven sites are inside
//! foundry-supplied cell libraries and IP models, so there is no edit the user is allowed
//! to make. The safety the refusal bought is bought instead by `W2003`, which doc-15 had
//! already reserved for exactly this and which `-Werror=W-PARSE-IMPLICIT-NET` promotes
//! back to a hard error.
//!
//! The BOUNDARY is the whole game, and it is iverilog-pinned rather than inferred: an
//! ordinary rhs, a procedural lvalue, and anything under `` `default_nettype none `` are
//! errors there too. Every case below was measured against iverilog 13.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r28_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.v"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.v")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

/// 8-A — a gate-primitive terminal, the foundry standard-cell idiom. 24 instances of
/// `HVTNOR2BD2` in the reporter's design, and the cell is a foundry deliverable.
/// iverilog: `y=0`.
#[test]
fn a_gate_terminal_declares_an_implicit_net() {
    let (o, ok) = run("module g1 (input AN, B, output Y);\n\
           not (Ax, AN);\n\
           nor (Y, Ax, B);\n\
         endmodule\n\
         module t; reg an = 1'b0, b = 1'b0; wire y; g1 u(.AN(an), .B(b), .Y(y));\n\
           initial begin #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert!(o.contains("y=0"), "iverilog gives y=0:\n{o}");
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
    assert!(
        o.contains("VITA-W2003"),
        "the implicit net must still be announced:\n{o}"
    );
}

/// 8-B / 8-C — a continuous-assignment LHS. iverilog: `y=1`.
#[test]
fn a_continuous_assign_lhs_declares_an_implicit_net() {
    let (o, ok) = run("module g2 (input a, output y);\n\
           assign mid = ~a;\n\
           assign y = mid;\n\
         endmodule\n\
         module t; reg a = 1'b0; wire y; g2 u(.a(a), .y(y));\n\
           initial begin #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert!(o.contains("y=1"), "iverilog gives y=1:\n{o}");
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
}

/// 8-E — a module-instance terminal, and the ORDER trap that came with it.
///
/// vita lowers continuous assigns BEFORE instances, so declaring the implicit net at the
/// terminal-list use site made `sub u(.o(IMPL)); assign o = IMPL;` fail at the READ while
/// succeeding at the terminal — one design, two verdicts, decided by phase order. §3.5 is
/// a declaration, so it runs as its own pass before anything is lowered. This test has
/// the read BEFORE the instantiation would be reached.
#[test]
fn an_instance_terminal_declares_it_early_enough_to_read() {
    let (o, ok) = run("module dn; wire o; sub u0(.o(IMPL)); assign o = IMPL;\n\
           initial begin #1 $display(\"o=%b\", o); $finish; end\n\
         endmodule\n\
         module sub(output o); assign o = 1'b1; endmodule\n");
    assert!(o.contains("o=1"), "iverilog gives o=1:\n{o}");
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
}

/// The boundary, in the three directions §3.5 does NOT cover. All three are errors in
/// iverilog too — this is conformance, not leniency, and each of these staying loud is
/// what keeps a typo from silently becoming a wire.
#[test]
fn the_positions_outside_section_3_5_stay_loud() {
    let cases: [(&str, &str); 3] = [
        (
            "an ordinary rhs",
            "module t; wire y; assign y = UNDECL;\n\
               initial begin #1 $display(\"y=%b\", y); $finish; end endmodule\n",
        ),
        (
            "a procedural lvalue",
            "module t; wire y; initial begin PROCV = 1'b1; end assign y = 1'b0;\n\
               initial begin #1 $display(\"y=%b\", y); $finish; end endmodule\n",
        ),
        (
            "under `default_nettype none",
            "`default_nettype none\n\
             module t; wire y; assign MID = 1'b1; assign y = MID;\n\
               initial begin #1 $display(\"y=%b\", y); $finish; end endmodule\n",
        ),
    ];
    for (what, src) in cases {
        let (o, ok) = run(src);
        assert_ne!(ok, Some(0), "{what} must stay an error:\n{o}");
        assert!(o.contains("VITA-E3010"), "{what} — expected E3010:\n{o}");
    }
}

/// `` `default_nettype `` is a real directive now, and it is STICKY in file order: the
/// module after `none` is strict, the module after a later `wire` is not. Both halves in
/// one file, so a policy that ignored the directive fails on one of them whichever way
/// it guessed.
#[test]
fn default_nettype_is_sticky_in_file_order() {
    let (o, ok) = run("`default_nettype none\n\
         module strict; wire y; assign y = 1'b0;\n\
           initial begin #1 $display(\"S=%b\", y); end\n\
         endmodule\n\
         `default_nettype wire\n\
         module loose; wire y; assign MID = 1'b1; assign y = MID;\n\
           initial begin #1 $display(\"L=%b\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "the `wire` module must elaborate:\n{o}");
    assert!(o.contains("L=1"), "implicit net in the `wire` module:\n{o}");
    assert!(o.contains("VITA-W2003"), "and it is announced:\n{o}");
}

/// ★ 8-D — the RTL defect the reporter found, made LOUD.
///
/// A §3.5 net is SCALAR, so a wider driver keeps bit 0 and discards the rest. Every
/// simulator does this silently because the code is legal; the reporter's site had a
/// 12-bit value assigned to an implicit 1-bit net because the port declaration sat behind
/// an inactive `ifdef` and the `assign` did not. vita matches the VALUE (differential
/// wins) and states the loss with both widths.
#[test]
fn a_wider_driver_on_an_implicit_net_says_how_many_bits_are_lost() {
    let (o, ok) = run("module t; wire [11:0] src = 12'hABC;\n\
           assign IMPL = src;\n\
           initial begin #1 $display(\"IMPL=%b\", IMPL); $finish; end\n\
         endmodule\n");
    assert!(
        o.contains("IMPL=0"),
        "iverilog gives IMPL=0 (bit 0 of 0xABC):\n{o}"
    );
    assert_eq!(ok, Some(0), "legal code, so not an error:\n{o}");
    assert!(
        o.contains("12 bits") && o.contains("top 11 are discarded"),
        "the truncation must be stated with the widths:\n{o}"
    );
}

/// 9-A — the diagnostic that sent the reporter looking for an assignment that does not
/// exist. A hierarchical name in an EVENT CONTROL was reported through `resolve_net`'s
/// lvalue text ("a hierarchical name in this lvalue context … a whole-net hierarchical
/// write `tb.dut.x = …` is supported"), and the design contains no hierarchical write.
#[test]
fn a_hierarchical_event_control_is_not_reported_as_an_lvalue() {
    let (o, ok) = run("module sub; reg [3:0] x; initial x = 4'h5; endmodule\n\
         module top; sub u(); reg [3:0] y;\n\
           always @(top.u.x) y = 4'h1;\n\
           initial begin #1 $display(\"y=%h\", y); $finish; end\n\
         endmodule\n");
    assert_ne!(ok, Some(0), "still unsupported, so still loud:\n{o}");
    assert!(
        o.contains("EVENT CONTROL"),
        "the message must name the context it is actually in:\n{o}"
    );
    assert!(
        !o.contains("lvalue"),
        "and must not send the reader looking for an assignment:\n{o}"
    );
    // The claim the message makes about what DOES work has to be true.
    let (o2, ok2) = run("module sub; reg [3:0] x; initial x = 4'h5; endmodule\n\
         module top; sub u(); reg [3:0] y;\n\
           always @(*) y = top.u.x;\n\
           initial begin #1 $display(\"y=%h\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(ok2, Some(0), "the offered workaround must work:\n{o2}");
    assert!(o2.contains("y=5"), "and give the right value:\n{o2}");
}

/// The appendix — `specparam` as a module-level constant. A vendor model that keeps its
/// timing constants there and references them from ordinary delay expressions is a common
/// pattern; the reporter had to rewrite the KEYWORD to get a foundry EFUSE model through.
#[test]
fn specparam_is_accepted_as_a_constant() {
    let (o, ok) = run("module sp1;\n\
           specparam tP = 4;\n\
           initial begin #(tP*0.5) $display(\"t=%0t\", $time); $finish; end\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "specparam must parse:\n{o}");
    assert!(o.contains("t=2"), "and fold like a localparam:\n{o}");
}

/// The boundary the existing `dotname_missing_signal_is_loud` pin caught in this slice.
///
/// The parser desugars the `.name` shorthand to `.name(name)`, so a naive "every named
/// port actual is a §3.5 terminal" rule made `sub u(.a, .b);` with no `a` declared
/// silently create one — turning a loud bind error into a working-but-wrong design.
/// IEEE 1800 §23.3.2.2 says the shorthand connects to an object "declared in the
/// instantiating module", and iverilog draws exactly that line: `.a(a)` with no `a` is
/// accepted (§3.5 applies), `.a` with no `a` is `Unable to bind wire/reg/memory 'a'`.
#[test]
fn the_dot_name_shorthand_is_not_a_section_3_5_position() {
    let sub = "module sub(input a, output b); assign b = ~a; endmodule\n";
    // Explicit `.a(a)` — §3.5 applies, iverilog accepts.
    let (o1, ok1) = run(&format!(
        "{sub}module top; logic b; sub u(.a(a), .b(b));\n\
           initial begin #1 $display(\"E=%b\", b); $finish; end endmodule\n"
    ));
    assert_eq!(ok1, Some(0), "`.a(a)` is a §3.5 terminal:\n{o1}");
    assert!(o1.contains("VITA-W2003"), "and is announced:\n{o1}");
    // Shorthand `.a` — not a §3.5 position, must stay loud.
    let (o2, ok2) = run(&format!(
        "{sub}module top; logic b; sub u(.a, .b);\n\
           initial begin #1 $display(\"S=%b\", b); $finish; end endmodule\n"
    ));
    assert_ne!(
        ok2,
        Some(0),
        "`.a` shorthand with no declared `a` must stay a loud bind error:\n{o2}"
    );
}
