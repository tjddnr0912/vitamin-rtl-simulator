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

// ---------------------------------------------------------------------------
// V34-6 (round-34) — an interface INSTANCE is a declaration, not a §3.5 position.
//
// The §3.5 pass above walks every module-instance terminal list looking for bare
// undeclared idents. An interface instance passed as a port actual looks exactly like
// one to that walk: `simple_if bus(); child c(bus);` has `bus` as a bare ident, and
// `net_is_undeclared("bus")` is TRUE, because the interface flatten registers symbols
// for the MEMBERS (`t.bus.d`) and never for the bare instance name.
//
// Oracles: iverilog 13 is NOT one here — it cannot parse an interface PORT at all
// (`module child(simple_if s)` ⇒ "syntax error / Errors in port declarations"), and the
// "implicit definition of wire 'bus'" it then prints is a consequence of that parse
// failure, not a ruling. verilator 5.050 compiles the designs below with no warning at
// default settings and prints the values pinned in each test.
// ---------------------------------------------------------------------------

/// Run a source that dumps a VCD and return (stdout+stderr, exit code, VCD text).
/// Unlike `run` this reads the dump back before deleting the directory.
fn run_dumping(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r34_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let vcd = std::fs::read_to_string(d.join("v346.vcd")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code(), vcd)
}

const IFACE_SRC: &str = "interface simple_if; logic [7:0] d; endinterface\n\
     module child(simple_if s); initial #1 $display(\"d=%02h\", s.d); endmodule\n";

/// V34-6 (a) — the instance is declared one line above, so nothing about it is implicit.
/// PRE: `W-PARSE-IMPLICIT-NET: implicit net \`t.bus\` inferred as a 1-bit wire`, exit 0.
/// verilator: silent, `d=5a`.
#[test]
fn an_interface_instance_actual_is_not_an_implicit_net() {
    let (o, ok) = run(&format!(
        "{IFACE_SRC}module t;\n\
           simple_if bus();\n\
           child c(bus);\n\
           initial bus.d = 8'h5a;\n\
         endmodule\n"
    ));
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
    assert!(o.contains("d=5a"), "verilator gives d=5a:\n{o}");
    assert!(
        !o.contains("VITA-W2003"),
        "an interface instance is a declaration, not a §3.5 terminal:\n{o}"
    );
    assert!(
        !o.contains("t.bus"),
        "and nothing may name `t.bus` as an implicit net:\n{o}"
    );
}

/// V34-6 (b) — the PRE advice was unfollowable in BOTH directions: an interface instance
/// cannot be redeclared as a net, and `` `default_nettype none `` (which the message
/// promised would turn the warning into an error) made it VANISH instead, because
/// `declare_implicit_net` returns early under `cur_nettype_none` without diagnosing.
/// One design must not get two verdicts from a directive that changes nothing here.
#[test]
fn the_nettype_directive_does_not_change_the_interface_verdict() {
    let body = "module t;\n\
           simple_if bus();\n\
           child c(bus);\n\
           initial bus.d = 8'h5a;\n\
         endmodule\n";
    let (o1, ok1) = run(&format!("{IFACE_SRC}{body}"));
    let (o2, ok2) = run(&format!("`default_nettype none\n{IFACE_SRC}{body}"));
    assert_eq!(ok1, Some(0), "default nettype:\n{o1}");
    assert_eq!(ok2, Some(0), "`default_nettype none`:\n{o2}");
    assert!(
        o1.contains("d=5a") && o2.contains("d=5a"),
        "{o1}\n---\n{o2}"
    );
    assert!(
        !o1.contains("VITA-W2003") && !o2.contains("VITA-W2003"),
        "the directive must not decide whether this warns:\n{o1}\n---\n{o2}"
    );
}

