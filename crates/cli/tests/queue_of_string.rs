//! T1-4: `string q[$]` — an unbounded QUEUE of strings.
//!
//! Oracle: iverilog, live, on every shape here except the byte select (which iverilog
//! rejects on a container element; the hand-IEEE answer is pinned against the fixed and
//! dynamic array twins that ARE oracle-verified).
//!
//! Admitting the dimension was not the work. The first attempt did exactly that and
//! shipped a silent-wrong: `q.size()` was right and **every element read back empty**
//! (iverilog's `2 aa bb` became `2   `). The queue push and insert paths each did their
//! own `.resize(w)`, and a string handle net has width 0 → `max(1)` → the byte string
//! truncated to a single bit. The dyn-ARRAY element write already had the branch that
//! avoids this; the queue paths had never needed it.
//!
//! So the fix is one funnel — `SimState::coerce_dyn_elem` — that every dynamic-container
//! element write now shares, keyed on `dyn_str_elem` (the same flag that makes the
//! engine hold these elements as byte strings in the first place). The regression teeth
//! for the funnel are the byte-queue truncation cases below: `push_back(300)` into a
//! `byte q[$]` must still store 44.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn compile(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_qos_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            }),
        out.status.success(),
    )
}

fn run(body: &str) -> String {
    let src = format!(
        "module m;\n  string q[$];\n  initial begin\n{body}\n    $finish;\n  end\nendmodule\n"
    );
    let (out, ok) = compile(&src);
    assert!(ok, "expected success for:\n{src}");
    out
}

#[test]
fn push_back_size_and_index() {
    // iverilog: 2 aa bb — the shape the first attempt turned into `2   `.
    assert_eq!(
        run("    q.push_back(\"aa\"); q.push_back(\"bb\");\n\
             $display(\"%0d %s %s\", q.size(), q[0], q[1]);"),
        "2 aa bb\n"
    );
}

#[test]
fn push_front_and_pop_front() {
    // iverilog: zz|aa then popped=zz left=1 head=aa
    assert_eq!(
        run("    string x;\n\
             q.push_back(\"aa\"); q.push_front(\"zz\");\n\
             $display(\"%s|%s\", q[0], q[1]);\n\
             x = q.pop_front();\n\
             $display(\"popped=%s left=%0d head=%s\", x, q.size(), q[0]);"),
        "zz|aa\npopped=zz left=1 head=aa\n"
    );
}

#[test]
fn insert_and_delete() {
    // iverilog: 3 aa|mid|bb then 2 aa|bb. `insert` had the same private resize as push.
    assert_eq!(
        run(
            "    q.push_back(\"aa\"); q.push_back(\"bb\"); q.insert(1,\"mid\");\n\
             $display(\"%0d %s|%s|%s\", q.size(), q[0], q[1], q[2]);\n\
             q.delete(1);\n\
             $display(\"%0d %s|%s\", q.size(), q[0], q[1]);"
        ),
        "3 aa|mid|bb\n2 aa|bb\n"
    );
}

#[test]
fn foreach_and_runtime_index() {
    // iverilog: 0:aa / 1:bb, then bb
    assert_eq!(
        run("    int k;\n\
             q.push_back(\"aa\"); q.push_back(\"bb\");\n\
             foreach(q[j]) $display(\"%0d:%s\", j, q[j]);\n\
             k=1; $display(\"%s\", q[k]);"),
        "0:aa\n1:bb\nbb\n"
    );
}

#[test]
fn decl_init_pattern_at_module_scope() {
    // iverilog: 3 aa bb cc. The decl-init collectors gate on the container dim, so
    // admitting `[$]` without widening them dropped the init SILENTLY — the int-queue
    // twin initialised fine while the string queue came out size 0 with no diagnostic.
    let (out, ok) = compile(
        "module m; string q[$] = '{\"aa\",\"bb\",\"cc\"};\n\
         initial begin $display(\"%0d %s %s %s\", q.size(), q[0], q[1], q[2]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected success");
    assert_eq!(out, "3 aa bb cc\n");
}

#[test]
fn decl_init_pattern_in_a_block_local() {
    // The two decl-init collectors must stay in step — one shared predicate now.
    let (out, ok) = compile(
        "module m;\n\
         initial begin string q[$] = '{\"aa\",\"bb\"}; $display(\"%0d %s %s\", q.size(), q[0], q[1]); end\n\
         initial #1 $finish;\n\
         endmodule\n",
    );
    assert!(ok, "expected success");
    assert_eq!(out, "2 aa bb\n");
}

#[test]
fn element_keeps_the_string_domain() {
    // iverilog: 3, [abc!], 1 — the element must behave as a string everywhere a scalar
    // string would, not as the packed value of a width-0 handle.
    assert_eq!(
        run("    q.push_back(\"abc\");\n\
             $display(\"%0d\", q[0].len());\n\
             $display(\"[%s]\", {q[0],\"!\"});\n\
             $display(\"%0d\", q[0]==\"abc\");"),
        "3\n[abc!]\n1\n"
    );
}

#[test]
fn element_byte_select() {
    // hand-IEEE: iverilog rejects a byte select on a container element. 119 is 'w',
    // pinned by the oracle-verified fixed (`string s[2]`) and dynamic twins.
    assert_eq!(
        run("    q.push_back(\"wa\"); $display(\"%0d\", q[0][0]);"),
        "119\n"
    );
}

#[test]
fn element_write_and_assignment() {
    // iverilog: zz then aa
    assert_eq!(
        run("    string x;\n\
             q.push_back(\"aa\"); x = q[0]; q[0] = \"zz\";\n\
             $display(\"%s %s\", q[0], x);"),
        "zz aa\n"
    );
}

#[test]
fn an_empty_string_queue_reports_size_zero() {
    assert_eq!(run("    $display(\"sz=%0d\", q.size());"), "sz=0\n");
}

// ── the funnel must not disturb non-string elements ──────────────────────────

#[test]
fn byte_queue_still_truncates() {
    // REGRESSION GUARD for `coerce_dyn_elem`: §5.5 assignment semantics still apply to
    // an integral element — `push_back(300)` into a `byte q[$]` stores 44, and `insert`
    // takes the same recipe. iverilog: 45 44.
    let (out, ok) = compile(
        "module m; byte q[$];\n\
         initial begin q.push_back(300); q.insert(0,301); \
         $display(\"%0d %0d\", q[0], q[1]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected success");
    assert_eq!(out, "45 44\n");
}

#[test]
fn int_queue_is_unchanged() {
    // The whole int-queue surface is byte-identical to before the funnel. iverilog:
    // 3|7 then 3 9 7.
    let (out, ok) = compile(
        "module m; int q[$];\n\
         initial begin q.push_back(7); q.push_front(3); $display(\"%0d|%0d\", q[0], q[1]);\n\
           q.delete(0); q.push_back(2); q.insert(0,9);\n\
           $display(\"%0d %0d %0d\", q.size(), q[0], q[1]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "expected success");
    assert_eq!(out, "3|7\n3 9 7\n");
}

#[test]
fn a_bounded_string_queue_stays_loud() {
    // `[$:N]` is loud for every element type in the MVP; admitting `[$]` must not have
    // widened that.
    let (_, ok) = compile(
        "module m; string q[$:3];\n\
         initial begin q.push_back(\"a\"); $display(\"%0d\", q.size()); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok, "expected a loud reject for a bounded string queue");
}
