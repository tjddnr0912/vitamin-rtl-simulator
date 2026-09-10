//! IEEE 1800-2017 §27.3: `function` / `task` declared INSIDE a generate block.
//!
//! Every value here is a measured oracle cell. iverilog 13 and verilator 5.052
//! agree on all of them but ONE — the `%m` of a subroutine declared in a
//! generate-if block: iverilog prints `t.u.g.show` (the block scope once),
//! verilator prints `t.u.g.g.show` (the label twice). vita pins iverilog's
//! spelling: it is the scope path IEEE §21.2.1 describes, and it is the same
//! `display_of` singleton-`[0]` strip every other generate scope already gets.
//!
//! The loud rows are the teeth. A generate-scoped routine is NOT visible from the
//! enclosing module by its bare name (iverilog: "Enable of unknown task"), a
//! hierarchical `u.g.f(x)` through the block is a capability gap vita keeps loud
//! rather than misroute (iverilog runs it), and a generate-scope `localparam W =
//! f(N)` stays loud because the elaborate-time const-function interpreter reads a
//! separate module-body-only table. All three are honest-loud, never silent-wrong.

use std::process::Command;

/// Run one design through one-shot `vita`; returns (exit code, stdout+stderr).
fn run(name: &str, body: &str, extra: &[&str]) -> (Option<i32>, String) {
    let dir = std::env::temp_dir().join(format!("vita_gensub_{}_{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("d.sv");
    std::fs::write(&sv, body).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .current_dir(&dir)
        .args(extra)
        .arg(&sv)
        .arg("-o")
        .arg(dir.join("d.vcd"))
        .output()
        .expect("run vita");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), text)
}

/// The design ran clean and printed every one of `wants`, in order.
fn expect_lines(name: &str, body: &str, wants: &[&str]) {
    let (rc, text) = run(name, body, &[]);
    assert_eq!(rc, Some(0), "expected a clean run, got:\n{text}");
    let mut from = 0usize;
    for w in wants {
        match text[from..].find(w) {
            Some(i) => from += i + w.len(),
            None => panic!("missing `{w}` (or out of order) in:\n{text}"),
        }
    }
}

/// The design was rejected with `code`, and the diagnostic names `needle`.
fn expect_loud(name: &str, body: &str, code: &str, needle: &str) {
    let (rc, text) = run(name, body, &[]);
    assert_eq!(rc, Some(1), "expected an elaborate error, got:\n{text}");
    assert!(
        text.contains(code) && text.contains(needle),
        "wanted {code} naming `{needle}`, got:\n{text}"
    );
}

// ── value rows ───────────────────────────────────────────────────────────────

/// The base case: a function declared in the TAKEN branch of a generate-if,
/// called from a cont-assign in the same block. iverilog/verilator: `f0`.
#[test]
fn gen_if_function_is_callable_in_its_block() {
    expect_lines(
        "gen_if",
        r#"
module m #(parameter int K = 1) (input logic [7:0] i_a, output logic [7:0] o_z);
  generate
    if (K != 0) begin : g
      function automatic logic [7:0] f(input logic [7:0] v);
        f = ~v;
      endfunction
      assign o_z = f(i_a);
    end else begin : g0
      assign o_z = i_a;
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; m u(.i_a(a),.o_z(z));
  initial begin #1 $display("%h", z); $finish; end endmodule
"#,
        &["f0"],
    );
}

