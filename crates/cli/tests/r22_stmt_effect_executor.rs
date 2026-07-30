//! Round-22: which executor a subroutine body runs on must not depend on an
//! UNRELATED statement in that body.
//!
//! The report's root, narrowed to one line: `compute_suspendable_tasks` asked only
//! "does this statement write outside the frame?" of a blocking assign, so
//! `rc = $fgets(line, fd);` — whose effect is in the RHS, not the destination —
//! looked like a plain in-frame write and left the task on the synchronous `&self`
//! frame executor, which reaches `$fgets` through the pure `eval` path that returns
//! 0 and touches nothing. Adding an unrelated `$display("x")` to the same body
//! flipped it: `Stmt::SysTask` fell to the `_ => true` arm, marked the task
//! suspendable, and moved the whole body to the `&mut` executor that performs the
//! effect. Every test here therefore pins the SAME body with and without that line.
//!
//! Oracle: iverilog 13 runs all of these (the tasks/functions here use only shapes it
//! accepts), and every expected value below was measured against it — a 15-effect ×
//! 6-subroutine-shape matrix. `$dist_uniform`, `$cast` and the assoc/queue steps have
//! no iverilog lane in some shapes; those pins are hand-IEEE (§13.3, §20.x, §21.3).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run a source string, returning combined stdout+stderr and whether exit was 0.
/// `extra` carries plusargs. Each test gets its own temp DIR so the data files the
/// sources create cannot collide when the suite runs in parallel.
fn run_with(src: &str, extra: &[&str]) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_r22_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.sv");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .args(extra)
        .arg("--timeout")
        .arg("400")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    (s, out.status.success())
}

fn run(src: &str) -> (String, bool) {
    run_with(src, &[])
}

/// Build the report's §3.1 shape: a subroutine whose body does `body`, optionally with
/// the unrelated `$display("x")` that used to be the difference between working and not.
fn fgets_task(disp: bool, lifetime: &str) -> String {
    let d = if disp { r#"$display("x");"# } else { "" };
    format!(
        r#"`timescale 1ns/1ps
module t;
  task {lifetime} rd (input int fd, output int rc);
    string line;
    {d}
    rc = $fgets(line, fd);
  endtask
  initial begin
    int wfd, fd, rc;
    wfd = $fopen("d.txt", "w"); $fdisplay(wfd, "24 abc"); $fclose(wfd);
    fd = $fopen("d.txt", "r");
    rd(fd, rc);
    $fclose(fd);
    $display("rc=%0d", rc);
    $finish;
  end
endmodule"#
    )
}

// ── §3.1 `$fgets` — the `.rsp` walker shape ──────────────────────────────────

/// The report's minimal repro. `rc=7` is the byte count of "24 abc\n" (iverilog).
#[test]
fn fgets_in_an_automatic_task_with_formals_reads_the_line() {
    let (o, ok) = run(&fgets_task(false, "automatic"));
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("rc=7"), "expected rc=7:\n{o}");
}

/// The boundary the report isolated: the SAME body plus one unrelated `$display`.
/// Both forms must now agree — that they ever disagreed was the bug.
#[test]
fn an_unrelated_display_does_not_change_the_fgets_result() {
    let (a, ok_a) = run(&fgets_task(false, "automatic"));
    let (b, ok_b) = run(&fgets_task(true, "automatic"));
    assert!(ok_a && ok_b, "expected exit 0:\n{a}\n---\n{b}");
    assert!(a.contains("rc=7"), "no-$display form:\n{a}");
    assert!(b.contains("rc=7"), "$display form:\n{b}");
}

/// Dropping the lifetime keyword was the report's worst cell: no diagnostic at all,
/// `rc=0`, exit 0. A static task's `string` body-local was registered as a plain Wire
/// (the inline-task collector used `map_net_kind_or_wire`, which has no String arm),
/// and `$fgets` writing that Wire failed silently — the E3018 procedural-assign check
/// that catches a plain `s = "x"` never sees a system function's destination write.
#[test]
fn fgets_in_a_static_task_with_a_string_local_reads_the_line() {
    let (o, ok) = run(&fgets_task(false, ""));
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("rc=7"), "expected rc=7:\n{o}");
}

