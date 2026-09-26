//! A LEVEL event control on a constant select wakes when the selected value changes
//! (IEEE 1800-2017 §9.4.2).
//!
//! `@(n[0])`, `@(n[3:2])`, `@(n[I+:2])`, `@(a[1])` and `@(a[1][0])` were E3009: vita's level
//! wait fires on any change of the nets it names, and the frozen `EdgeTerm` / `WaitCause::Level`
//! carry no bit field. elaborate now derives a net per select (`Elaborator::level_select_net`:
//! `$ia_tmp$<n>` driven by `assign … = <select>;`) and the waiter watches it. The continuous
//! assign moves the net only when the selected bits change, and a constant slice or word is a
//! copy net, so time 0 invents no transition.
//!
//! Every value is iverilog 13.0's. verilator 5.052 agrees after time 0 and additionally runs
//! every header level `always` once at time 0 (ROADMAP "Oracle splits"); it is 2-state, so it
//! misses the x transitions. Every cell asserts native = interp = vm.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, backend: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_lse_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), s)
}

fn t_lines(s: &str) -> Vec<String> {
    s.lines()
        .filter(|l| l.starts_with('T'))
        .map(|l| l.trim_end().to_string())
        .collect()
}

/// Native must print `want`; the interpreter and the VM must print the same.
fn check(src: &str, want: &[&str]) {
    let (ok, out) = run(src, "native");
    assert!(ok, "expected exit 0, got:\n{out}");
    let native = t_lines(&out);
    assert_eq!(native, want, "native:\n{out}");
    for be in ["interp", "vm"] {
        let (ok, out) = run(src, be);
        assert!(ok, "{be}: expected exit 0, got:\n{out}");
        assert_eq!(t_lines(&out), native, "{be} disagrees with native");
    }
}

fn refused(src: &str, text: &str) {
    for be in ["native", "interp", "vm"] {
        let (ok, out) = run(src, be);
        assert!(!ok, "{be}: expected a refusal, got:\n{out}");
        assert!(
            out.contains("VITA-E3009") && out.contains(text),
            "{be}: expected E3009 naming `{text}`:\n{out}"
        );
    }
}

/// A stimulus over `n`: bit 0 up, bit 1 up, the same value, bit 0 down, high bits only, then
/// bits 0 and 2 up with bit 1 down.
const STIM: &str = "  initial begin
    #1 n = 8'h01;
    #1 n = 8'h03;
    #1 n = 8'h03;
    #1 n = 8'h02;
    #1 n = 8'hf2;
    #1 n = 8'h0d;
    #1 $finish;
  end
";

fn with_stim(decl: &str, waiter: &str) -> String {
    format!("module t;\n  {decl}\n  {waiter}\n{STIM}endmodule\n")
}

#[test]
fn a_header_level_select_wakes_on_its_bits_only() {
    let n = "logic [7:0] n = 8'h00;";
    let w = |sel: &str| format!("always @({sel}) $display(\"T %0t W n=%h\", $time, n);");
    check(
        &with_stim(n, &w("n[0]")),
        &["T 1 W n=01", "T 4 W n=02", "T 6 W n=0d"],
    );
    check(&with_stim(n, &w("n[1]")), &["T 2 W n=03", "T 6 W n=0d"]);
    check(
        &with_stim(n, &w("n[1:0]")),
        &["T 1 W n=01", "T 2 W n=03", "T 4 W n=02", "T 6 W n=0d"],
    );
    check(
        &with_stim(&format!("{n} localparam I = 2;"), &w("n[I+:2]")),
        &["T 6 W n=0d"],
    );
}

#[test]
fn an_in_body_level_select_wakes_on_its_bits_only() {
    let n = "logic [7:0] n = 8'h00;";
    let w = |sel: &str| {
        format!("initial forever begin @({sel}); $display(\"T %0t W n=%h\", $time, n); end")
    };
    check(
        &with_stim(n, &w("n[0]")),
        &["T 1 W n=01", "T 4 W n=02", "T 6 W n=0d"],
    );
    check(&with_stim(n, &w("n[3:2]")), &["T 6 W n=0d"]);
}

