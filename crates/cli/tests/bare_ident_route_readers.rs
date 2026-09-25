//! ROADMAP §2 🆕 O — the `lookup_net_scoped` readers that resolved a bare name
//! without asking `bare_ident_route`.
//!
//! `lookup_net_scoped` walks `symbols` ALONE, so a generate-scope `localparam` /
//! `parameter` shadowing an outer NET does not stop it. Every reader built directly
//! on that walk therefore answered for the OUTER object while the whole-name read in
//! the same statement answered for the constant — one name, two objects, in one
//! design. Eleven readers were measured divergent against BOTH oracles; a twelfth
//! defect ran the other way (a speculative write lvalue refused a READ actual).
//!
//! They now share one funnel, `lookup_net_unshadowed`: a constant binding declines
//! the net walk, a site with a constant path takes it, and a site without one refuses
//! through `error_const_shadows_net`.
//!
//! Values are pinned to iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`); each test says what both oracles do with its cells.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_birr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, needle: &str) {
    let (s, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{s}");
    assert!(s.contains(needle), "expected `{needle}` in:\n{s}");
}

/// The §2 O refusal text, shared by every site that has no constant path.
const SHADOW: &str = "resolves to a constant";

// ── the SELECT OFFSET ────────────────────────────────────────────────

/// `norm_offset_if_net` normalized the offset against the SHADOWED net's declared
/// range while the value came from the constant, so `V[15:12]` answered
/// `99 >> (12 - 8)` = 6. Both oracles read bits 15:12 of 99 = 0.
#[test]
fn a_select_offset_normalizes_against_the_object_the_value_came_from() {
    let bit_range = run("module top;\n\
           logic [15:8] V;\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; $display(\"P=%h\", V[15:12]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(bit_range, "P=0\n");

    // The indexed-part twin of the same select, through the same normalizer.
    let indexed = run("module top;\n\
           logic [15:8] V;\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             logic [3:0] r;\n\
             initial begin #1; r = V[12 +: 4]; $display(\"R=%h\", r); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(indexed, "R=0\n");
}

/// The NO-SHADOW controls of the two cells above: the same non-zero-LSB net select
/// with the generate `localparam` deleted. Both oracles `P=a` / `R=a`, and the
/// normalizer must still subtract the declared LSB.
#[test]
fn the_unshadowed_non_zero_lsb_select_is_unchanged() {
    let bit_range = run("module top;\n\
           logic [15:8] V;\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             initial begin #1; $display(\"P=%h\", V[15:12]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(bit_range, "P=a\n");

    let indexed = run("module top;\n\
           logic [15:8] V;\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             logic [3:0] r;\n\
             initial begin #1; r = V[12 +: 4]; $display(\"R=%h\", r); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(indexed, "R=a\n");
}

// ── the EVENT CONTROL ────────────────────────────────────────────────

/// `lsb_bitselect_net` armed `@(posedge V[0])` on the SHADOWED net, so the process
/// woke at 2 when that net changed. Both oracles leave it blocked forever (only
/// `DONE` prints) — the bit came from a constant, which cannot change.
///
/// The CONSTANT EVENT PATH now exists (`events.rs::event_term_never_wakes`), so this
/// is no longer the refusal this slice first shipped: the term is dropped and the
/// process waits forever, which is what both oracles do. `DONE` alone, exit 0.
#[test]
fn an_edge_bit_select_through_a_constant_binding_no_longer_arms_the_outer_net() {
    let out = run("module top;\n\
           logic [7:0] V;\n\
           initial begin V = 8'h00; #2 V = 8'hFF; end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin @(posedge V[0]); $display(\"EDGE at %0t\", $time); end\n\
           end endgenerate\n\
           initial #5 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "DONE\n");
}

/// The control: the same event control with no shadow still arms the net and fires.
#[test]
fn an_unshadowed_lsb_edge_bit_select_still_arms_the_net() {
    let out = run("module top;\n\
           logic [7:0] V;\n\
           initial begin V = 8'h00; #2 V = 8'hFF; end\n\
           generate if (1) begin : g\n\
             initial begin @(posedge V[0]); $display(\"EDGE at %0t\", $time); end\n\
           end endgenerate\n\
           initial #5 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "EDGE at 2\nDONE\n");
}

/// The UNSHADOWED twin, which was the pre-existing FALSE LOUD this slice first
/// recorded: vita refused an event control on a constant whether or not a net
/// shadowed it, while both oracles accept the program and simply never wake the
/// process. Closing it is what closes the shadowed cell above — one constant event
/// path serves both — so the two are pinned together, `DONE` alone in all three
/// tools.
#[test]
fn a_constant_event_control_never_wakes_with_or_without_a_shadow() {
    let out = run("module top;\n\
           localparam int K = 99;\n\
           initial begin @(posedge K[0]); $display(\"EDGE\"); end\n\
           initial #5 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "DONE\n");
}

/// The PROCESS-HEADER twin of the two cells above, shadowed and unshadowed. This
/// is the spelling the slice REGRESSED (it ran on the baseline and printed `DONE`,
/// then refused), so it is pinned in both shapes. All three tools: `DONE` alone.
#[test]
fn a_header_edge_bit_select_on_a_constant_never_wakes() {
    let shadowed = run("module top;\n\
           logic [7:0] V = 8'hA5;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #2; $display(\"DONE\"); $finish; end\n\
             always @(posedge V[0]) $display(\"EDGE at %0t\", $time);\n\
           end endgenerate\n\
           initial #1 V = 8'hA6;\n\
         endmodule\n");
    assert_eq!(shadowed, "DONE\n");
    // The same design with the write CROSSING bit 0 (`A4` → `A5`): on the baseline
    // this printed `EDGE at 1` first — the silent-wrong half of the same cell.
    let crossing = run("module top;\n\
           logic [7:0] V = 8'hA4;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #2; $display(\"DONE\"); $finish; end\n\
             always @(posedge V[0]) $display(\"EDGE at %0t\", $time);\n\
           end endgenerate\n\
           initial #1 V = 8'hA5;\n\
         endmodule\n");
    assert_eq!(crossing, "DONE\n");
    let unshadowed = run("module top;\n\
           localparam int K = 99;\n\
           initial begin #2; $display(\"DONE\"); $finish; end\n\
           always @(posedge K[0]) $display(\"EDGE\");\n\
         endmodule\n");
    assert_eq!(unshadowed, "DONE\n");
}

/// The WHOLE-name edge spellings, which never reached the bit-select arm at all —
/// they went to `resolve_net` and came back E3010 "undeclared net/variable". Both
/// oracles print `DONE` alone for each.
#[test]
fn a_header_edge_on_a_bare_constant_never_wakes() {
    for edge in ["posedge", "negedge"] {
        let out = run(&format!(
            "module top;\n\
               localparam int K = 99;\n\
               initial begin #2; $display(\"DONE\"); $finish; end\n\
               always @({edge} K) $display(\"EDGE\");\n\
             endmodule\n"
        ));
        assert_eq!(out, "DONE\n", "@({edge} K)");
    }
    // `always_ff` takes the same lane through `force_edge`; the block never runs, so
    // `q` keeps its power-on value. iverilog `DONE q=x` (vita's answer); verilator
    // `DONE q=0`, its own x-initialisation, not this lane.
    let ff = run("module top;\n\
           localparam int K = 99;\n\
           logic q;\n\
           initial begin #2; $display(\"DONE q=%0d\", q); $finish; end\n\
           always_ff @(posedge K) q <= 1;\n\
         endmodule\n");
    assert_eq!(ff, "DONE q=x\n");
}

/// A constant term is DROPPED, not fatal to its siblings: the live term still arms.
/// All three tools print `EDGE at 1` then `DONE`.
#[test]
fn a_constant_term_beside_a_live_edge_term_leaves_the_live_one_armed() {
    let out = run("module top;\n\
           localparam int K = 99;\n\
           logic clk = 0;\n\
           initial begin #1 clk = 1; #2; $display(\"DONE\"); $finish; end\n\
           always @(posedge K or posedge clk) $display(\"EDGE at %0t\", $time);\n\
         endmodule\n");
    assert_eq!(out, "EDGE at 1\nDONE\n");
}

/// An IN-BODY wait is reached after the time-zero settle, so a NON-EDGE constant
/// term never wakes it either — both oracles print `DONE` alone, with no `NEVER`.
#[test]
fn an_in_body_level_wait_on_a_constant_never_wakes() {
    let out = run("module top;\n\
           localparam int K = 99;\n\
           initial begin @(K); $display(\"NEVER\"); end\n\
           initial begin #2; $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "DONE\n");
}

/// A PROCESS-HEADER non-edge term on a constant fires ONCE at time zero in both
/// oracles, because the header sensitivity is armed before the time-zero settle:
/// iverilog 13.0 and verilator 5.052 both print `EDGE at 0` then `DONE` for this
/// cell. vita REFUSES it (`error_header_level_const`) — BACK ON vita_pre's ROUTE after
/// a slice that ran it: vita's `$finish` ends a time step without running the
/// processes already woken in it, and a `$finish` can reach time 0 through any
/// process, so a list with no live term is never admitted. Beside a live term the
/// time-0 run happens (`const_level_event_t0.rs`). Pinned on the sentence that
/// names that reason, and on the constant, not `E3010 undeclared net/variable` (a
/// false sentence: `K` IS declared).
#[test]
fn a_header_level_term_on_a_constant_stays_loud() {
    loud(
        "module top;\n\
           localparam int K = 99;\n\
           initial begin #2; $display(\"DONE\"); $finish; end\n\
           always @(K) $display(\"EDGE at %0t\", $time);\n\
         endmodule\n",
        "a level event control on the constant `K`",
    );
}

/// The SHADOW twin of the cell above: a generate `localparam V` shadowing a module
/// net, read as a header LEVEL term. `lookup_net_scoped` walks `symbols` alone, so
/// vita once armed the OUTER NET and fired AGAIN when that net changed — `HDR fired
/// at 0` + `HDR fired at 1`, where iverilog 13.0 and verilator 5.052 both print `HDR
/// fired at 0` then `DONE`. It is refused with the §2 🆕 O sentence, which names the
/// object vita took — BACK ON vita_pre's ROUTE for the reason above (no live term).
#[test]
fn a_header_level_term_on_a_shadowing_constant_is_loud() {
    let src = "module top;\n\
           logic V;\n\
           initial begin V = 0; #1 V = 1; end\n\
           generate if (1) begin : g\n\
             localparam int V = 2;\n\
             always @(V) $display(\"HDR fired at %0t\", $time);\n\
           end endgenerate\n\
           initial #3 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n";
    loud(src, SHADOW);
    let (out, _ok) = vita(src);
    assert!(
        !out.contains("HDR fired at 1"),
        "must not arm the shadowed net:\n{out}"
    );
    // The CONTROL: the same design with the constant renamed arms the net and fires
    // at 0 and at 1 — all three tools.
    let out = run("module top;\n\
           logic V;\n\
           initial begin V = 0; #1 V = 1; end\n\
           generate if (1) begin : g\n\
             localparam int W = 2;\n\
             always @(V) $display(\"HDR fired at %0t\", $time);\n\
           end endgenerate\n\
           initial #3 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "HDR fired at 0\nHDR fired at 1\nDONE\n");
}

/// A constant LEVEL term beside a LIVE one is not refused — refusing it swallowed
/// the sibling: `generate … localparam int V = 99; always @(V or W)` with `W` a real
/// net was refused whole where all three tools run the design. Both oracles print
/// `HDR at 0` / `HDR at 1` / `DONE`. vita then DROPPED the term (the edge lane's
/// rule) and printed `HDR at 1` / `DONE`, the time-0 line lost at exit 0; the
/// time-0 lane (`const_level_header.rs`) adds it, so vita prints the oracles' text.
#[test]
fn a_constant_level_term_beside_a_live_one_is_dropped_not_refused() {
    let out = run("module top;\n\
           logic [7:0] V = 8'h0;\n\
           logic [7:0] W = 8'h0;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             always @(V or W) $display(\"HDR at %0t\", $time);\n\
           end endgenerate\n\
           initial begin #1 W = 8'h1; #1 $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "HDR at 0\nHDR at 1\nDONE\n");
    // The UNSHADOWED twin takes the same lane (the rule is keyed on the LIST, not on
    // shadowing): `localparam int K = 99; always @(K or clk)`. Both oracles print
    // `HDR at 0` / `HDR at 1` / `DONE`.
    let out = run("module top;\n\
           localparam int K = 99;\n\
           reg clk = 0;\n\
           always @(K or clk) $display(\"HDR at %0t\", $time);\n\
           initial begin #1 clk = 1; #1 $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "HDR at 0\nHDR at 1\nDONE\n");
    // When the live sibling is written at time zero the t0 line appeared before the
    // time-0 lane too (the write woke the process), and it still appears once —
    // iverilog and verilator print these same lines.
    let out = run("module top;\n\
           logic V, W;\n\
           initial begin V = 0; W = 0; #1 W = 1; #1 V = 1; end\n\
           generate if (1) begin : g\n\
             localparam int V = 2;\n\
             always @(V or W) $display(\"HDR fired at %0t\", $time);\n\
           end endgenerate\n\
           initial #3 begin $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "HDR fired at 0\nHDR fired at 1\nDONE\n");
}

/// The two shapes the level refusal must NOT reach.
#[test]
fn the_level_constant_refusal_stops_at_a_bare_head() {
    // A select of a constant as a LEVEL term was refused as a "single-bit level"
    // event control; both oracles run `always @(K[0] or clk)` and print `HDR at 0` /
    // `HDR at 1` / `DONE`. A select of a constant is a constant, so the time-0 lane
    // takes it and vita prints the same.
    let out = run("module top;\n\
           localparam logic [3:0] K = 4'h1;\n\
           reg clk = 0;\n\
           always @(K[0] or clk) $display(\"HDR at %0t\", $time);\n\
           initial begin #1 clk = 1; #1 $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "HDR at 0\nHDR at 1\nDONE\n");
    // A `$unit` ENUM LABEL with a module net of the same name: innermost-wins gives
    // the NET, nothing binds a constant, and the EDGE term arms. All three tools:
    // `EDGE at 1` then `DONE`. (The refusal is keyed on the BINDING, not on "a
    // constant of this name exists somewhere".)
    let out = run("typedef enum logic [7:0] { V = 8'h1 } e_t;\n\
         module top;\n\
           logic [7:0] V = 8'h0;\n\
           generate if (1) begin : g\n\
             always @(posedge V) $display(\"EDGE at %0t\", $time);\n\
           end endgenerate\n\
           initial begin #1 V = 8'h1; #1 $display(\"DONE\"); $finish; end\n\
         endmodule\n");
    assert_eq!(out, "EDGE at 1\nDONE\n");
}

/// The CONTROLS the constant path must not take with it: a real net still arms
/// (`EDGE at 1`), and a NON-LSB bit of a real net keeps its existing refusal.
#[test]
fn a_net_edge_bit_select_is_untouched_by_the_constant_path() {
    let fires = run("module top;\n\
           logic [7:0] V = 8'hA4;\n\
           initial begin #2; $display(\"DONE\"); $finish; end\n\
           always @(posedge V[0]) $display(\"EDGE at %0t\", $time);\n\
           initial #1 V = 8'hA5;\n\
         endmodule\n");
    assert_eq!(fires, "EDGE at 1\nDONE\n");
    loud(
        "module top;\n\
           logic [7:0] V = 8'hA4;\n\
           initial begin #2; $display(\"DONE\"); $finish; end\n\
           always @(posedge V[3]) $display(\"EDGE at %0t\", $time);\n\
           initial #1 V = 8'hAC;\n\
         endmodule\n",
        "edge event-control bit-select",
    );
}

// ── CONTAINER / ARRAY / STRING METHODS ───────────────────────────────

/// A method whose receiver binds a constant read and MUTATED the shadowed container.
/// Both oracles reject every cell here ("Object top.g.V has no method size(...)" /
/// "Member call on object 'VARREF 'V''"; "Enable of unknown task V.push_back").
#[test]
fn a_method_on_a_constant_binding_is_loud_for_every_receiver_kind() {
    // queue `.size()` — `dyn_handle` answered the outer `int V[$]` (`Q=2`).
    loud(
        "module top;\n\
           int V [$];\n\
           initial begin V.push_back(7); V.push_back(8); end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; $display(\"Q=%0d\", V.size()); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        SHADOW,
    );
    // queue `.push_back()` as a STATEMENT — this one WROTE the outer queue (`N=2`).
    loud(
        "module top;\n\
           int V [$];\n\
           initial V.push_back(1);\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; V.push_back(5); end\n\
           end endgenerate\n\
           initial begin #3; $display(\"N=%0d\", V.size()); #5 $finish; end\n\
         endmodule\n",
        SHADOW,
    );
    // §7.12.3 reduction with a REAL constant shadow — the hand-rolled `lookup_scoped`
    // guard saw the NUMERIC `params` map alone, so a real / string / wide localparam
    // sailed past it and `V.sum()` folded the outer `int V[0:3]` to `10`.
    loud(
        "module top;\n\
           int V [0:3];\n\
           initial begin V[0]=1; V[1]=2; V[2]=3; V[3]=4; end\n\
           generate if (1) begin : g\n\
             localparam real V = 2.5;\n\
             initial begin #1; $display(\"S=%0d\", V.sum()); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        SHADOW,
    );
    // the INT twin of the reduction: it was already loud through the old guard, and
    // it now says which object the name means instead of naming an instance path.
    loud(
        "module top;\n\
           int V [0:3];\n\
           initial begin V[0]=1; V[1]=2; V[2]=3; V[3]=4; end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; $display(\"S=%0d\", V.sum()); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        SHADOW,
    );
    // a string method whose receiver binds a NON-string constant: `V.len()` answered
    // the outer `string V`'s 5.
    loud(
        "module top;\n\
           string V;\n\
           initial V = \"hello\";\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; $display(\"L=%0d\", V.len()); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        SHADOW,
    );
}

/// A constant receiver with NO net of that name keeps the diagnostic it had — the
/// refusal above is gated on the SHADOW, because its sentence names a net.
#[test]
fn a_method_on_an_unshadowed_constant_keeps_its_own_diagnostic() {
    loud(
        "module top;\n\
           localparam int K = 99;\n\
           initial begin #1; $display(\"Q=%0d\", K.size()); end\n\
           initial #5 $finish;\n\
         endmodule\n",
        "unsupported hierarchical function call",
    );
}

/// The one method receiver that HAS a constant path: a `string` parameter is a string
/// VALUE, and `lower_string_method_expr_handle` already takes a literal `Const`
/// handle. Both oracles `L=2` — the same `$display` whose `%s` already printed `ab`.
#[test]
fn a_string_method_on_a_string_constant_reads_the_constant() {
    let shadowed = run("module top;\n\
           string V;\n\
           initial V = \"hello\";\n\
           generate if (1) begin : g\n\
             localparam string V = \"ab\";\n\
             initial begin #1; $display(\"L=%0d S=%s\", V.len(), V); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(shadowed, "L=2 S=ab\n");

    // …and with no net of that name at all, which was a FALSE loud before this arm
    // ("unsupported hierarchical function call `S.len`") against both oracles.
    let unshadowed = run("module top;\n\
           localparam string S = \"ab\";\n\
           initial begin #1; $display(\"L=%0d S=%s C=%0d\", S.len(), S, (S==\"ab\")); end\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(unshadowed, "L=2 S=ab C=1\n");

    // The control: no constant shadow, so the method reads the NET.
    let net = run("module top;\n\
           string V;\n\
           initial V = \"hello\";\n\
           generate if (1) begin : g\n\
             initial begin #1; $display(\"L=%0d S=%s\", V.len(), V); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(net, "L=5 S=hello\n");
}

// ── CLASS HANDLES ────────────────────────────────────────────────────

/// `resolve_class_member` read field `x` out of the outer class handle and printed
/// `X=42`. Both oracles reject (iverilog "Parameter name V can't have member names
/// (x)"; verilator "Member call on object 'VARREF 'V''").
#[test]
fn a_class_member_through_a_constant_binding_is_loud() {
    loud(
        "class C; int x; function new(); x = 42; endfunction endclass\n\
         module top;\n\
           C V;\n\
           initial V = new();\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; $display(\"X=%0d\", V.x); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        "undeclared hierarchical name `V.x`",
    );
}

/// The control: the same member read with no constant shadow still resolves.
#[test]
fn an_unshadowed_class_member_read_is_unchanged() {
    let out = run(
        "class C; int x; function new(); x = 42; endfunction endclass\n\
         module top;\n\
           C V;\n\
           initial V = new();\n\
           generate if (1) begin : g\n\
             initial begin #1; $display(\"X=%0d\", V.x); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
    );
    assert_eq!(out, "X=42\n");
}

// ── WRITES ───────────────────────────────────────────────────────────

/// The whole-array write surface. `V = '{9,9,9,9}` stored into the OUTER `int V[0:3]`
/// at exit 0; both oracles reject ("Could not find variable ``V'' in ``top.g''" /
/// "Storing to parameter variable 'V' in a context that is determined only at
/// runtime").
#[test]
fn a_whole_array_write_through_a_constant_binding_is_loud() {
    loud(
        "module top;\n\
           int V [0:3];\n\
           initial begin V[0]=1; V[1]=2; V[2]=3; V[3]=4; end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; V = '{9,9,9,9}; end\n\
           end endgenerate\n\
           initial begin #3; $display(\"M=%0d %0d\", V[0], V[1]); #5 $finish; end\n\
         endmodule\n",
        SHADOW,
    );
}

/// The control: the same whole-array write with no shadow still lands.
#[test]
fn an_unshadowed_whole_array_write_is_unchanged() {
    let out = run("module top;\n\
           int V [0:3];\n\
           initial begin V[0]=1; V[1]=2; V[2]=3; V[3]=4; end\n\
           generate if (1) begin : g\n\
             initial begin #1; V = '{9,9,9,9}; end\n\
           end endgenerate\n\
           initial begin #3; $display(\"M=%0d %0d\", V[0], V[1]); #5 $finish; end\n\
         endmodule\n");
    assert_eq!(out, "M=9 9\n");
}

// ── PORT / FORMAL ACTUALS ────────────────────────────────────────────

/// Three actual positions that wired the SHADOWED net: an unpacked-array module port,
/// an instance-ARRAY port, and a hierarchical task call's array formal. Both oracles
/// reject all three (the port ones on the constant's 32-bit width).
#[test]
fn an_actual_that_binds_a_constant_is_not_the_shadowed_net() {
    // unpacked-array port — wired the outer `int V[0:1]` element by element.
    loud(
        "module ch(input int p [0:1]); initial begin #2; $display(\"C=%0d %0d\", p[0], p[1]); end endmodule\n\
         module top;\n\
           int V [0:1];\n\
           initial begin V[0]=10; V[1]=20; end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             ch u (.p(V));\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        SHADOW,
    );
    // instance ARRAY port — sliced the outer `logic [1:0] V` and printed `O=11`.
    loud(
        "module ch(input p, output q); assign q = p; endmodule\n\
         module top;\n\
           logic [1:0] V; logic [1:0] o;\n\
           initial V = 2'b10;\n\
           generate if (1) begin : g\n\
             localparam int V = 3;\n\
             ch u [1:0] (.p(V), .q(o));\n\
           end endgenerate\n\
           initial begin #1; $display(\"O=%b\", o); #5 $finish; end\n\
         endmodule\n",
        SHADOW,
    );
    // hierarchical task call, unpacked-array formal — forwarded the outer array.
    loud(
        "module top;\n\
           int V [0:3];\n\
           task show(input int a [0:3]); $display(\"T=%0d %0d\", a[0], a[1]); endtask\n\
           initial begin V[0]=10; V[1]=20; V[2]=30; V[3]=40; end\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; top.show(V); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        "an array formal needs a bare whole-array actual",
    );
}

/// The controls: the same three actuals with no constant shadow still wire.
#[test]
fn unshadowed_actuals_are_unchanged() {
    let port = run("module ch(input int p [0:1]); initial begin #2; $display(\"C=%0d %0d\", p[0], p[1]); end endmodule\n\
         module top;\n\
           int V [0:1];\n\
           initial begin V[0]=10; V[1]=20; end\n\
           generate if (1) begin : g\n\
             ch u (.p(V));\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(port, "C=10 20\n");

    let inst_arr = run("module ch(input p, output q); assign q = p; endmodule\n\
         module top;\n\
           logic [1:0] V; logic [1:0] o;\n\
           initial V = 2'b10;\n\
           generate if (1) begin : g\n\
             ch u [1:0] (.p(V), .q(o));\n\
           end endgenerate\n\
           initial begin #1; $display(\"O=%b\", o); #5 $finish; end\n\
         endmodule\n");
    assert_eq!(inst_arr, "O=10\n");

    let arr_formal = run("module top;\n\
           int V [0:3];\n\
           task show(input int a [0:3]); $display(\"T=%0d %0d\", a[0], a[1]); endtask\n\
           initial begin V[0]=10; V[1]=20; V[2]=30; V[3]=40; end\n\
           generate if (1) begin : g\n\
             initial begin #1; top.show(V); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(arr_formal, "T=10 20\n");
}

// ── the OPPOSITE direction: a speculative lvalue that spoke ──────────

/// A deferred hierarchical task enable lowers EVERY actual as a candidate copy-OUT
/// target, because the callee's port directions do not exist yet. `lower_lvalue` then
/// reached the write guard and refused a READ actual: `top.sh(V[7:0])` with an
/// INPUT-only formal was a hard E3009 where both oracles print the value — while the
/// LOCAL-call and NO-shadow spellings of the same line ran.
#[test]
fn a_read_actual_of_a_hierarchical_call_is_not_judged_as_a_write() {
    let part_select = run("module top;\n\
           logic [15:0] V;\n\
           task automatic sh(input [7:0] a); $display(\"A=%h\", a); endtask\n\
           initial V = 16'hA5C3;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; top.sh(V[7:0]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    // Both oracles `A=63` — bits 7:0 of the CONSTANT 99.
    assert_eq!(part_select, "A=63\n");

    // A mixed call: `z` is the inout actual, `V` the input one.
    let mixed = run("module top;\n\
           logic [7:0] V;\n\
           task automatic bump(inout logic [7:0] a, input logic [7:0] b); a = a + b; endtask\n\
           initial V = 8'h10;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             logic [7:0] z;\n\
             initial begin #1; z = 8'h01; top.bump(z, V); $display(\"Z=%h\", z); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    // Both oracles `Z=64` — 1 + 99.
    assert_eq!(mixed, "Z=64\n");

    // A default-arg call and a nested enable through the same defer.
    let defaulted = run("module top;\n\
           logic [7:0] V;\n\
           task automatic sh(input [7:0] a = V); $display(\"A=%h\", a); endtask\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             initial begin #1; top.sh(8'h11); sh_local(); end\n\
             task automatic sh_local(); top.sh(V); endtask\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(defaulted, "A=11\nA=63\n");
}

/// The WRITE twin must stay loud: an OUTPUT/INOUT formal bound to a name that binds a
/// constant. Both oracles reject ("Could not find variable ``V'' in ``top.g''" /
/// "Storing to parameter variable 'V'"); only the reason vita gives moved, from the
/// call site's write guard to the resolver that actually knows the direction.
#[test]
fn an_output_formal_bound_to_a_constant_binding_stays_loud() {
    loud(
        "module top;\n\
           logic [7:0] V;\n\
           task automatic bump(inout logic [7:0] a, input logic [7:0] b); a = a + b; endtask\n\
           initial V = 8'h10;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             logic [7:0] z;\n\
             initial begin #1; z = 8'h01; top.bump(V, z); $display(\"Z=%h\", z); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n",
        "an output/inout argument must be a writable net or select",
    );
}

/// The controls for the same class: a LOCAL call with the identical actual, and the
/// hierarchical call with NO shadow. Both ran before this slice and must not move.
#[test]
fn the_local_call_and_no_shadow_controls_are_unchanged() {
    let local_call = run("module top;\n\
           logic [15:0] V;\n\
           initial V = 16'hA5C3;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             task automatic sh(input [7:0] a); $display(\"A=%h\", a); endtask\n\
             initial begin #1; sh(V[7:0]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(local_call, "A=63\n");

    let no_shadow = run("module top;\n\
           logic [15:0] W;\n\
           task automatic sh(input [7:0] a); $display(\"A=%h\", a); endtask\n\
           initial W = 16'hA5C3;\n\
           generate if (1) begin : g\n\
             initial begin #1; top.sh(W[7:0]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(no_shadow, "A=c3\n");

    let local_inout = run("module top;\n\
           logic [7:0] V;\n\
           initial V = 8'h10;\n\
           generate if (1) begin : g\n\
             localparam int V = 99;\n\
             logic [7:0] z;\n\
             task automatic bump(inout logic [7:0] a, input logic [7:0] b); a = a + b; endtask\n\
             initial begin #1; z = 8'h01; bump(z, V); $display(\"Z=%h\", z); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(local_inout, "Z=64\n");
}

// ── the axis this slice does NOT close ───────────────────────────────

/// An ENUM LABEL declared in the generate scope does not shadow the outer net in vita
/// at all — not in the readers this slice fixed and not in the whole-name read either.
/// Measured on every one of this file's shapes: the enum twin is byte-identical before
/// and after. The oracles split on it (iverilog reads the NET, verilator the LABEL),
/// so it stays a recorded split rather than a side to pin; this test only proves the
/// slice did not move it.
#[test]
fn an_enum_label_shadow_is_untouched_by_this_slice() {
    let select = run("module top;\n\
           logic [15:8] V;\n\
           initial V = 8'hA5;\n\
           generate if (1) begin : g\n\
             typedef enum int { V = 2 } e_t;\n\
             initial begin #1; $display(\"P=%h\", V[15:12]); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    // iverilog `P=a` (the net, which is vita's answer), verilator `P=0` (the label).
    assert_eq!(select, "P=a\n");

    let string_method = run("module top;\n\
           string V;\n\
           initial V = \"hello\";\n\
           generate if (1) begin : g\n\
             typedef enum int { V = 2 } e_t;\n\
             initial begin #1; $display(\"L=%0d S=%s\", V.len(), V); end\n\
           end endgenerate\n\
           initial #5 $finish;\n\
         endmodule\n");
    assert_eq!(string_method, "L=5 S=hello\n");
}