/// V34-6 (c) — the PRE warning fired once per module that shared the bus, so the
/// canonical "N agents on one interface" design got N copies of a fake warning while a
/// local-only use of the same interface was clean. verilator: silent, and all three
/// children read `5a` (their relative print ORDER is IEEE §4.7 nondeterministic —
/// verilator prints 2,1,0 and vita 0,1,2 — so only the values are pinned).
#[test]
fn sharing_one_interface_across_modules_warns_zero_times() {
    let (o, ok) = run("interface simple_if; logic [7:0] d; endinterface\n\
         module rd(simple_if s, input [7:0] tag);\n\
           initial #1 $display(\"rd tag=%0d d=%02h\", tag, s.d);\n\
         endmodule\n\
         module t;\n\
           simple_if bus();\n\
           rd r0(bus, 8'd0);\n\
           rd r1(bus, 8'd1);\n\
           rd r2(bus, 8'd2);\n\
           initial bus.d = 8'h5a;\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
    for tag in 0..3 {
        assert!(
            o.contains(&format!("rd tag={tag} d=5a")),
            "every sharer reads 5a:\n{o}"
        );
    }
    assert_eq!(
        o.matches("VITA-W2003").count(),
        0,
        "N sharers used to mean N fake warnings:\n{o}"
    );
}

/// V34-6 side effect, pinned deliberately — the PRE run did not only warn, it CREATED a
/// 1-bit wire that never had a driver or a reader. It appeared in the waveform as
/// `$var wire 1 ! bus $end` sitting at `z` for the whole run, and because it took the
/// first id-code it shifted every real signal's code by one. Measured PRE / POST on this
/// exact source:
///
///   PRE : `$scope module t` → `$var wire 1 ! bus`, `$scope module bus` → `$var wire 8 " d`
///   POST: `$scope module t` → `$scope module bus` → `$var wire 8 ! d`
///
/// The phantom net is GONE on purpose; `net_count` for `t` drops by one per interface
/// instance. Nothing observable is lost — the interface port bind aliases SYMBOLS
/// (`bind_iface_port`), it never reads a net under the bare instance name.
#[test]
fn no_phantom_wire_for_the_interface_instance_in_the_vcd() {
    let (o, ok, vcd) = run_dumping(&format!(
        "{IFACE_SRC}module t;\n\
           simple_if bus();\n\
           child c(bus);\n\
           initial begin\n\
             $dumpfile(\"v346.vcd\"); $dumpvars(0, t);\n\
             bus.d = 8'h5a;\n\
             #2 $finish;\n\
           end\n\
         endmodule\n"
    ));
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
    assert!(o.contains("d=5a"), "value unchanged:\n{o}");
    assert!(!vcd.is_empty(), "a VCD must have been written:\n{o}");
    // The interface SCOPE and its member survive — only the phantom net is gone.
    assert!(
        vcd.contains("$scope module bus $end"),
        "the interface scope must still be dumped:\n{vcd}"
    );
    assert!(
        vcd.contains("$var wire 8 ! d [7:0] $end"),
        "the member keeps its width and now takes the FIRST id-code:\n{vcd}"
    );
    // `$scope module bus $end` also ends in " bus $end", so match the $var line itself.
    assert!(
        !vcd.lines()
            .any(|l| l.starts_with("$var ") && l.ends_with(" bus $end")),
        "no 1-bit phantom wire named `bus` may be declared:\n{vcd}"
    );
    assert_eq!(
        vcd.matches("$var ").count(),
        1,
        "exactly one variable in the whole design:\n{vcd}"
    );
}

/// The boundary: the skip is keyed on the interface-instance NAMES from this same module
/// body, so a genuinely undeclared terminal standing next to one still gets §3.5 — a
/// blanket "this instance has an interface port, skip it" rule would have swallowed it.
#[test]
fn a_real_implicit_net_beside_an_interface_instance_still_warns() {
    let (o, ok) = run("interface simple_if; logic [7:0] d; endinterface\n\
         module child(simple_if s, input w2);\n\
           initial #1 $display(\"d=%02h w2=%b\", s.d, w2);\n\
         endmodule\n\
         module t;\n\
           simple_if bus();\n\
           child c(bus, undeclared_w);\n\
           assign undeclared_w = 1'b1;\n\
           initial bus.d = 8'h5a;\n\
         endmodule\n");
    assert_eq!(ok, Some(0), "must elaborate cleanly:\n{o}");
    assert!(o.contains("d=5a w2=1"), "both halves must be right:\n{o}");
    assert!(
        o.contains("implicit net `t.undeclared_w`"),
        "the real §3.5 net must still be announced:\n{o}"
    );
    assert!(
        !o.contains("t.bus"),
        "and the interface instance must not be:\n{o}"
    );
}
