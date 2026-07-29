//! R16 §3.6 / §3.7 / §4: a discarding pop on a record queue, a dynamic-array formal in a
//! `return` or a concat, and the diagnostic-quality items that travelled with them.
//!
//! §3.6 A struct with a NAMED-PARAMETER-width member cannot have its member offsets
//!      computed in the parser, so such a variable is stored per field (`$unp$q$f`)
//!      rather than packed into one vector. The record-queue fan-out covered
//!      `push_back`/`push_front`/`insert`/`delete` and the ASSIGNING pop
//!      (`rec = q.pop_front()`), but not the DISCARDING one. `q.pop_front();` and
//!      `void'(q.pop_front());` therefore fell through to the generic 2-segment enable,
//!      found no net named `q`, and surfaced as "unsupported hierarchical task call
//!      `q.pop_front`" — a message about instance paths for something that is neither.
//!      Only the product (param-width struct × discarded result) failed, so the same
//!      source with a literal width worked and the shape looked far narrower than it was.
//!
//! §3.7 A dynamic-array formal's caller array is snapshotted into the callee's formal
//!      slot by a marker emitted just before the expression. Only a direct blocking-assign
//!      rhs emitted one, so `return f(arr);` was loud. A `return` IS that assignment, so
//!      it takes the same route; a call BURIED in a concat takes it too, either by
//!      hoisting to a temp (outside a frame body) or by emitting the markers in place
//!      (inside one, where a temp cannot live).
//!
//! §4   The hierarchical-task-call reject was the only diagnostic in the report with no
//!      `file:line:col`; it fires in a resolve pass that runs long after lowering, so the
//!      enable's span is now carried on the deferred record.
//!
//! ORACLE. iverilog rejects unpacked structs outright ("sorry"), so §3.6 is pinned
//! hand-IEEE (§7.10.2: `pop_front` removes and returns the first element; the fields must
//! stay in step). §3.7's cases all run under iverilog and are pinned against it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sqpd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

fn runs(src: &str, want: &[&str]) {
    let (o, ok) = run(src);
    assert!(ok, "expected acceptance, got:\n{o}");
    let got: Vec<&str> = o.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(got, want, "output mismatch:\n{o}");
}

// ---------------------------------------------------------------------------
// §3.6 — a discarding pop on a SoA record queue
// ---------------------------------------------------------------------------

/// The report's reproducer.
#[test]
fn void_cast_pop_on_a_param_width_struct_queue() {
    runs(
        r#"module t;
             localparam int W = 8;
             typedef struct { logic [W-1:0] addr; } pkt_t;
             pkt_t q [$];
             pkt_t p;
             initial begin
               p.addr = 8'hA5;
               q.push_back(p);
               void'(q.pop_front());
               $display("R %0d", q.size());
             end
           endmodule"#,
        &["R 0"],
    );
}

/// The bare statement form fails and succeeds for the same reason — the report framed
/// this as a `void'()` problem, but the discarded result is what matters.
#[test]
fn bare_statement_pop_on_a_param_width_struct_queue() {
    runs(
        r#"module t;
             localparam int W = 8;
             typedef struct { logic [W-1:0] addr; } pkt_t;
             pkt_t q [$];
             pkt_t p;
             initial begin
               p.addr = 8'hA5;
               q.push_back(p);
               q.pop_back();
               $display("R %0d", q.size());
             end
           endmodule"#,
        &["R 0"],
    );
}

/// MULTI-FIELD: every field queue must pop exactly once, or the fields desync and a
/// later assigning pop returns one record's `addr` beside another's `len`. Popping the
/// head twice (once discarded, once assigned) is what proves they stayed in step.
#[test]
fn discarding_pop_keeps_the_field_queues_in_step() {
    runs(
        r#"module t;
             localparam int W = 8;
             typedef struct { logic [W-1:0] addr; logic [W-1:0] len; } pkt_t;
             pkt_t q [$];
             pkt_t p, r;
             initial begin
               p.addr = 8'hA5; p.len = 8'h03; q.push_back(p);
               p.addr = 8'hB6; p.len = 8'h04; q.push_back(p);
               p.addr = 8'hC7; p.len = 8'h05; q.push_back(p);
               void'(q.pop_front());
               r = q.pop_front();
               $display("R %0h %0h %0d", r.addr, r.len, q.size());
             end
           endmodule"#,
        &["R b6 4 1"],
    );
}

/// The report's PASS boundary: a LITERAL-width member packs into one vector and never
/// took the per-field path at all.
#[test]
fn literal_width_struct_queue_still_works() {
    runs(
        r#"module t;
             typedef struct { logic [7:0] addr; } pkt_t;
             pkt_t q [$];
             pkt_t p;
             initial begin
               p.addr = 8'hA5;
               q.push_back(p);
               void'(q.pop_front());
               $display("R %0d", q.size());
             end
           endmodule"#,
        &["R 0"],
    );
}

