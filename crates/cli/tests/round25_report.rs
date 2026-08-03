//! External report round-25 (2026-08-03) — the four items, pinned.
//!
//! Two were carried over from earlier rounds and two were isolated by the reporter while
//! profiling a real workload against Xcelium. The performance one (§3.3) was the largest
//! single defect this repository has measured: a `string` element read inside a
//! subroutine body was O(len), so a per-character loop was O(len^2), and on the
//! reporter's CAVP walker that was 99.4% of the testbench's cost.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_src(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r25_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg(f.to_str().unwrap()).args(args).current_dir(&d);
    let out = c.output().expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

/// §3.1 — `automatic <unpacked struct>` as a SUBROUTINE local.
///
/// `tf_body`'s `automatic` branch knew two kinds of declaration, a builtin
/// (`net_var_kind`) and a typedef (`peek_block_typedef_decl`). An unpacked struct type
/// is neither — it lives in `unpacked_struct_layouts` — so both arms missed, the
/// fallthrough `bump()`ed the TYPE NAME, and the remaining `r;` parsed as a task enable:
/// `E3010: call to undeclared task \`r\``. `block_body` had the identical hole and fixed
/// it in R18 §3.3; this is the subroutine twin.
#[test]
fn an_automatic_unpacked_struct_local_in_a_subroutine_resolves() {
    let src = "module t;\n\
        typedef struct { int n; string h; } rec_t;\n\
        task automatic tk (output int ok);\n\
          automatic rec_t r;\n\
          r.n = 1; r.h = \"zz\";\n\
          ok = (r.h == \"zz\" && r.n == 1);\n\
        endtask\n\
        initial begin int ok = 0; tk(ok);\n\
          if (ok) $display(\"PASS\"); else $display(\"BAD\");\n\
          $finish; end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(o.contains("PASS"), "automatic struct local in a task:\n{o}");
    assert!(!o.contains("E3010"), "must not read as a task enable:\n{o}");
}

/// §3.1 boundary — a FUNCTION body takes the same path.
#[test]
fn the_same_declaration_works_in_a_function_body() {
    let src = "module t;\n\
        typedef struct { int n; } rec_t;\n\
        function automatic int fn ();\n\
          automatic rec_t r;\n\
          r.n = 7;\n\
          return r.n;\n\
        endfunction\n\
        initial begin\n\
          if (fn() == 7) $display(\"PASS\"); else $display(\"BAD\");\n\
          $finish; end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(
        o.contains("PASS"),
        "automatic struct local in a function:\n{o}"
    );
}

/// §3.2 — the rejection must name the statement that is actually there.
///
/// `s[i] = v` on a `string` lowers to a `StrPutC` system task, and the catch-all arm
/// reported every rejected statement as "$systask / nonblocking / force / release".
/// None of those four appears in the source, and the same message calls the offending
/// line supported ("blocking assigns to its own locals"). The restriction is right — a
/// function body has no call statement to carry the write out — only the wording was
/// not. Same defect class as R23 §4 on the terminator arm.
#[test]
fn the_frame_subset_rejection_names_the_real_statement() {
    let src = "module t;\n\
        function automatic string fn ();\n\
          string s; s = \"zz\"; s[1] = 66; return s;\n\
        endfunction\n\
        initial begin string r; r = fn(); $display(\"r=%s\", r); $finish; end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(o.contains("E3009"), "must still be loud:\n{o}");
    assert!(
        o.contains("string element assignment"),
        "must name the statement that is present:\n{o}"
    );
    assert!(
        !o.contains("$systask / nonblocking / force / release"),
        "must not list four causes the source does not contain:\n{o}"
    );
}

/// §3.2 boundary — the same assignment in a TASK body works, and the message says so.
#[test]
fn a_string_element_assignment_in_a_task_body_works() {
    let src = "module t;\n\
        task automatic tk (output string o);\n\
          string s; s = \"zz\"; s[1] = 66; o = s;\n\
        endtask\n\
        initial begin string r; tk(r);\n\
          if (r == \"zB\") $display(\"PASS\"); else $display(\"BAD got='%s'\", r);\n\
          $finish; end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(o.contains("PASS"), "string putc in a task body:\n{o}");
}