/// The whole shape at once (r10b): a generate `f` SHADOWS the module's own `f`;
/// the untaken branch's same-named `f` is a different routine reached only by the
/// `K=0` instance; a generate-for body declares one `h` per iteration and each
/// reads ITS OWN genvar; and a generate-scoped task's `%m` names the block.
///
/// iverilog: `g.show f0 t.u.g.show` / `f0 0f 10 10`.
/// verilator: same values, `t.u.g.g.show` for the `%m` (the pinned split).
#[test]
fn shadowing_branches_iterations_and_percent_m() {
    expect_lines(
        "shape",
        r#"
module m #(parameter int K = 1, parameter int N = 2)
         (input logic [7:0] i_a, output logic [7:0] o_z, output logic [7:0] o_y [N]);
  function automatic logic [7:0] f(input logic [7:0] v); f = v; endfunction
  generate
    if (K != 0) begin : g
      function automatic logic [7:0] f(input logic [7:0] v); f = ~v; endfunction
      task automatic show(input logic [7:0] v); $display("g.show %h %m", v); endtask
      assign o_z = f(i_a);
      initial show(f(8'h0f));
    end else begin : g0
      function automatic logic [7:0] f(input logic [7:0] v); f = v + 1; endfunction
      assign o_z = f(i_a);
    end
    for (genvar i = 0; i < N; i++) begin : gl
      function automatic logic [7:0] h(input logic [7:0] v); h = v + 8'(i); endfunction
      assign o_y[i] = h(i_a);
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; logic [7:0] y[2];
  m u(.i_a(a),.o_z(z),.o_y(y)); m #(.K(0)) u0(.i_a(a),.o_z(),.o_y());
  initial begin #1 $display("%h %h %h %h", z, y[0], y[1], u0.o_z); $finish; end endmodule
"#,
        // `g.show f0 t.u.g.show` — the module `f` would have printed `0f`, so `f0`
        // is the shadow taking effect; `t.u.g.show` is the %m of the DECLARING
        // block, not of the calling process.
        &["g.show f0 t.u.g.show", "f0 0f 10 10"],
    );
}

/// A generate-scoped body reads the generate scope's OWN localparam and net.
/// 0x0f + 0x20 + 0x03 = 0x32 (both oracles).
#[test]
fn body_reads_its_generate_scope_localparam_and_net() {
    expect_lines(
        "scope_reads",
        r#"
module m(input logic [7:0] i_a, output logic [7:0] o_z);
  generate
    if (1) begin : g
      localparam logic [7:0] BIAS = 8'h20;
      logic [7:0] gnet;
      assign gnet = 8'h03;
      function automatic logic [7:0] f(input logic [7:0] v); f = v + BIAS + gnet; endfunction
      assign o_z = f(i_a);
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; m u(.i_a(a),.o_z(z));
  initial begin #1 $display("%h", z); $finish; end endmodule
"#,
        &["32"],
    );
}

/// The outward half of the same walk: a generate-scoped body still reaches the
/// enclosing MODULE's function and net. 0x0f + 0x10 + 0x02 = 0x21 (both oracles).
#[test]
fn body_still_reaches_the_enclosing_module() {
    expect_lines(
        "outward",
        r#"
module m(input logic [7:0] i_a, output logic [7:0] o_z);
  logic [7:0] mnet; assign mnet = 8'h02;
  function automatic logic [7:0] mfn(input logic [7:0] v); mfn = v + 8'h10; endfunction
  generate
    if (1) begin : g
      function automatic logic [7:0] f(input logic [7:0] v); f = mfn(v) + mnet; endfunction
      assign o_z = f(i_a);
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; m u(.i_a(a),.o_z(z));
  initial begin #1 $display("%h", z); $finish; end endmodule
"#,
        &["21"],
    );
}

/// A NAMED block nested inside the generate block calls the block's function.
/// The `$blk$`/named-block scope is transparent to the routine-key walk, so the
/// call resolves; `%m` is the named block's own path. Both oracles: `nb f0 m.g.nb`.
#[test]
fn nested_named_block_calls_the_generate_function() {
    expect_lines(
        "nested_block",
        r#"
module m;
  generate
    if (1) begin : g
      function automatic logic [7:0] f(input logic [7:0] v); f = ~v; endfunction
      initial begin : nb
        logic [7:0] r;
        r = f(8'h0f);
        $display("nb %h %m", r);
      end
    end
  endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        &["nb f0 m.g.nb"],
    );
}

// ── loud rows ────────────────────────────────────────────────────────────────

/// NOT visible from the enclosing module by its bare name — the generate block is
/// a scope, so the name does not leak outward. iverilog 13 agrees, verbatim:
/// `p_a.sv:7: error: Enable of unknown task ``tk''.`
#[test]
fn generate_scoped_task_is_invisible_from_the_module() {
    expect_loud(
        "invisible",
        r#"
module m;
  generate
    if (1) begin : g
      task automatic tk(input logic [7:0] v); $display("tk %h", v); endtask
    end
  endgenerate
  initial begin tk(8'h5); $finish; end
endmodule
"#,
        "VITA-E3010",
        "call to undeclared task `tk`",
    );
}

/// A HIERARCHICAL call through the generate block stays LOUD. iverilog runs it
/// (`u.g.f(8'h1)` = `fe`), so this is a capability gap and not an oracle
/// agreement: the generate-scoped key `g[0]$f` is not a name `hier_resolve` can
/// reconstruct from the call's `u`/`g`/`f` segments, so no `hier_funcs` entry is
/// registered at all — no entry rather than a wrong one.
#[test]
fn hierarchical_call_through_a_generate_block_stays_loud() {
    expect_loud(
        "hier",
        r#"
module m(input logic [7:0] i_a, output logic [7:0] o_z);
  generate
    if (1) begin : g
      function automatic logic [7:0] f(input logic [7:0] v); f = ~v; endfunction
      assign o_z = f(i_a);
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; m u(.i_a(a),.o_z(z));
  initial begin #1 $display("%h %h", z, u.g.f(8'h1)); $finish; end endmodule
"#,
        "VITA-E3009",
        "unsupported hierarchical function call `u.g.f`",
    );
}

/// A generate-scope `localparam W = f(N)` stays LOUD. iverilog folds it (`04`).
/// The elaborate-time const-function interpreter reads `const_func_table`, which
/// is collected from the module BODY only; wiring the generate half is a
/// follow-on, and half-wiring it would fold some calls and silently zero others.
#[test]
fn generate_scope_localparam_calling_a_generate_function_stays_loud() {
    expect_loud(
        "constfn",
        r#"
module m(output logic [7:0] o_z);
  generate
    if (1) begin : g
      function automatic int f(input int v); f = v + 1; endfunction
      localparam int W = f(3);
      assign o_z = W[7:0];
    end
  endgenerate
endmodule
module t; logic [7:0] z; m u(.o_z(z)); initial begin #1 $display("%h", z); $finish; end endmodule
"#,
        "VITA-E3009",
        "generate-scope parameter `W` value is not a constant",
    );
}

/// `defparam` inside a generate block is still deferred — the arm this slice
/// removed the func/task halves from keeps its third variant, with the message
/// narrowed to what it now actually covers.
#[test]
fn defparam_inside_generate_is_still_deferred() {
    expect_loud(
        "defparam",
        r#"
module c #(parameter int P = 1) (output logic [7:0] o); assign o = P[7:0]; endmodule
module m(output logic [7:0] o_z);
  generate
    if (1) begin : g
      c u1(.o(o_z));
      defparam u1.P = 2;
    end
  endgenerate
endmodule
module t; logic [7:0] z; m u(.o_z(z)); initial begin #1 $display("%h", z); $finish; end endmodule
"#,
        "VITA-E3009",
        "a `defparam` inside a generate block is deferred",
    );
}

// ── OBS rail ─────────────────────────────────────────────────────────────────

/// The static `subroutines` census in `run.json` gets one row per generate-scoped
/// routine, keyed by its table key — `g[0]$f`, `gl[0]$h`, `gl[1]$h`, and the
/// UNTAKEN branch's `g0[0]$f` (registered by the `K=0` instance). The key carries
/// `[`, `]` and `$`; no consumer on the rail rejects them.
#[test]
fn obs_subroutines_rows_carry_the_generate_scope_key() {
    let body = r#"
module m #(parameter int K = 1, parameter int N = 2)
         (input logic [7:0] i_a, output logic [7:0] o_z, output logic [7:0] o_y [N]);
  function automatic logic [7:0] f(input logic [7:0] v); f = v; endfunction
  generate
    if (K != 0) begin : g
      function automatic logic [7:0] f(input logic [7:0] v); f = ~v; endfunction
      task automatic show(input logic [7:0] v); $display("g.show %h %m", v); endtask
      assign o_z = f(i_a);
      initial show(f(8'h0f));
    end else begin : g0
      function automatic logic [7:0] f(input logic [7:0] v); f = v + 1; endfunction
      assign o_z = f(i_a);
    end
    for (genvar i = 0; i < N; i++) begin : gl
      function automatic logic [7:0] h(input logic [7:0] v); h = v + 8'(i); endfunction
      assign o_y[i] = h(i_a);
    end
  endgenerate
endmodule
module t; logic [7:0] a=8'h0f, z; logic [7:0] y[2];
  m u(.i_a(a),.o_z(z),.o_y(y)); m #(.K(0)) u0(.i_a(a),.o_z(),.o_y());
  initial begin #1 $finish; end endmodule
"#;
    let dir = std::env::temp_dir().join(format!("vita_gensub_obs_{}", std::process::id()));
    let obs = dir.join("obs");
    let (rc, text) = run(
        "obs",
        body,
        &["--obs-dir", obs.to_str().expect("utf-8 temp path")],
    );
    assert_eq!(rc, Some(0), "expected a clean run, got:\n{text}");
    let json = std::fs::read_to_string(obs.join("run.json")).expect("run.json");
    for want in [
        r#""name": "g[0]$f""#,
        r#""name": "g0[0]$f""#,
        r#""name": "g[0]$show""#,
        r#""name": "gl[0]$h""#,
        r#""name": "gl[1]$h""#,
    ] {
        assert!(json.contains(want), "missing {want} in run.json:\n{json}");
    }
    // The module's own `f` keeps its bare key and its own row — the generate `f`
    // shadowed it at every call site, so it is declared and never called.
    assert!(
        json.contains(r#""name": "f""#),
        "the module-scope `f` lost its row:\n{json}"
    );
}