/// SOUNDNESS PIN. `pop_*` takes no arguments; the fan-out must not silently drop one.
#[test]
fn pop_with_an_argument_stays_loud() {
    let (o, ok) = run(r#"module t;
             localparam int W = 8;
             typedef struct { logic [W-1:0] addr; } pkt_t;
             pkt_t q [$];
             pkt_t p;
             initial begin p.addr = 1; q.push_back(p); void'(q.pop_front(2)); end
           endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(
        o.contains("take no arguments"),
        "expected the arity reject, got:\n{o}"
    );
}

// ---------------------------------------------------------------------------
// §3.7 — a dynamic-array formal in a return / concat
// ---------------------------------------------------------------------------

const DYN_FNS: &str = "module t;\n\
     function automatic string h (input byte b []); return $sformatf(\"%02x\", b[0]); endfunction\n\
     function automatic string g (input byte b []); return $sformatf(\"<%02x>\", b[1]); endfunction\n";

fn dyn_case(extra: &str, body: &str, want: &[&str]) {
    runs(
        &format!(
            "{DYN_FNS}{extra}\
             byte arr []; string s;\n\
             initial begin arr = new[2]; arr[0] = 8'hab; arr[1] = 8'hcd;\n{body}end\n\
             endmodule"
        ),
        want,
    );
}

/// The report's reproducer. iverilog prints `R ab`.
#[test]
fn return_of_a_dyn_formal_call() {
    dyn_case(
        "function automatic string ht (input byte b []); return h(b); endfunction\n",
        "  $display(\"R %s\", ht(arr));\n",
        &["R ab"],
    );
}

/// A concat operand at module-process level. iverilog prints `R ab!`.
#[test]
fn concat_operand_at_process_level() {
    dyn_case(
        "",
        "  s = {h(arr), \"!\"}; $display(\"R %s\", s);\n",
        &["R ab!"],
    );
}

/// TWO calls to the same function in one concat, outside a frame body — each is hoisted
/// to its own temp, so they do not share a formal slot. iverilog prints `R abab`.
#[test]
fn two_same_target_calls_in_a_concat_outside_a_frame() {
    dyn_case(
        "",
        "  s = {h(arr), h(arr)}; $display(\"R %s\", s);\n",
        &["R abab"],
    );
}

/// A concat INSIDE a frame body, where the temp hoist cannot go — the markers are
/// emitted in place instead. iverilog prints `R ab!`.
#[test]
fn concat_operand_inside_a_frame_body() {
    dyn_case(
        "function automatic string hc (input byte b []); return {h(b), \"!\"}; endfunction\n",
        "  $display(\"R %s\", hc(arr));\n",
        &["R ab!"],
    );
}

/// Two DISTINCT callees in one frame-body concat — distinct formal slots, so the
/// in-place markers cannot interfere. iverilog prints `R ab<cd>`.
#[test]
fn two_distinct_callees_in_a_frame_body_concat() {
    dyn_case(
        "function automatic string hd (input byte b []); return {h(b), g(b)}; endfunction\n",
        "  $display(\"R %s\", hd(arr));\n",
        &["R ab<cd>"],
    );
}

/// SOUNDNESS PIN. Two calls to the SAME function in one frame-body expression would both
/// read the last snapshot, and there is nowhere inside a frame to put a temp — so this
/// stays loud rather than pick a value.
#[test]
fn two_same_target_calls_inside_a_frame_body_stay_loud() {
    let (o, ok) = run(&format!(
        "{DYN_FNS}\
         function automatic string he (input byte b []); return {{h(b), h(b)}}; endfunction\n\
         byte arr []; initial begin arr = new[2]; arr[0] = 8'hab; $display(\"R %s\", he(arr)); end\n\
         endmodule"
    ));
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
}

/// SOUNDNESS PIN. A RECURSIVE call inside the function's own body would have its formals
/// overwritten by the marker before the rest of the expression reads them. It gives the
/// right answer when every level passes the same array and a wrong one otherwise, so it
/// is refused outright.
#[test]
fn self_recursive_dyn_formal_call_stays_loud() {
    let (o, ok) = run(r#"module t;
             function automatic int f (input int c [], input int n);
               int d [];
               if (n <= 0) return 0;
               d = new[1]; d[0] = 100;
               return c[0] + f(d, n-1);
             endfunction
             int a []; int r;
             initial begin a = new[1]; a[0] = 3; r = f(a, 2); $display("R %0d", r); end
           endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
}

/// R16 §4-3: the reject must no longer claim the restriction is "at module-process
/// level" — r17 lifted that, and a reader who believed it moved the call for nothing.
#[test]
fn the_dyn_formal_reject_no_longer_claims_module_process_level() {
    let (o, ok) = run(r#"module t;
             function automatic int f (input int c [], input int n);
               int d [];
               if (n <= 0) return 0;
               d = new[1]; d[0] = 100;
               return c[0] + f(d, n-1);
             endfunction
             int a []; int r;
             initial begin a = new[1]; a[0] = 3; r = f(a, 2); $display("R %0d", r); end
           endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(
        !o.contains("at module-process level"),
        "stale restriction still claimed:\n{o}"
    );
    assert!(
        o.contains("RECURSIVE call"),
        "the real reason should be named:\n{o}"
    );
}

// ---------------------------------------------------------------------------
// §4-1 — the located diagnostic
// ---------------------------------------------------------------------------

/// The hierarchical-task-call reject fires in a resolve pass that runs long after the
/// enable was lowered. It was the ONLY diagnostic in the round-16 report without a
/// position, and in one testbench it was the only diagnostic at all — so that log had no
/// position anywhere in it.
#[test]
fn the_hierarchical_task_call_reject_carries_a_location() {
    let (o, ok) = run(
        "module sub;\n  task nope(input string s); $display(\"%s\", s); endtask\nendmodule\n\
         module t;\n  sub u();\n  initial begin\n    u.nope(\"x\");\n  end\nendmodule\n",
    );
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    let line = o
        .lines()
        .find(|l| l.contains("unsupported hierarchical task call"))
        .unwrap_or_else(|| panic!("expected the hierarchical-task reject:\n{o}"));
    assert!(
        line.contains("t.sv:7:5: "),
        "expected a file:line:col anchored at the enable, got:\n{line}"
    );
}
