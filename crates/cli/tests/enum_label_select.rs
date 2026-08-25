//! A BIT / PART / INDEXED-PART select of an **enum label**.
//!
//! `typedef enum logic [31:0] { EA = 32'hAB34 } e_t;` then `logic [EA[7:0]-1:0] v;`
//! declared a **one-bit** net at exit 0 where both oracles declare 52 — while the
//! RUNTIME read of the same `EA[7:0]` was already 52 in all three tools. The value
//! was never in doubt; the DECLARED WIDTH was, because `param_range` — the map that
//! answers *"is this constant's width a declared fact?"* — is written at parameter
//! declaration sites and a label is not one.
//!
//! ⭐ A label's width IS a declared fact: it comes from the enum's base type, never
//! from the label's value. `enum_base_range` reports it, and the three sites that
//! bind labels (module body, package, function/task body) record it beside the
//! `param_meta` they already wrote.
//!
//! ⭐⭐ The second prerequisite — *the value must be canonical at that width* — was
//! already established, by a gate written for another reason: `enum_label_range.rs`
//! makes a label outside its base's range a loud `E2002` (§6.19), and an unfoldable
//! base range makes `enum_base_range` decline. So no reachable label carries bits
//! outside the width this fold reads.
//!
//! ⚠️⚠️ **The oracles split on a non-zero-LSB base and that axis is untouched.** With
//! `enum logic [39:8] { EA = 32'hAB34 }`, `EA[15:8]` is **171** in iverilog — which
//! reads the label as a plain value of the base's WIDTH, indexed from 0 — and **52**
//! in verilator, which honours the declared LSB as both tools do for a NET of that
//! type. An ascending base is worse: iverilog rejects the declaration outright.
//! `enum_base_range` declines both, so those cells stay exactly where they were.
//!
//! Every value here is pinned to LIVE iverilog 13.0 and is also verilator 5.050's,
//! except where a cell is explicitly marked as a split or a single-oracle decline.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_enumsel_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

/// `EA[7:0]` is `0x34` = 52 for every one of these bases.
const BASES: [(&str, &str); 6] = [
    ("explicit packed range", "logic [31:0]"),
    ("int", "int"),
    ("byte", "byte"),
    ("base-less (int per §6.19)", ""),
    ("signed", "logic signed [31:0]"),
    ("narrow", "logic [7:0]"),
];

fn enum_decl(base: &str) -> String {
    let v = if base == "byte" || base == "logic [7:0]" {
        "8'h34"
    } else {
        "32'hAB34"
    };
    let sp = if base.is_empty() {
        String::new()
    } else {
        format!(" {base}")
    };
    format!("  typedef enum{sp} {{ EA = {v} }} e_t;\n")
}