/// A select beside a live term, in a comma list, two selects in one list, and a guard.
#[test]
fn a_select_in_a_list_or_under_a_guard() {
    let stim_m = STIM.replace("#1 $finish;", "#1 m = 1;\n    #1 $finish;");
    check(
        &format!(
            "module t;\n  logic [7:0] n = 8'h00; logic m = 0;\n  always @(n[0] or m) \
             $display(\"T %0t W n=%h m=%b\", $time, n, m);\n{stim_m}endmodule\n"
        ),
        &[
            "T 1 W n=01 m=0",
            "T 4 W n=02 m=0",
            "T 6 W n=0d m=0",
            "T 7 W n=0d m=1",
        ],
    );
    let n = "logic [7:0] n = 8'h00;";
    check(
        &with_stim(
            n,
            "always @(n[0], n[1]) $display(\"T %0t C n=%h\", $time, n);",
        ),
        &["T 1 C n=01", "T 2 C n=03", "T 4 C n=02", "T 6 C n=0d"],
    );
    check(
        &with_stim(
            n,
            "always @(n[7:4] or n[0]) $display(\"T %0t OR n=%h\", $time, n);",
        ),
        &["T 1 OR n=01", "T 4 OR n=02", "T 5 OR n=f2", "T 6 OR n=0d"],
    );
    // iverilog cannot parse `iff`; verilator after time 0.
    check(
        &with_stim(
            &format!("{n} logic en = 1;"),
            "always @(n[0] iff en) $display(\"T %0t IFF n=%h\", $time, n);",
        ),
        &["T 1 IFF n=01", "T 4 IFF n=02", "T 6 IFF n=0d"],
    );
}

/// A wire, a 2-state `bit` vector, a non-zero-LSB range and a packed struct member.
#[test]
fn every_vector_kind_behaves_the_same() {
    let w = "always @(n[0]) $display(\"T %0t W n=%h\", $time, n);";
    let want = ["T 1 W n=01", "T 4 W n=02", "T 6 W n=0d"];
    check(&with_stim("bit [7:0] n = 8'h00;", w), &want);
    check(
        &with_stim(
            "logic [15:8] n = 8'h00;",
            "always @(n[8]) $display(\"T %0t W n=%h\", $time, n);",
        ),
        &want,
    );
    check(
        &format!(
            "module t;\n  logic [7:0] r = 8'h00; wire [7:0] n = r;\n  {w}\n{}endmodule\n",
            STIM.replace(" n = ", " r = ")
        ),
        &want,
    );
    check(
        &with_stim(
            "typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t n = 0;",
            "always @(n.b) $display(\"T %0t S n=%h\", $time, n);",
        ),
        &["T 1 S n=01", "T 2 S n=03", "T 4 S n=02", "T 6 S n=0d"],
    );
}

/// An unpacked element and a bit of one; a time-0 write wakes, a 2-state or initialized array's
/// default does not.
#[test]
fn an_element_select_wakes_on_its_element() {
    check(
        "module t;\n  logic [3:0] a [0:1];\n  always @(a[1]) $display(\"T %0t W a1=%h\", $time, a[1]);\n  \
         initial begin\n    a[0] = 0; a[1] = 0;\n    #1 a[0] = 4'h1;\n    #1 a[1] = 4'h2;\n    \
         #1 a[1] = 4'h2;\n    #1 a[1] = 4'h3;\n    #1 $finish;\n  end\nendmodule\n",
        &["T 0 W a1=0", "T 2 W a1=2", "T 4 W a1=3"],
    );
    check(
        "module t;\n  logic [3:0] a [0:1];\n  always @(a[1][0]) $display(\"T %0t W a1=%h\", $time, a[1]);\n  \
         initial begin\n    a[0] = 0; a[1] = 0;\n    #1 a[0] = 4'h1;\n    #1 a[1] = 4'h2;\n    \
         #1 a[1] = 4'h3;\n    #1 a[1] = 4'h1;\n    #1 $finish;\n  end\nendmodule\n",
        &["T 0 W a1=0", "T 3 W a1=3"],
    );
    for decl in ["bit [3:0] a [0:1];", "logic [3:0] a [0:1] = '{4'd1, 4'd2};"] {
        check(
            &format!(
                "module t;\n  {decl}\n  always @(a[1][0]) $display(\"T %0t W a1=%h\", $time, a[1]);\n  \
                 initial begin\n    #1 a[1] = 4'h3;\n    #1 $finish;\n  end\nendmodule\n"
            ),
            &["T 1 W a1=3"],
        );
    }
}

