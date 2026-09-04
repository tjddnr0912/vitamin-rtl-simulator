//! An ARRAY parameter in the ANSI `#(…)` header — `module m #(parameter pmp_cfg_t
//! PMPRstCfg[PMP_MAX_REGIONS] = ibex_pkg::PmpCfgRst, …)` — ROADMAP §3 ⑤ ⓒ, the header
//! half (ibex_top.sv:22-23, ibex_core:22, ibex_cs_registers:26, ibex_lockstep:17):
//! a whole-array DEFAULT (`= pkg::Rst`, an imported bare name, a sibling header
//! array) and the instance OVERRIDE channel for aggregates (a `'{…}` pattern of
//! constants, `pkg::Arr`, the parent's own array parameter forwarded, a pattern of
//! the parent's elements, positional or named, an instance array, `defparam`-free).
//!
//! Mechanism: the parser keeps the A2a desugar (a header array parameter is the
//! same const variable-array decl a body `localparam` array is, placed at the front
//! of the body after the header imports) and adds a scalar `ParamDecl` TWIN of it
//! — same name, same `span`, the whole-array default as its value — to the module's
//! `params`, so the parameter occupies its override slot (positional counting,
//! §6.20.1 "a header exists" rule, `-G`/`defparam` name resolution) without a new
//! AST field. Elaborate recognises the pair (`array_param_twin`): an override is
//! folded as a whole array IN THE PARENT (`const_array_override_vals`), recorded
//! under the child's fq name (`bind_array_param`), and both consumers of the decl's
//! `'{…}` — the const-fold capture and the decl-init flush — read the recorded
//! values (or, for a non-pattern default, the named array's captured elements)
//! through one source (`array_param_vals_src`). Every element travels the 64-bit
//! lane: wider elements keep the literal-pattern path only (an override / whole-
//! array default of them is loud).
//!
//! Every value here was measured on verilator 5.050 (iverilog 13.0 rejects an
//! unpacked array parameter outright: "sorry: unpacked array parameters are not
//! supported yet"). 183 cells in the slice's census, with a body-`localparam`
//! keyword control and a variable twin per element type.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hap_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

/// Every `DIGEST=` line, in emission order, joined by ` | `.
fn digest(src: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    assert!(!v.is_empty(), "no DIGEST line:\n{out}");
    v.join(" | ")
}

fn loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "expected a loud reject:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

const PKG: &str = "package p;
  parameter logic [3:0] R[2] = '{4'd1, 4'd2};
  typedef struct packed { logic l; logic [1:0] m; } cfg_t;
  parameter cfg_t CR[2] = '{'{l:1'b1, m:2'd2}, '{l:1'b0, m:2'd3}};
  typedef enum logic [1:0] {E0, E1, E2, E3} en_t;
  parameter en_t ER[2] = '{E2, E1};
  parameter int SG[2] = '{-1, 7};
  parameter logic signed [3:0] SR[2] = '{-4, 3};
  parameter int N2 = 2;
endpackage
";

/// `module c #(parameter logic [3:0] A[2] = '{4'd3, 4'd5}) ();` + a DIGEST of both.
const C: &str = "module c #(parameter logic [3:0] A[2] = '{4'd3, 4'd5}) (); \
  initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n";

fn tb(inst: &str) -> String {
    format!("{PKG}{C}module tb; {inst} initial #1 $finish; endmodule\n")
}

#[test]
fn header_array_default_and_overrides() {
    // verilator: 3 5 (default) / 7 9 / 7 9 / 1 2 / 3 5 (`.A()` keeps the default)
    assert_eq!(digest(&tb("c u();")), "3 5");
    assert_eq!(digest(&tb("c #(.A('{4'd7, 4'd9})) u();")), "7 9");
    assert_eq!(digest(&tb("c #('{4'd7, 4'd9}) u();")), "7 9");
    assert_eq!(digest(&tb("c #(.A(p::R)) u();")), "1 2");
    assert_eq!(digest(&tb("c #(.A()) u();")), "3 5");
    // An instance array forwards the same override to every element.
    assert_eq!(digest(&tb("c #(.A('{4'd7, 4'd9})) u[1:0]();")), "7 9 | 7 9");
}