/// The same `string` body-local, written plainly. This was a loud E3018 —
/// "procedural assignment to net `t.$itask$rd$L.s`" — for a variable the user
/// declared as a `string`, in the static task only.
#[test]
fn a_string_local_in_a_static_task_is_procedurally_assignable() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  task w (output int ob);
    string s;
    s = $sformatf("v=%0d", 5);
    ob = (s == "v=5");
  endtask
  initial begin int ob; w(ob); $display("ob=%0d", ob); $finish; end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("ob=1"), "expected ob=1:\n{o}");
}

// ── §3.2 `$value$plusargs` — the exit-0 silent case ──────────────────────────

/// `+LIM=7` must override the default. This produced NO diagnostic and exit 0, so a
/// testbench selecting behaviour by plusarg ran its default config and reported PASS.
#[test]
fn value_plusargs_in_an_automatic_task_overrides_the_default() {
    let (o, ok) = run_with(
        r#"`timescale 1ns/1ps
module t;
  task automatic cfg (output int lim);
    int rc;
    lim = 20;
    rc = $value$plusargs("LIM=%d", lim);
  endtask
  initial begin int lim; cfg(lim); $display("lim=%0d", lim); $finish; end
endmodule"#,
        &["+LIM=7"],
    );
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("lim=7"), "expected lim=7:\n{o}");
}

// ── §3.3 `$random(seed)` — the value is silently wrong ───────────────────────

/// Two seeded draws must differ and the seed must advance. Returned 0/0 with the seed
/// untouched, at exit 0. `$urandom` was unaffected (no ref-arg writeback), which is
/// what made this one so easy to miss.
#[test]
fn seeded_random_in_an_automatic_task_advances_the_seed() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  task automatic act (output int differ, output int moved);
    int seed = 1; int a, b;
    a = $random(seed);
    b = $random(seed);
    differ = (a != b); moved = (seed != 1);
  endtask
  initial begin
    int differ, moved;
    act(differ, moved);
    $display("differ=%0d moved=%0d", differ, moved);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(
        o.contains("differ=1 moved=1"),
        "seed must advance between draws:\n{o}"
    );
}

// ── the function half of the same root ───────────────────────────────────────

/// A FUNCTION with output formals hit this identically. Functions were excluded from
/// the suspendable classifier on the grounds that IEEE forbids timing controls in a
/// function — but what the set actually grants is the `&mut` executor, and a framed
/// function is already proven leaf and non-suspending by `classify_frame_body`.
#[test]
fn fgets_in_an_automatic_function_with_output_formals_reads_the_line() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function automatic int rd (input int fd, output int n);
    string line;
    n = $fgets(line, fd);
    return 0;
  endfunction
  initial begin
    int wfd, fd, n, dummy;
    wfd = $fopen("d.txt", "w"); $fdisplay(wfd, "24 abc"); $fclose(wfd);
    fd = $fopen("d.txt", "r");
    dummy = rd(fd, n);
    $fclose(fd);
    $display("n=%0d", n);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("n=7"), "expected n=7:\n{o}");
}

/// An INPUT-ONLY function called from an expression is the subtler half: with no
/// output formal there is no copy-out call to route, so it lowered to an `Expr::Call`
/// that `eval` runs synchronously. Such functions now join `inout_func_names`, derived
/// from the suspendable set itself rather than from a second list of `$name` strings.
#[test]
fn a_statement_effect_in_an_input_only_function_is_routed() {
    let (o, ok) = run_with(
        r#"`timescale 1ns/1ps
module t;
  function automatic int cfg ();
    int lim = 20; int rc;
    rc = $value$plusargs("LIM=%d", lim);
    return lim;
  endfunction
  initial begin int r; r = cfg(); $display("r=%0d", r); $finish; end
endmodule"#,
        &["+LIM=7"],
    );
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("r=7"), "expected r=7:\n{o}");
}