/// x transitions (iverilog; verilator is 2-state), a time-0 write, a same-process glitch that
/// returns before the waiter runs (invisible) against one split by `#0` (two wakes), and an
/// NBA-driven counter.
#[test]
fn transitions_glitches_and_nonblocking_writes() {
    let w = "always @(n[0]) $display(\"T %0t W n=%h\", $time, n);";
    check(
        &format!(
            "module t;\n  logic [7:0] n;\n  {w}\n  initial begin\n    #1 n = 8'h00;\n    #1 n = 8'h01;\n    \
             #1 n = 8'hx1;\n    #1 n = 8'h0x;\n    #1 $finish;\n  end\nendmodule\n"
        ),
        &["T 1 W n=00", "T 2 W n=01", "T 4 W n=0x"],
    );
    check(
        &format!(
            "module t;\n  logic [7:0] n = 8'h00;\n  {w}\n  initial begin\n    n = 8'h01;\n    \
             #1 n = 8'h00;\n    #1 $finish;\n  end\nendmodule\n"
        ),
        &["T 0 W n=01", "T 1 W n=00"],
    );
    check(
        &format!(
            "module t;\n  logic [7:0] n = 8'h00;\n  {w}\n  initial begin\n    #1 n = 8'h01; n = 8'h00;\n    \
             #1 n = 8'h01; #0 n = 8'h00;\n    #1 $finish;\n  end\nendmodule\n"
        ),
        &["T 2 W n=01", "T 2 W n=00"],
    );
    check(
        &format!(
            "module t;\n  logic [7:0] n = 8'h00; logic clk = 0;\n  {w}\n  always #1 clk = ~clk;\n  \
             always @(posedge clk) n <= n + 1;\n  initial #9 $finish;\nendmodule\n"
        ),
        &[
            "T 1 W n=01",
            "T 3 W n=02",
            "T 5 W n=03",
            "T 7 W n=04",
            "T 9 W n=05",
        ],
    );
}

/// Time 0 over a partly-defined source (iverilog; verilator is 2-state): a copy of all-x bits
/// beside a bit that moved does not wake, a copy of a `z` does, once. The derived holder starts
/// at `x`; from a wire's `z` default both answers inverted (vita's `wire s = vv[0];` twin keeps
/// that recorded class).
#[test]
fn time_zero_over_a_partly_defined_source() {
    let count = |decl: &str, sens: &str| {
        format!(
            "module t;\n  {decl}\n  integer c = 0; always @({sens}) c = c + 1;\n  \
             initial #1 begin $display(\"T c=%0d\", c); $finish; end\nendmodule\n"
        )
    };
    check(&count("wire [1:0] vv = 2'b1x;", "vv[0]"), &["T c=0"]);
    check(&count("wire [1:0] vv = 2'b1z;", "vv[0]"), &["T c=1"]);
    check(
        "module t;\n  reg r;\n  wire [1:0] bus;\n  assign bus[1] = 1'b1;\n  assign bus[0] = r;\n  \
         always @(bus[0]) $display(\"T %0t B0 bus=%b\", $time, bus);\n  \
         initial begin #1 r = 0; #1 r = 1; #1 $finish; end\nendmodule\n",
        &["T 1 B0 bus=10", "T 2 B0 bus=11"],
    );
}

/// The waiter's own write to another bit of the base does not re-trigger it.
#[test]
fn a_write_to_other_bits_does_not_retrigger() {
    check(
        "module t;\n  logic [7:0] n = 0;\n  always @(n[0]) begin n[1] = ~n[1]; \
         $display(\"T %0t SELF n=%h\", $time, n); end\n  initial begin\n    #1 n[0] = 1;\n    \
         #1 n[0] = 0;\n    #1 $finish;\n  end\nendmodule\n",
        &["T 1 SELF n=03", "T 2 SELF n=00"],
    );
}