#[test]
fn header_array_forwarded_through_a_parent() {
    // verilator: 7 9 — the parent's header array (itself overridden) forwarded whole;
    // a parent BODY localparam array; a pattern of the parent's elements (9 7); a
    // pattern of the parent's scalar parameters (2 11).
    let mid =
        "module mid #(parameter logic [3:0] A[2] = '{4'd6, 4'd8}) (); c #(.A(A)) u(); endmodule\n";
    assert_eq!(
        digest(&format!(
            "{PKG}{C}{mid}module tb; mid #(.A('{{4'd7, 4'd9}})) m(); initial #1 $finish; endmodule\n"
        )),
        "7 9"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}{C}{mid}module tb; mid m(); initial #1 $finish; endmodule\n"
        )),
        "6 8"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}{C}module mid (); localparam logic [3:0] A[2] = '{{4'd7, 4'd9}}; c #(.A(A)) u(); endmodule\n\
             module tb; mid m(); initial #1 $finish; endmodule\n"
        )),
        "7 9"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}{C}module mid #(parameter logic [3:0] A[2] = '{{4'd7, 4'd9}}) (); c #(.A('{{A[1], A[0]}})) u(); endmodule\n\
             module tb; mid m(); initial #1 $finish; endmodule\n"
        )),
        "9 7"
    );
    assert_eq!(
        digest(&tb(
            "localparam logic [3:0] P0 = 4'd2; parameter P1 = 11; c #(.A('{P0, P1})) u();"
        )),
        "2 11"
    );
}