/// The boundary on that routing, found by the adversarial review after it had already
/// broken `dyn_formal_wrapped_call`. `foreach` over a dyn-array formal desugars to
/// `b.first(i)` / `b.next(i)`, which ARE statement-level effects, so keying the
/// call-shape decision on the suspendable set rerouted this working function — and the
/// copy-out path cannot bind a dyn-array input formal, so it went loud. Over-marking is
/// free for the suspend classifier and is NOT free for anything that changes a call's
/// shape; the two questions need two predicates.
#[test]
fn a_foreach_over_a_dyn_formal_keeps_its_direct_call_path() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function automatic logic [63:0] packk (input byte b[]);
    packk = 0;
    foreach (b[i]) packk += b[i];
  endfunction
  initial begin
    byte v[]; logic [63:0] s; bit en = 1;
    v = new[3]; v[0] = 1; v[1] = 2; v[2] = 3;
    s = en ? packk(v) : 64'd0;
    $display("s=%0d", s);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(!o.contains("E3009"), "must not go loud:\n{o}");
    assert!(o.contains("s=6"), "expected s=6:\n{o}");
}

/// A BARE sys-read statement (return discarded) in a frame task body was warn+skip:
/// `W3056 … skipped`, exit 0, destination untouched and the fd not advanced, while
/// iverilog performs the read.
#[test]
fn a_bare_sys_read_statement_in_a_task_body_performs_the_read() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  string line;
  task automatic w (input int fd, output int ob);
    $fgets(line, fd);
    ob = (line == "24 abc\n");
  endtask
  initial begin
    int wfd, fd, ob;
    wfd = $fopen("d.txt", "w"); $fdisplay(wfd, "24 abc"); $fclose(wfd);
    fd = $fopen("d.txt", "r");
    w(fd, ob);
    $fclose(fd);
    $display("ob=%0d", ob);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("ob=1"), "expected the line to be read:\n{o}");
    assert!(
        !o.contains("W3056"),
        "the read must not be skipped with a warning:\n{o}"
    );
}

// ── a container op in a task: the classifier change fixed these too ──────────

/// A module-scope queue popped inside a frame task was `W4020 … (X; not popped)` with
/// the wrong value at exit 0 — the `&self` executor genuinely cannot pop, and nothing
/// routed the task away from it.
#[test]
fn a_queue_pop_inside_a_frame_task_pops() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  int q[$];
  task automatic w (output int oa, output int ob);
    oa = q.pop_front();
    ob = q.size();
  endtask
  initial begin
    int oa, ob;
    q.push_back(5); q.push_back(6);
    w(oa, ob);
    $display("oa=%0d ob=%0d", oa, ob);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("oa=5 ob=1"), "expected a real pop:\n{o}");
}

// ── the copy-out call inside a frame body: destination decides ───────────────

/// A `task automatic` body calling a `$fgets`-bearing function was the last shape left
/// loud after the classifier fix, while the identical call from a module process or an
/// inlined task worked. `hoist_stmt_top` stands down inside a frame body because its
/// rewrite needs a MODULE-net temp; when the rhs is the call and nothing else, the
/// copy-out can target the statement's own frame-local lvalue and no temp is needed.
#[test]
fn a_frame_task_body_can_call_a_statement_effect_function() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function automatic int rd (input int f);
    string line; int rc;
    rc = $fgets(line, f);
    return rc;
  endfunction
  task automatic outer (input int f, output int n);
    n = rd(f);
  endtask
  initial begin
    int wfd, fd, n;
    wfd = $fopen("d.txt", "w"); $fdisplay(wfd, "24 abc"); $fclose(wfd);
    fd = $fopen("d.txt", "r");
    outer(fd, n);
    $fclose(fd);
    $display("n=%0d", n);
    $finish;
  end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("n=7"), "expected n=7:\n{o}");
}