/// The positions a select can sit in: a static and an automatic task body, a generate loop,
/// a fork branch, a net driven through a port, a child instance's net, an interface port.
#[test]
fn every_position_takes_the_derived_net() {
    let stim4 = "  initial begin\n    #1 n = 8'h01;\n    #1 n = 8'h03;\n    #1 n = 8'h02;\n    \
                 #1 n = 8'hf2;\n    #1 $finish;\n  end\n";
    for task in ["task", "task automatic"] {
        check(
            &format!(
                "module t;\n  logic [7:0] n = 0;\n  {task} tw(); @(n[0]); \
                 $display(\"T %0t TW n=%h\", $time, n); endtask\n  initial forever tw();\n\
                 {stim4}endmodule\n"
            ),
            &["T 1 TW n=01", "T 3 TW n=02"],
        );
    }
    check(
        &format!(
            "module t;\n  logic [7:0] n = 0;\n  for (genvar g = 0; g < 2; g++) begin : G \
             always @(n[g]) $display(\"T %0t G%0d n=%h\", $time, g, n); end\n{stim4}endmodule\n"
        ),
        &["T 1 G0 n=01", "T 2 G1 n=03", "T 3 G0 n=02"],
    );
    check(
        &format!(
            "module t;\n  logic [7:0] n = 0;\n  initial begin fork begin @(n[1]); \
             $display(\"T %0t F1 n=%h\", $time, n); end begin @(n[0]); \
             $display(\"T %0t F0 n=%h\", $time, n); end join end\n{stim4}endmodule\n"
        ),
        &["T 1 F0 n=01", "T 2 F1 n=03"],
    );
    check(
        &format!(
            "module drv(output [7:0] o, input [7:0] i); assign o = i; endmodule\n\
             module t;\n  wire [7:0] n; logic [7:0] r = 0; drv d(.o(n), .i(r));\n  \
             always @(n[0]) $display(\"T %0t P n=%h\", $time, n);\n{}endmodule\n",
            stim4.replace(" n = ", " r = ")
        ),
        &["T 1 P n=01", "T 3 P n=02"],
    );
    check(
        &format!(
            "module sub; logic [7:0] v = 0; endmodule\nmodule t;\n  sub u();\n  \
             always @(u.v[0]) $display(\"T %0t U v=%h\", $time, u.v);\n{}endmodule\n",
            stim4.replace(" n = ", " u.v = ")
        ),
        &["T 1 U v=01", "T 3 U v=02"],
    );
    // iverilog cannot parse an interface port: hand-IEEE, bit 1 changes at 2 and 3.
    check(
        "interface bus_if; logic [7:0] data = 0; endinterface\n\
         module m(bus_if b);\n  always @(b.data[1]) $display(\"T %0t IF data=%h\", $time, b.data);\n\
         endmodule\nmodule t;\n  bus_if bi();\n  m u(.b(bi));\n  initial begin #1 bi.data = 8'h01; \
         #1 bi.data = 8'h03; #1 bi.data = 8'h01; #1 $finish; end\nendmodule\n",
        &["T 2 IF data=03", "T 3 IF data=01"],
    );
}

/// A select's waiter and a whole-net waiter of the same change resume in declaration order —
/// verilator's order. iverilog resumes the select's waiter second whatever the order
/// (`T 1 WHOLE`, `T 1 BIT` for both), recorded under ROADMAP "Oracle splits".
#[test]
fn a_select_waiter_and_a_whole_net_waiter_resume_in_declaration_order() {
    let n = "logic [7:0] n = 8'h00;";
    let whole = "always @(n) $display(\"T %0t WHOLE n=%h\", $time, n);";
    let bit = "always @(n[0]) $display(\"T %0t BIT n=%h\", $time, n);";
    check(
        &with_stim(n, &format!("{whole}\n  {bit}")),
        &[
            "T 1 WHOLE n=01",
            "T 1 BIT n=01",
            "T 2 WHOLE n=03",
            "T 4 WHOLE n=02",
            "T 4 BIT n=02",
            "T 5 WHOLE n=f2",
            "T 6 WHOLE n=0d",
            "T 6 BIT n=0d",
        ],
    );
    check(
        &with_stim(n, &format!("{bit}\n  {whole}")),
        &[
            "T 1 BIT n=01",
            "T 1 WHOLE n=01",
            "T 2 WHOLE n=03",
            "T 4 BIT n=02",
            "T 4 WHOLE n=02",
            "T 5 WHOLE n=f2",
            "T 6 BIT n=0d",
            "T 6 WHOLE n=0d",
        ],
    );
}

/// Still refused: a variable index (the derived net would move off `z` at the time-0 settle
/// and wake the process where §9.4.2 and iverilog do not), a subroutine's automatic local,
/// and a queue element. A select of a CONSTANT whose index the header lane cannot prove
/// constant stays refused too (`const_level_event_order.rs`).
#[test]
fn the_shapes_that_stay_loud() {
    refused(
        "module t;\n  logic [7:0] n = 8'h00; int i = 0;\n  always @(n[i]) $display(\"T %0t W\", $time);\n  \
         initial #2 $finish;\nendmodule\n",
        "variable index",
    );
    refused(
        "module t;\n  task automatic tw(); logic [7:0] loc; loc = 0; @(loc[0]); endtask\n  \
         initial tw();\nendmodule\n",
        "automatic local",
    );
    refused(
        "module t;\n  int q[$];\n  initial q = {0, 0};\n  always @(q[0]) $display(\"T %0t Q\", $time);\n  \
         initial #2 $finish;\nendmodule\n",
        "dynamic-storage handle",
    );
}