#[test]
fn header_array_whole_array_defaults() {
    // verilator: 1 2 — `= p::R` scoped, `= R` through a header `import p::*`, and a
    // sibling header array (`B[2] = A` → 3 5).
    assert_eq!(
        digest(&format!(
            "{PKG}module c #(parameter logic [3:0] A[2] = p::R) (); initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n\
             module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "1 2"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}module c import p::*; #(parameter logic [3:0] A[2] = R) (); initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n\
             module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "1 2"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}module c #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}, parameter logic [3:0] B[2] = A) (); initial $display(\"DIGEST=%0d %0d\", B[0], B[1]); endmodule\n\
             module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "3 5"
    );
    // A body `localparam` array may take a whole-array default too (was loud).
    assert_eq!(
        digest(&format!(
            "{PKG}module tb; localparam logic [3:0] L[2] = p::R; initial $display(\"DIGEST=%0d %0d\", L[0], L[1]); initial #1 $finish; endmodule\n"
        )),
        "1 2"
    );
}

#[test]
fn header_array_element_types() {
    let one = |decl: &str, dflt: &str, read: &str, inst: &str| {
        digest(&format!(
            "{PKG}module c #(parameter {decl} A[2] = {dflt}) (); initial $display(\"DIGEST={read}); endmodule\n\
             module tb; {inst} initial #1 $finish; endmodule\n"
        ))
    };
    // int: -3 5 / -7 9 / p::SG -1 7
    assert_eq!(
        one("int", "'{-3, 5}", "%0d %0d\", A[0], A[1]", "c u();"),
        "-3 5"
    );
    assert_eq!(
        one(
            "int",
            "'{-3, 5}",
            "%0d %0d\", A[0], A[1]",
            "c #(.A('{-7, 9})) u();"
        ),
        "-7 9"
    );
    assert_eq!(
        one(
            "int",
            "'{-3, 5}",
            "%0d %0d\", A[0], A[1]",
            "c #(.A(p::SG)) u();"
        ),
        "-1 7"
    );
    // int unsigned: 3-4 wraps at 32 bits
    assert_eq!(
        one(
            "int unsigned",
            "'{3, 5}",
            "%0d %0d\", A[0]-4, A[1]",
            "c u();"
        ),
        "4294967295 5"
    );
    // logic signed [3:0]: the override's elements are sign-read at 4 bits
    assert_eq!(
        one(
            "logic signed [3:0]",
            "'{-3, 5}",
            "%0d %0d\", A[0], A[1]",
            "c #(.A(p::SR)) u();"
        ),
        "-4 3"
    );
    assert_eq!(
        one(
            "logic signed [3:0]",
            "'{-3, 5}",
            "%0d %0d\", A[0], A[1]",
            "c #(.A('{-7, 7})) u();"
        ),
        "-7 7"
    );
    // a struct typedef (scoped): member reads desugar as for the body array
    let st = "%0d %0d %0d\", A[1].m, A[0].l, A[1]";
    assert_eq!(
        one(
            "p::cfg_t",
            "'{'{l:1'b1, m:2'd2}, '{l:1'b0, m:2'd3}}",
            st,
            "c u();"
        ),
        "3 1 3"
    );
    assert_eq!(one("p::cfg_t", "p::CR", st, "c u();"), "3 1 3");
    assert_eq!(
        one(
            "p::cfg_t",
            "'{'{l:1'b0, m:2'd0}, '{l:1'b0, m:2'd0}}",
            st,
            "c #(.A(p::CR)) u();"
        ),
        "3 1 3"
    );
    // an enum typedef
    assert_eq!(
        one(
            "p::en_t",
            "'{p::E2, p::E1}",
            "%0d %0d\", A[0], A[1]",
            "c u();"
        ),
        "2 1"
    );
    assert_eq!(
        one(
            "p::en_t",
            "'{p::E2, p::E1}",
            "%0d %0d\", A[0], A[1]",
            "c #(.A('{p::E3, p::E0})) u();"
        ),
        "3 0"
    );
    // >64-bit elements: the literal-pattern default works
    assert_eq!(
        one(
            "logic [95:0]",
            "'{96'h1, 96'h2}",
            "%0d %0d\", A[0][3:0], A[1][3:0]",
            "c u();"
        ),
        "1 2"
    );
}

#[test]
fn header_array_dims() {
    let one = |dims: &str, read: &str, inst: &str| {
        digest(&format!(
            "{PKG}module c #(parameter logic [3:0] A{dims} = '{{4'd3, 4'd5}}) (); initial $display(\"DIGEST={read}); endmodule\n\
             module tb; {inst} initial #1 $finish; endmodule\n"
        ))
    };
    // verilator: 7 9 for every dim spelling, left-to-right in declared index order
    assert_eq!(
        one(
            "[0:1]",
            "%0d %0d\", A[0], A[1]",
            "c #(.A('{4'd7, 4'd9})) u();"
        ),
        "7 9"
    );
    assert_eq!(
        one(
            "[1:0]",
            "%0d %0d\", A[1], A[0]",
            "c #(.A('{4'd7, 4'd9})) u();"
        ),
        "7 9"
    );
    assert_eq!(
        one(
            "[1:2]",
            "%0d %0d\", A[1], A[2]",
            "c #(.A('{4'd7, 4'd9})) u();"
        ),
        "7 9"
    );
    assert_eq!(
        one(
            "[p::N2]",
            "%0d %0d\", A[0], A[1]",
            "c #(.A('{4'd7, 4'd9})) u();"
        ),
        "7 9"
    );
    // a dim from an earlier header parameter
    assert_eq!(
        digest(&format!(
            "{PKG}module c2 #(parameter N = 2, parameter logic [3:0] A[N] = '{{4'd3, 4'd5}}) (); initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n\
             module tb; c2 #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        )),
        "7 9"
    );
    // a 2-D default works; a NESTED override pattern is loud (v1)
    let d2 = "module c #(parameter logic [3:0] A[2][2] = '{'{4'd1,4'd2},'{4'd3,4'd4}}) (); initial $display(\"DIGEST=%0d %0d\", A[1][0], A[0][1]); endmodule\n";
    assert_eq!(
        digest(&format!(
            "{PKG}{d2}module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "3 2"
    );
    loud(
        &format!("{PKG}{d2}module tb; c #(.A('{{'{{4'd5,4'd6}},'{{4'd7,4'd8}}}})) u(); initial #1 $finish; endmodule\n"),
        "is not a constant array",
    );
}

#[test]
fn header_array_consumers() {
    let one = |extra: &str, read: &str| {
        digest(&format!(
            "{PKG}module c #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}) (); {extra} initial $display(\"DIGEST={read}); endmodule\n\
             module tb; c #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        ))
    };
    // runtime index / $size + $bits / a constant element read / a generate-if on an
    // element / foreach / a continuous assign (7+9 at 4 bits = 0)
    assert_eq!(one("logic i = 1;", "%0d\", A[i]"), "9");
    assert_eq!(one("", "%0d %0d\", $size(A), $bits(A[0])"), "2 4");
    assert_eq!(one("localparam logic [3:0] E = A[1];", "%0d\", E"), "9");
    assert_eq!(
        one(
            "if (A[0] == 4'd7) begin : g localparam X = 1; end else begin : g localparam X = 2; end",
            "%0d\", g.X"
        ),
        "1"
    );
    assert_eq!(
        one(
            "int s = 0; initial begin foreach (A[k]) s += A[k]; end",
            "%0d\", s"
        ),
        "16"
    );
    assert_eq!(
        one("logic [3:0] w; assign w = A[1] + A[0];", "%0d\", w"),
        "0"
    );
    // an element as a CHILD override inside a generate-for (the ibex_cs_registers
    // shape `.ResetValue(PMPRstCfg[i])`), and as a port connection
    assert_eq!(
        digest(&format!(
            "{PKG}module leaf #(parameter logic [3:0] RV = 4'd0) (); initial $display(\"DIGEST=%0d\", RV); endmodule\n\
             module c #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}) (); for (genvar i = 1; i < 2; i++) begin : g leaf #(.RV(A[i])) l(); end endmodule\n\
             module tb; c #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        )),
        "9"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}module leaf (input logic [3:0] x); initial #0 $display(\"DIGEST=%0d\", x); endmodule\n\
             module c #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}) (); leaf l(.x(A[1])); endmodule\n\
             module tb; c #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        )),
        "9"
    );
    // a hierarchical read from the parent, and a block-local shadow (1) beside the
    // parameter (7)
    assert_eq!(
        digest(&tb(
            "c #(.A('{4'd7, 4'd9})) u(); initial $display(\"DIGEST=h%0d\", u.A[1]);"
        )),
        "h9 | 7 9"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}module c #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}) (); initial begin : b logic [3:0] A[2]; A[0] = 4'd1; #0 $display(\"DIGEST=%0d\", A[0]); end initial #0 $display(\"DIGEST=%0d\", A[0]); endmodule\n\
             module tb; c #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        )),
        "1 | 7"
    );
}

#[test]
fn header_array_override_slot_rules() {
    // A comma-list continuation inherits the array element type; positional
    // overrides count the array as ONE slot (verilator: 5 9 2 / 1 9 2 / 9 2 5).
    let grp = "module c #(parameter logic [3:0] A[2] = '{4'd3, 4'd5}, B[2] = '{4'd1, 4'd2}, S = 4'd6) (); initial $display(\"DIGEST=%0d %0d %0d\", A[1], B[1], S); endmodule\n";
    assert_eq!(
        digest(&format!("{PKG}{grp}module tb; c #(.B('{{4'd8, 4'd9}}), .S(4'd2)) u(); initial #1 $finish; endmodule\n")),
        "5 9 2"
    );
    assert_eq!(
        digest(&format!("{PKG}{grp}module tb; c #('{{4'd1,4'd1}}, '{{4'd8, 4'd9}}, 4'd2) u(); initial #1 $finish; endmodule\n")),
        "1 9 2"
    );
    assert_eq!(
        digest(&format!(
            "{PKG}module c #(parameter S = 4'd6, parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}, parameter T = 1) (); initial $display(\"DIGEST=%0d %0d %0d\", A[1], S, T); endmodule\n\
             module tb; c #(4'd2, '{{4'd8, 4'd9}}, 5) u(); initial #1 $finish; endmodule\n"
        )),
        "9 2 5"
    );
    // IEEE §6.20.1: with a header present, a BODY `parameter` is not overridable —
    // the header made of one array parameter alone still counts as a header
    // (verilator: "Instance attempts to override 'B' … but it is a local parameter").
    let hb = "module c #(parameter logic [3:0] A[2] = '{4'd3, 4'd5}) (); parameter B = 3; initial $display(\"DIGEST=%0d %0d\", A[1], B); endmodule\n";
    loud(
        &format!("{PKG}{hb}module tb; c #(.B(9)) u(); initial #1 $finish; endmodule\n"),
        "override of unknown parameter `B`",
    );
    assert_eq!(
        digest(&format!(
            "{PKG}{hb}module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "5 3"
    );
    // A `localparam` in the header takes no override.
    let lp = "module c #(localparam logic [3:0] A[2] = '{4'd3, 4'd5}) (); initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n";
    assert_eq!(
        digest(&format!(
            "{PKG}{lp}module tb; c u(); initial #1 $finish; endmodule\n"
        )),
        "3 5"
    );
    loud(
        &format!(
            "{PKG}{lp}module tb; c #(.A('{{4'd7, 4'd9}})) u(); initial #1 $finish; endmodule\n"
        ),
        "cannot override localparam `A`",
    );
}

#[test]
fn header_array_override_value_shapes() {
    // Elements are sized to the element type like the declared pattern's:
    // -1 → 15, 8'd200 → 8, a fill `'1` → 15 (verilator-measured).
    assert_eq!(digest(&tb("c #(.A('{-1, 4'd9})) u();")), "15 9");
    assert_eq!(digest(&tb("c #(.A('{4'd7, 8'd200})) u();")), "7 8");
    assert_eq!(digest(&tb("c #(.A('{'1, '0})) u();")), "15 0");
    assert_eq!(
        digest(&tb("parameter P1 = 11; c #(.A('{4'(P1), 4'd1})) u();")),
        "11 1"
    );
}

#[test]
fn header_array_loud_shapes() {
    // Every one of these is a verilator error too, or a v1 limit that must never
    // run with the declared default in silence.
    let needle = "the override of array parameter `A` is not a constant array";
    loud(&tb("c #(.A(4'd7)) u();"), needle);
    loud(
        &tb("logic [3:0] s = 4'd2; c #(.A('{s, 4'd9})) u();"),
        needle,
    );
    loud(&tb("c #(.A('{4'bx, 4'd9})) u();"), needle);
    loud(&tb("c #(.A('{'{4'd1}, 4'd2})) u();"), needle);
    loud(&tb("c #(.A(\"ab\")) u();"), needle);
    loud(&tb("parameter P1 = 11; c #(.A(P1)) u();"), needle);
    // an element count that does not match the declaration
    loud(
        &tb("c #(.A('{4'd7})) u();"),
        "1 element value(s) for an array of 2",
    );
    loud(
        &tb("c #(.A('{4'd7, 4'd8, 4'd9})) u();"),
        "3 element value(s) for an array of 2",
    );
    // `-G` and `defparam` carry a scalar and cannot target an array parameter
    let top = format!("{PKG}module tb #(parameter logic [3:0] A[2] = '{{4'd3, 4'd5}}) (); initial begin $display(\"DIGEST=%0d %0d\", A[0], A[1]); #1 $finish; end endmodule\n");
    let (out, rc) = run_args(&top, &["-G", "A=5"]);
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains(needle), "{out}");
    loud(
        &tb("c u(); defparam u.A = '{4'd7, 4'd9};"),
        "defparam: a non-constant override value is unsupported",
    );
    // >64-bit elements: an override / whole-array default is loud (i64 lane)
    let w = "module c #(parameter logic [95:0] A[2] = '{96'h1, 96'h2}) (); initial $display(\"DIGEST=%0d %0d\", A[0][3:0], A[1][3:0]); endmodule\n";
    loud(
        &format!(
            "{PKG}{w}module tb; c #(.A('{{96'h7, 96'h9}})) u(); initial #1 $finish; endmodule\n"
        ),
        "an element type wider than 64 bits is unsupported",
    );
    // an interface header, and a multi-dimensional packed element type
    loud(
        "interface ifc #(parameter logic [3:0] A[2] = '{4'd3, 4'd5}) (); endinterface\nmodule tb; ifc i(); initial begin $display(\"DIGEST=%0d\", i.A[1]); #1 $finish; end endmodule\n",
        "an array parameter is supported only in a module header",
    );
    loud(
        &tb("c u();").replace("logic [3:0] A[2]", "logic [1:0][1:0] A[2]"),
        "a one-dimensional packed element type on an array parameter",
    );
    // a whole-array default naming a VARIABLE array (verilator: "variable isn't
    // const"); the runtime copy would have read it before its own init (`x x`)
    loud(
        &format!(
            "{PKG}module c #(parameter logic [3:0] A[2] = R) (); logic [3:0] R[2] = '{{4'd6, 4'd7}}; initial $display(\"DIGEST=%0d %0d\", A[0], A[1]); endmodule\n\
             module tb; c u(); initial #1 $finish; endmodule\n"
        ),
        "the whole-array default `R` is not a constant array",
    );
    // a write to the parameter net stays loud through the A2a const mechanism
    loud(
        &tb("c u(); initial u.A[0] = 4'd1;"),
        "cannot assign to parameter `A`",
    );
}

#[test]
fn header_array_ibex_shape() {
    // The ibex_top → ibex_core forward chain with the package's reset tables as
    // the defaults, one instance overriding the address table from a tb-local
    // array. verilator-measured, both instances.
    let src = "package ibx;
  parameter int unsigned PMP_MAX_REGIONS = 4;
  parameter int unsigned PMP_ADDR_MSB = 33;
  typedef enum logic [1:0] { PMP_MODE_OFF = 2'b00, PMP_MODE_TOR = 2'b01, PMP_MODE_NA4 = 2'b10, PMP_MODE_NAPOT = 2'b11 } pmp_cfg_mode_e;
  typedef struct packed { logic lock; pmp_cfg_mode_e mode; logic exec; logic write; logic read; } pmp_cfg_t;
  parameter pmp_cfg_t PmpCfgRst[PMP_MAX_REGIONS] = '{
    '{lock: 1'b0, mode: PMP_MODE_OFF, exec: 1'b0, write: 1'b0, read: 1'b0},
    '{lock: 1'b1, mode: PMP_MODE_NAPOT, exec: 1'b1, write: 1'b0, read: 1'b1},
    '{lock: 1'b0, mode: PMP_MODE_TOR, exec: 1'b0, write: 1'b1, read: 1'b1},
    '{lock: 1'b1, mode: PMP_MODE_NA4, exec: 1'b1, write: 1'b1, read: 1'b1}};
  parameter logic [PMP_ADDR_MSB:0] PmpAddrRst[PMP_MAX_REGIONS] = '{34'h0, 34'h3_0000_0001, 34'h1234_5678, 34'hF};
endpackage
module core import ibx::*; #(
  parameter ibx::pmp_cfg_t PMPRstCfg[PMP_MAX_REGIONS] = ibx::PmpCfgRst,
  parameter logic [PMP_ADDR_MSB:0] PMPRstAddr[PMP_MAX_REGIONS] = ibx::PmpAddrRst
) ();
  logic [1:0] i = 2'd1;
  initial begin
    #0 $display(\"DIGEST=%0d %0d %0d %0d %0d %0d %0d\", PMPRstCfg[1].mode, PMPRstCfg[3].lock, PMPRstCfg[i], PMPRstAddr[1], PMPRstAddr[2][7:0], PMPRstAddr[i], $size(PMPRstCfg));
  end
endmodule
module top import ibx::*; #(
  parameter ibx::pmp_cfg_t PMPRstCfg[PMP_MAX_REGIONS] = ibx::PmpCfgRst,
  parameter logic [PMP_ADDR_MSB:0] PMPRstAddr[PMP_MAX_REGIONS] = ibx::PmpAddrRst
) ();
  core #(.PMPRstCfg(PMPRstCfg), .PMPRstAddr(PMPRstAddr)) u_core();
endmodule
module tb;
  localparam logic [ibx::PMP_ADDR_MSB:0] AR[ibx::PMP_MAX_REGIONS] = '{34'h3_FFFF_FFFF, 34'h11, 34'h22, 34'h33};
  top t0();
  top #(.PMPRstAddr(AR)) t1();
  initial #1 $finish;
endmodule
";
    assert_eq!(
        digest(src),
        "3 1 61 12884901889 120 12884901889 4 | 3 1 61 17 34 17 4"
    );
}
