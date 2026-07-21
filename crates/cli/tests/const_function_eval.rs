//! §4.5.186 (loud -> supported): a CONSTANT-FUNCTION call in a constant context
//! (`localparam W = clog2(N)`, `logic [f(N)-1:0] bus`). vita previously rejected any
//! function call in a const context with E3009 "not a foldable constant expression";
//! iverilog evaluates it. The elaborator now interprets an integer const-function body
//! at compile time: `const_eval_in_scope`'s new Call arm binds the input formals to
//! their (folded) arg values and runs the body over a local env (blocking `=`,
//! if/else, for/while/repeat, return; recursion and nested calls too), returning the
//! value coerced to the declared return width. Elaborate-only — no AST/IR/format
//! change.
//!
//! correct-or-loud (a wrong param value poisons every downstream width silently, so
//! this is airtight): the i64 integer domain only. A real/string return or
//! formal/local, an output/inout/unpacked-array formal, a reference to a runtime
//! signal, a system task / non-blocking / timing / case / unmodeled statement, a
//! non-terminating loop (a step cap), or recursion past a depth cap all return
//! None -> LOUD, never a silently-wrong constant.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first `K=` line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cfe_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let key = text
        .lines()
        .find(|l| l.starts_with("K="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (key, out.status.success())
}

fn loud(src: &str) -> bool {
    !run(src).1
}

// ── supported: constant-function evaluation ─────────────────────────────────

#[test]
fn clog_while_loop_in_width() {
    // The motivating case: a clog2-style function with a while loop sizing a vector.
    let (k, ok) = run("module top;\n\
         function automatic int clog(int n); int r=0; while (n>1) begin n=n/2; r++; end return r; endfunction\n\
         localparam W = clog(256);\n\
         logic [W-1:0] r;\n\
         initial begin r = 8'hAB; $display(\"K=%0d %0d %h\", W, $bits(r), r); $finish; end endmodule");
    assert!(ok && k == "K=8 8 ab", "got ({k}, {ok})");
}

#[test]
fn return_expression() {
    let (k, ok) = run("module top;\n\
         function automatic int dbl(int x); return x*2; endfunction\n\
         parameter P = dbl(21);\n\
         initial begin $display(\"K=%0d\", P); $finish; end endmodule");
    assert!(ok && k == "K=42", "got ({k}, {ok})");
}

#[test]
fn for_loop_sum_of_squares() {
    let (k, ok) = run("module top;\n\
         function automatic int sumsq(int n);\n\
           int s = 0;\n\
           for (int i=1; i<=n; i++) s = s + i*i;\n\
           return s;\n\
         endfunction\n\
         localparam S = sumsq(5);\n\
         initial begin $display(\"K=%0d\", S); $finish; end endmodule");
    assert!(ok && k == "K=55", "got ({k}, {ok})");
}

#[test]
fn recursion_factorial() {
    let (k, ok) = run("module top;\n\
         function automatic int fact(int n); if (n<=1) return 1; return n*fact(n-1); endfunction\n\
         localparam F = fact(5);\n\
         initial begin $display(\"K=%0d\", F); $finish; end endmodule");
    assert!(ok && k == "K=120", "got ({k}, {ok})");
}

#[test]
fn function_name_return() {
    // No explicit `return`; the value is the function-name variable.
    let (k, ok) = run("module top;\n\
         function automatic int addone(int x); addone = x + 1; endfunction\n\
         localparam A = addone(41);\n\
         initial begin $display(\"K=%0d\", A); $finish; end endmodule");
    assert!(ok && k == "K=42", "got ({k}, {ok})");
}

#[test]
fn multi_arg_and_nested_call() {
    let (k, ok) = run("module top;\n\
         function automatic int mx(int a, int b); return (a>b)?a:b; endfunction\n\
         function automatic int mx3(int a, int b, int c); return mx(mx(a,b),c); endfunction\n\
         localparam M = mx3(3, 9, 5);\n\
         initial begin $display(\"K=%0d\", M); $finish; end endmodule");
    assert!(ok && k == "K=9", "got ({k}, {ok})");
}

#[test]
fn narrow_return_type_coercion() {
    // A `byte` return truncates to 8 bits: 300 -> 44 (`300 & 0xFF`).
    let (k, ok) = run("module top;\n\
         function automatic byte trunc(int x); return x; endfunction\n\
         localparam int T = trunc(300);\n\
         initial begin $display(\"K=%0d\", T); $finish; end endmodule");
    assert!(ok && k == "K=44", "got ({k}, {ok})");
}

#[test]
fn param_arg_and_chained() {
    let (k, ok) = run("module top #(parameter DW = 100);\n\
         function automatic int clog(int n); int r=0; while ((1<<r)<n) r++; return r; endfunction\n\
         localparam AW = clog(DW);\n\
         localparam AW2 = AW + 1;\n\
         initial begin $display(\"K=%0d %0d\", AW, AW2); $finish; end endmodule");
    assert!(ok && k == "K=7 8", "got ({k}, {ok})");
}

#[test]
fn same_function_param_and_runtime() {
    // Regression: a function used in BOTH a const param AND a runtime call still runs
    // its runtime path (inline/frame-call) unchanged.
    let (k, ok) = run("module top;\n\
         function automatic int dbl(int x); return x*2; endfunction\n\
         localparam P = dbl(10);\n\
         int rv;\n\
         initial begin rv = dbl(21); $display(\"K=%0d %0d\", P, rv); $finish; end endmodule");
    assert!(ok && k == "K=20 42", "got ({k}, {ok})");
}

#[test]
fn negative_arg_abs() {
    let (k, ok) = run("module top;\n\
         function automatic int absv(int x); return (x<0)?-x:x; endfunction\n\
         localparam A = absv(-42);\n\
         initial begin $display(\"K=%0d\", A); $finish; end endmodule");
    assert!(ok && k == "K=42", "got ({k}, {ok})");
}

// ── correct-or-loud boundaries: must STAY loud (never a wrong constant) ──────

#[test]
fn real_function_stays_loud() {
    assert!(loud(
        "module top;\n\
         function automatic real half(int x); return x/2.0; endfunction\n\
         localparam real R = half(7);\n\
         initial begin $display(\"K=%f\", R); $finish; end endmodule"
    ));
}

#[test]
fn nonterminating_loop_stays_loud() {
    // A loop that never decrements trips the step cap -> loud (never hangs elaboration).
    assert!(loud(
        "module top;\n\
         function automatic int bad(int n); int r=0; while (n>0) r++; return r; endfunction\n\
         localparam B = bad(5);\n\
         initial begin $display(\"K=%0d\", B); $finish; end endmodule"
    ));
}

#[test]
fn system_task_in_body_stays_loud() {
    assert!(loud(
        "module top;\n\
         function automatic int noisy(int x); $display(\"side\"); return x; endfunction\n\
         localparam N = noisy(5);\n\
         initial begin $display(\"K=%0d\", N); $finish; end endmodule"
    ));
}

#[test]
fn array_formal_stays_loud() {
    assert!(loud(
        "module top;\n\
         function automatic int suma(input int a[4]); int s=0; foreach(a[i]) s+=a[i]; return s; endfunction\n\
         localparam S = suma('{1,2,3,4});\n\
         initial begin $display(\"K=%0d\", S); $finish; end endmodule"
    ));
}

#[test]
fn runtime_signal_reference_stays_loud() {
    // A const function that reads a non-constant module signal cannot fold -> loud.
    assert!(loud(
        "module top;\n\
         logic [3:0] rt;\n\
         function automatic int usesig(int x); return x + rt; endfunction\n\
         localparam U = usesig(5);\n\
         initial begin $display(\"K=%0d\", U); $finish; end endmodule"
    ));
}