#[test]
fn a_label_select_sizes_a_net_for_every_base() {
    for (what, base) in BASES {
        let (out, c) = run(&format!(
            "module top;\n{}  logic [EA[7:0]-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n",
            enum_decl(base)
        ));
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

/// The three scopes a label can be declared in reach the same table.
#[test]
fn a_label_select_folds_in_every_scope() {
    let e = enum_decl("logic [31:0]");
    for (what, src) in [
        (
            "module body",
            format!(
                "module top;\n{e}  logic [EA[7:0]-1:0] v;\n\
                   initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                 endmodule\n"
            ),
        ),
        (
            "package, `pk::EA`",
            format!(
                "package pk;\n{e}endpackage\n\
                 module top;\n  logic [pk::EA[7:0]-1:0] v;\n\
                   initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                 endmodule\n"
            ),
        ),
        (
            "package, wildcard-imported",
            format!(
                "package pk;\n{e}endpackage\nimport pk::*;\n\
                 module top;\n  logic [EA[7:0]-1:0] v;\n\
                   initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                 endmodule\n"
            ),
        ),
        (
            "generate scope",
            format!(
                "module top;\n{e}  generate if (1) begin : g\n\
                     logic [EA[7:0]-1:0] v;\n\
                     initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                   end endgenerate\n\
                 endmodule\n"
            ),
        ),
        (
            "submodule port list",
            format!(
                "module sub(o);\n{e}  output [EA[7:0]-1:0] o;\n\
                   initial begin $display(\"BITS=%0d\", $bits(o)); $finish; end\n\
                 endmodule\n\
                 module top; wire [200:0] w; sub u(.o(w)); endmodule\n"
            ),
        ),
    ] {
        let (out, c) = run(&src);
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

/// Every constant consumer of the fold. `$size` is here because its failure mode was
/// not a wrong number but an unpacked dimension of `x`.
#[test]
fn every_constant_consumer_of_a_label_select() {
    let e = enum_decl("logic [31:0]");
    for (what, body) in [
        (
            "unpacked dim",
            "logic v [EA[7:0]-1:0];\n\
             initial begin $display(\"BITS=%0d\", $size(v)); $finish; end\n",
        ),
        (
            "replication count",
            "initial begin $display(\"BITS=%0d\", $bits({EA[7:0]{1'b1}})); $finish; end\n",
        ),
        (
            "generate condition",
            "generate if (EA[7:0] == 52) begin : g\n\
               initial begin $display(\"BITS=52\"); $finish; end\n\
             end else begin : h\n\
               initial begin $display(\"BITS=BAD\"); $finish; end\n\
             end endgenerate\n",
        ),
    ] {
        let (out, c) = run(&format!("module top;\n{e}  {body}endmodule\n"));
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

/// ⚠️ The `localparam` consumer splits on SCOPE, and not because of this axis. A
/// module-scope body parameter binds at phase (3b) and the module's enum labels at
/// (3c), so `localparam int Q = EA;` — with no select at all — is already
/// *"undefined name `EA`"*, and adding a select changes nothing. The PACKAGE and
/// wildcard-imported spellings of the identical text fold, because a package's labels
/// are folded before any module body binds. Both oracles answer 52 for all three.
/// Pinned as-is so the ordering gap is visible rather than hidden inside this axis;
/// closing it is ROADMAP §3.
#[test]
fn a_module_scope_label_is_not_visible_to_a_body_localparam() {
    let e = enum_decl("logic [31:0]");
    let (out, c) = run(&format!(
        "module top;\n{e}  localparam int Q = EA;\n\
           initial begin $display(\"BITS=%0d\", Q); $finish; end\n\
         endmodule\n"
    ));
    assert_ne!(c, Some(0), "the ordering gap is loud, not silent:\n{out}");
    assert!(out.contains("undefined name `EA`"), "{out}");

    // …and the same text through a package folds, select and all.
    let (out, c) = run(&format!(
        "package pk;\n{e}endpackage\n\
         module top;\n  localparam int Q = pk::EA[7:0];\n\
           initial begin $display(\"BITS=%0d\", Q); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0), "{out}");
    assert!(out.contains("BITS=52"), "{out}");
}

/// A submodule parameter override taking a label select.
#[test]
fn a_label_select_may_override_an_instance_parameter() {
    let e = enum_decl("logic [31:0]");
    let (out, c) = run(&format!(
        "module sub #(parameter int N = 1) ();\n\
           initial begin $display(\"BITS=%0d\", N); $finish; end\n\
         endmodule\n\
         module top;\n{e}  sub #(.N(EA[7:0])) u();\nendmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=52"), "{out}");
}

/// Every select FORM, including the two whose failure value coincides with the
/// correct one unless the result is scaled — `EA[5]` is 1, and so is a clamped bound.
#[test]
fn every_select_form() {
    let e = enum_decl("logic [31:0]");
    for (what, sel) in [
        ("part", "EA[7:0]"),
        ("indexed part, ascending", "EA[0+:8]"),
        ("indexed part, descending", "EA[7-:8]"),
        ("bit", "EA[5]*52"),
        ("equal endpoints", "EA[5:5]*52"),
        ("nested", "EA[15:0][7:0]"),
    ] {
        let (out, c) = run(&format!(
            "module top;\n{e}  logic [{sel}-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

/// An IMPLICIT label — one that takes the running counter rather than an explicit
/// value — carries the same declared width.
#[test]
fn an_implicit_label_carries_the_base_width_too() {
    let (out, c) = run("module top;\n\
           typedef enum logic [31:0] { Z0 = 32'hAB33, EA } e_t;\n\
           logic [EA[7:0]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=52"), "{out}");
}

/// The sign bits above the declared width belong to the i64 container, not to the
/// value: a signed enum's negative label selects the masked byte.
#[test]
fn a_negative_label_of_a_signed_enum_selects_the_masked_byte() {
    let (out, c) = run("module top;\n\
           typedef enum logic signed [31:0] { EA = -32'sd52 } e_t;\n\
           logic [EA[15:8]-203:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=53"), "{out}");
}

// ── the cells that must not move ────────────────────────────────────────────

/// ⚠️⚠️ THE ORACLE SPLIT. Recording a non-zero-LSB base's range would install
/// verilator's reading (52) as vita's answer on an axis where iverilog says 171.
/// Declining leaves the cell where it was, and this pins that it stays there.
#[test]
fn a_non_zero_lsb_enum_base_is_not_this_slices_business() {
    let (out, c) = run("module top;\n\
           typedef enum logic [39:8] { EA = 32'hAB34 } e_t;\n\
           logic [EA[15:8]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    // iverilog 171 · verilator 52 · vita unchanged at the clamped 1.
    assert!(out.contains("BITS=1"), "non-zero-LSB enum base:\n{out}");
}

/// The ascending base is the same axis with only ONE oracle — iverilog rejects the
/// declaration outright — so it declines for the same reason.
#[test]
fn an_ascending_enum_base_declines_too() {
    let (out, c) = run("module top;\n\
           typedef enum logic [0:31] { EA = 32'hAB34 } e_t;\n\
           logic [EA[24:31]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=1"), "ascending enum base:\n{out}");
}

/// A local declaration shadowing a label wins — for the RANGE as much as the value.
/// The single binder (`bind_param_value` clears, `bind_param_range` sets) is what
/// makes that automatic rather than a rule each binder has to remember.
#[test]
fn an_inner_declaration_wins_the_labels_range() {
    let (out, c) = run("module top;\n\
           typedef enum logic [31:0] { EA = 32'hAB34 } e_t;\n\
           generate if (1) begin : g\n\
             localparam [39:8] EA = 32'h1122;\n\
             logic [EA[15:8]-1:0] v;\n\
             initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    // 0x1122 on `[39:8]`: declared bits 15..8 are `0x22` = 34.
    assert!(out.contains("BITS=34"), "inner declaration wins:\n{out}");
}

/// §6.19 rejects a label outside its base's range, and that gate is what makes the
/// value canonical at the width this fold reads. Both shapes stay loud.
#[test]
fn a_label_outside_its_base_range_stays_loud() {
    for (what, src) in [
        (
            "explicit value too wide",
            "module top;\n  typedef enum logic [3:0] { A = 8'hFF } e_t;\n\
               logic [A[3:0]*4-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n",
        ),
        (
            "auto-increment past the base width",
            "module top;\n  typedef enum logic [3:0] { A = 4'hF, B } e_t;\n\
               initial begin $display(\"BITS=%0d\", B); $finish; end\n\
             endmodule\n",
        ),
    ] {
        let (out, c) = run(src);
        assert_ne!(c, Some(0), "{what} should stay loud:\n{out}");
    }
}

/// A select reaching outside the declared range reads `x` (§11.5.1), which this
/// integer domain cannot represent — it declines, landing on iverilog's answer.
#[test]
fn an_out_of_range_label_select_declines() {
    let (out, c) = run(
        "module top;\n  typedef enum logic [7:0] { EA = 8'h34 } e_t;\n\
           logic [EA[15:8]+52-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=1"), "out-of-range label select:\n{out}");
}

/// The RUNTIME read was correct before this slice and must stay byte-identical.
#[test]
fn the_runtime_read_is_unchanged() {
    let e = enum_decl("logic [31:0]");
    let (out, c) = run(&format!(
        "module top;\n{e}  initial begin $display(\"V=%0d %0d\", EA[7:0], EA[15:8]); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("V=52 171"), "runtime label select:\n{out}");
}