/// §3.3 — a `string` element read inside a subroutine must be O(1), not O(len).
///
/// `.getc()` materialised the whole string on every call: for a NET it cloned the heap
/// bytes, and for a frame FORMAL it built the packed-ASCII `Value` (8 bits per
/// character) and unpacked it BIT BY BIT — 128,000 `get_vu` calls per character read of
/// a 16,000-character string. A per-character loop was therefore O(len^2). Measured by
/// the reporter on their `hex2bytes`: 0.12 / 0.40 / 1.74 / 6.90 s at N = 4k / 8k / 16k /
/// 32k, quadrupling per doubling.
///
/// THE DISCRIMINATOR IS THE POINT. "Per-access O(len)" is only visible when the ACCESS
/// COUNT is held fixed and the STRING LENGTH varies — vary both and a linear
/// implementation doubles too, which is what the first version of this test did, and it
/// passed with the fast path deleted. The string is also built by repeated doubling
/// (`b = {b, b}`), because the obvious `b = {b, "a"}` loop is itself O(len^2) and would
/// dominate the measurement.
///
/// Same reads, twice the string: O(1) per access stays flat, O(len) doubles.
#[test]
fn a_string_element_read_in_a_subroutine_is_not_quadratic() {
    let src = "module t;\n\
        function automatic int f (input string s, input int reads);\n\
          int acc = 0;\n\
          for (int i = 0; i < reads; i++) acc += s.getc(i);\n\
          return acc;\n\
        endfunction\n\
        function automatic string mk (input int n);\n\
          string b = \"a\";\n\
          while (b.len() < n) b = {b, b};\n\
          return b;\n\
        endfunction\n\
        int N = 4096, reps = 200, r = 0;\n\
        string hex;\n\
        initial begin\n\
          void'($value$plusargs(\"N=%d\", N));\n\
          hex = mk(N);\n\
          for (int k = 0; k < reps; k++) r = f(hex, 2048);\n\
          $display(\"acc=%0d\", r);\n\
          $finish;\n\
        end\n\
      endmodule\n";

    let t = |n: &str| -> f64 {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let s = std::time::Instant::now();
            let (o, _) = run_src(src, &[&format!("+N={n}")]);
            assert!(o.contains("acc="), "run failed:\n{o}");
            best = best.min(s.elapsed().as_secs_f64());
        }
        best
    };
    // 2048 reads either way; only the string behind them grows 8x.
    let small = t("4096");
    let big = t("32768");
    assert!(
        big < small * 2.5,
        "same read count over an 8x longer string must not cost materially more \
         (4096-char {small:.3}s vs 32768-char {big:.3}s) — a per-access O(len) read \
         would be several times slower"
    );
}

/// §3.3 correctness — the fast path must return the same bytes as the slow one did.
#[test]
fn string_element_reads_are_correct_from_every_operand_kind() {
    let src = "module t;\n\
        function automatic int from_formal (input string s, input int i);\n\
          return s.getc(i);\n\
        endfunction\n\
        function automatic int from_local (input int i);\n\
          string s = \"ABCDEF\";\n\
          return s.getc(i);\n\
        endfunction\n\
        string m = \"ABCDEF\";\n\
        initial begin\n\
          $display(\"net=%0d %0d %0d\", m.getc(0), m.getc(5), m.getc(9));\n\
          $display(\"formal=%0d %0d %0d\", from_formal(m,0), from_formal(m,5), from_formal(m,9));\n\
          $display(\"local=%0d %0d %0d\", from_local(0), from_local(5), from_local(9));\n\
          $finish;\n\
        end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    // 'A' = 65, 'F' = 70, out of range reads 0 (IEEE §6.16.2).
    for want in ["net=65 70 0", "formal=65 70 0", "local=65 70 0"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

/// §3.4 — a long loop at time 0 is not an oscillation.
///
/// The in-body guard counted BLOCK STEPS but bounded them with `max_deltas`, and
/// reported the overrun as "zero-delay loop / combinational oscillation". A plain
/// `for (i = 0; i < 500000; i++)` has neither. Measured by the reporter: 400,000 passed,
/// 500,000 was fatal, and 2,000,000 with twenty `#1`s passed — i.e. the counter, not
/// convergence.
#[test]
fn a_long_loop_at_time_zero_is_not_reported_as_oscillation() {
    let src = "module t;\n\
        initial begin\n\
          int i, acc = 0;\n\
          for (i = 0; i < 2000000; i++) acc += i;\n\
          $display(\"PASS acc=%0d\", acc);\n\
          $finish;\n\
        end\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(
        o.contains("PASS acc="),
        "a loop with no feedback must run:\n{o}"
    );
    assert!(!o.contains("F4016"), "must not claim non-convergence:\n{o}");
    assert!(!o.contains("F4027"), "must not hit the step budget:\n{o}");
}

/// §3.4 the other side — an UNBOUNDED loop must still be loud, and must say what it is.
#[test]
fn an_unbounded_loop_is_still_loud_and_names_the_real_condition() {
    let src = "module t;\n\
        int x = 0;\n\
        initial begin while (1) x = x + 1; end\n\
        initial #10 $finish;\n\
      endmodule\n";
    let (o, _) = run_src(src, &[]);
    assert!(
        o.contains("F4027"),
        "an unbounded loop must stay loud:\n{o}"
    );
    assert!(
        o.contains("without reaching a"),
        "must name what was observed — a process that never suspended:\n{o}"
    );
    assert!(
        !o.contains("combinational oscillation"),
        "must not assert a cause it did not observe:\n{o}"
    );
}