/// The gate on that path, and the reason it is a DESTINATION test rather than a
/// frame-body test. Routing the copy-out when the destination is a MODULE net aborts the
/// engine with `frame lvalue net is routed` (rc=101) — this is the condition the
/// round-19 note recorded as "NOT YET NAMED", found by measuring rather than guessing a
/// third guard. A module-net destination must stay loud; it must never panic.
#[test]
fn a_copy_out_to_a_module_net_from_a_frame_body_stays_loud() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  int g;
  function automatic int fn (input int a, output int o);
    o = a * 2; return a + 1;
  endfunction
  task automatic tk (output int r);
    int o;
    g = fn(5, o);
    r = g + o;
  endtask
  initial begin int r; tk(r); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(!ok, "a module-net destination must stay loud:\n{o}");
    assert!(o.contains("E3009"), "expected the elaborate reject:\n{o}");
    assert!(
        !o.contains("panicked") && !o.contains("frame lvalue net is routed"),
        "must not reach the engine assertion:\n{o}"
    );
}

/// A frame-LOCAL destination is the safe half of that same gate, including this
/// subroutine's own output formal.
#[test]
fn a_copy_out_to_a_frame_local_from_a_frame_body_works() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function automatic int fn (input int a, output int o);
    o = a * 2; return a + 1;
  endfunction
  task automatic tk (output int r);
    int o;
    r = fn(5, o);
    r = r + o;
  endtask
  initial begin int r; tk(r); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("r=16"), "expected 6 + 10:\n{o}");
}

/// The OTHER half of that gate, and the one the first version missed. A copy-out call has
/// a destination per `output`/`inout` formal as well as one for the return value, and the
/// engine asserts every write it performs is frame-local. Here `r` is frame-local and
/// passes, but the output actual `gv` is a MODULE net — and that is the write that trips
/// `frame_write_lvalue`'s assert (rc=101 in debug; a write into the wrong net with the
/// assert stripped). Checking only the value's destination checks the wrong half.
#[test]
fn an_output_actual_on_a_module_net_keeps_the_frame_body_loud() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nxt (input int a, output int o);
    o = a + 1; nxt = o;
  endfunction
  task automatic tk (output int r);
    r = nxt(5, gv);
  endtask
  initial begin int z; gv = 50; tk(z); $display("z=%0d gv=%0d", z, gv); $finish; end
endmodule"#);
    assert!(!ok, "a module-net output actual must stay loud:\n{o}");
    assert!(o.contains("E3009"), "expected the elaborate reject:\n{o}");
    assert!(
        !o.contains("panicked"),
        "must report, not panic — the engine assert is not a diagnostic:\n{o}"
    );
}

/// Placement guard for that same path: it calls `lower_lvalue`, and `lower_stmt`'s
/// §6.16.3 note is explicit that reaching `lower_lvalue` ahead of the `s[i]` detection
/// emits a silent packed BIT-write. So it sits BELOW the string-element and array
/// specials, and an `s[i] = f(…)` must stay loud rather than be quietly mis-lowered.
#[test]
fn a_string_element_destination_is_not_silently_bit_written() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function automatic int fn (input int a, output int o);
    o = a; return 65;
  endfunction
  task automatic tk (output int r);
    string s; int o;
    s = "zz";
    s[0] = fn(7, o);
    r = (s == "Az") ? o : -1;
  endtask
  initial begin int r; tk(r); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(!ok, "expected a loud reject:\n{o}");
    assert!(o.contains("E3009"), "expected the elaborate reject:\n{o}");
}

// ── §4 the fatal must actually stop the run ──────────────────────────────────

/// A class-method body is one of the few positions vita cannot route (virtual dispatch
/// goes through `Expr::Call`, which has no `Terminator::Call` for the router to see),
/// so it stays loud — and the loud must HALT. It used to latch the diagnostic and let
/// the process run on, so the body reached its own `$finish` and the run ended as a
/// clean Finish with the testbench free to print its own verdict afterwards.
#[test]
fn a_frame_fatal_stops_the_process_that_raised_it() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
class C;
  function int rd (input int f);
    string line; int rc;
    rc = $fgets(line, f);
    return rc;
  endfunction
endclass
module t;
  initial begin
    C c; int wfd, fd, r;
    wfd = $fopen("d.txt", "w"); $fdisplay(wfd, "24 abc"); $fclose(wfd);
    c = new(); fd = $fopen("d.txt", "r");
    r = c.rd(fd);
    $display("VERDICT-AFTER-FATAL r=%0d", r);
    $finish;
  end
endmodule"#);
    assert!(!ok, "a fatal must fail the run:\n{o}");
    assert!(o.contains("F4004"), "expected the frame fatal:\n{o}");
    assert!(
        !o.contains("VERDICT-AFTER-FATAL"),
        "the process must stop at the fatal, not run on and print a verdict:\n{o}"
    );
    assert!(
        o.contains("ended (Error)"),
        "the run must end as Error, not Finish:\n{o}"
    );
}

/// The five effects that were SILENT in that same unroutable position — `$fopen`,
/// `$value$plusargs`, a seeded `$random`, a seeded `$dist_*` and `$cast` — returning 0
/// at exit 0 with no diagnostic, right next to a loud `$fgets`. The frame gate now
/// defers to the one canonical family predicate instead of its own id list.
#[test]
fn every_statement_effect_is_loud_where_it_cannot_be_routed() {
    for (name, decls, body) in [
        (
            "fopen",
            "int nfd;",
            r#"nfd = $fopen("o.txt", "w"); return nfd;"#,
        ),
        (
            "plusargs",
            "int lim = 20; int rc;",
            r#"rc = $value$plusargs("LIM=%d", lim); return lim;"#,
        ),
        (
            "random",
            "int seed = 1; int a;",
            "a = $random(seed); return seed;",
        ),
        (
            "dist",
            "int seed = 1; int a;",
            "a = $dist_uniform(seed, 0, 99); return seed;",
        ),
        ("cast", "int d = 0; int ok;", "ok = $cast(d, 7); return d;"),
    ] {
        let (o, ok) = run(&format!(
            r#"`timescale 1ns/1ps
class C;
  function int act ();
    {decls}
    {body}
  endfunction
endclass
module t;
  initial begin C c; int r; c = new(); r = c.act(); $display("r=%0d", r); $finish; end
endmodule"#
        ));
        assert!(!ok, "{name}: an unperformable effect must be loud:\n{o}");
        assert!(o.contains("F4004"), "{name}: expected F4004:\n{o}");
    }
}

/// `$sformatf` must NOT join that gate: it is a `StmtEffect` on the process path, but
/// the frame executor has a working intercept for it. Marking it would have moved
/// correct designs onto a path they do not need.
#[test]
fn sformatf_still_works_in_an_unroutable_position() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
class C;
  function int act ();
    string s;
    s = $sformatf("v=%0d", 42);
    return (s == "v=42");
  endfunction
endclass
module t;
  initial begin C c; int r; c = new(); r = c.act(); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("r=1"), "expected r=1:\n{o}");
}

// ── §3.4 explicit `static` lifetime ──────────────────────────────────────────

/// IEEE 1800 §13.3. `automatic` parsed, `static` produced ten E2002s off one header.
#[test]
fn an_explicit_static_lifetime_parses_on_tasks_and_functions() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  function static int fn (input int a);
    return a + 1;
  endfunction
  task static tk (input int a, output int b);
    b = fn(a);
  endtask
  initial begin int r; tk(1, r); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(o.contains("r=2"), "expected r=2:\n{o}");
}

/// `static` is not a reserved word in this lexer, so a subprogram NAMED `static` is
/// legal Verilog-2005 and must keep parsing. The discriminator is the token after it:
/// a lifetime is followed by more header, a name by `;` or `(`.
#[test]
fn a_subprogram_named_static_still_parses() {
    let (o, ok) = run(r#"`timescale 1ns/1ps
module t;
  task static; $display("named-task"); endtask
  function int static2 (input int a); return a; endfunction
  initial begin int r; static; r = static2(3); $display("r=%0d", r); $finish; end
endmodule"#);
    assert!(ok, "expected exit 0:\n{o}");
    assert!(
        o.contains("named-task"),
        "expected the named task to run:\n{o}"
    );
    assert!(o.contains("r=3"), "expected r=3:\n{o}");
}
