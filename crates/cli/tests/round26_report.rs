//! External report round-26 (2026-08-03) — §3, the one remaining item.
//!
//! R25 §3.2 replaced a wrong four-way list with an explanation that names the real
//! construct. The explanation was correct; where it was SPLICED was not. The E3009
//! template is `body uses {reason}, which is outside the frame-call subset (…)`, and the
//! new reason ended with "…the same assignment in a `task` body, or in a module process,
//! does work" — so the trailing clause landed on THAT, and the message stated that the
//! WORKING form was the unsupported one.
//!
//! The fix is structural rather than cosmetic (parenthesising the explanation would have
//! worked once and left the next author the same trap): the classifier now returns
//! `(what, detail)`, where `what` is a bare noun phrase the subset clause attaches to and
//! `detail` is appended after it as its own sentence. These pins assert the ORDER, not
//! the wording — anything that puts an explanation before the subset clause fails them.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_src(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r26_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_vita"));
    c.arg(f.to_str().unwrap()).current_dir(&d);
    let out = c.output().expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (text, out.status.code())
}

/// The property, stated once: whatever the reason is, the subset clause must attach to
/// the construct — so no part of the explanation may appear BEFORE it. `does work` is
/// the specific phrase the report caught, and it is the worst possible one to strand
/// there (it inverts the message), so it is checked by name as well.
fn subset_clause_attaches_to_the_construct(o: &str) {
    const CLAUSE: &str = "which is outside the frame-call subset";
    let i = o
        .find(CLAUSE)
        .unwrap_or_else(|| panic!("no subset clause in the message:\n{o}"));
    let head = &o[..i];
    assert!(
        !head.contains("does work"),
        "the subset clause attached to the WORKING form — the message now says the form \
         that works is the one outside the subset:\n{o}"
    );
    // Belt and braces: the head is the construct, so it is short. The pre-fix message
    // had 270+ characters of explanation in front of the clause.
    let head = head.split("body uses ").nth(1).unwrap_or(head);
    assert!(
        head.len() < 80,
        "an explanation is spliced in front of the subset clause ({} chars): {head:?}\n{o}",
        head.len()
    );
}

/// §3 as reported: `s[i] = v` in a frame FUNCTION body.
#[test]
fn the_strputc_reject_reads_as_one_sentence() {
    let (o, ok) = run_src(
        "`timescale 1ns/1ps\n\
         module t;\n\
           function automatic string fn ();\n\
             string s;\n\
             s = \"zz\";\n\
             s[1] = 66;\n\
             return s;\n\
           endfunction\n\
           initial begin\n\
             string r;\n\
             r = fn();\n\
             if (r == \"zB\") $display(\"PASS\"); else $display(\"BAD got='%s'\", r);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert_ne!(ok, Some(0), "must still be loud:\n{o}");
    assert!(o.contains("error[VITA-E3009]"), "expected E3009:\n{o}");
    subset_clause_attaches_to_the_construct(&o);
    // R25 §3.2's content must survive the reshuffle: the construct is still named, and
    // the two forms that DO work are still offered.
    assert!(
        o.contains("string element assignment"),
        "the construct is no longer named:\n{o}"
    );
    assert!(
        o.contains("does work"),
        "the working alternatives were dropped, not moved:\n{o}"
    );
}

/// The terminator arm (R23 §4) carries the other long explanation. It escaped the report
/// only because its explanation happened to be wrapped in parentheses — the same splice,
/// one punctuation mark away from the same misreading. It gets the same shape and the
/// same pin.
#[test]
fn the_output_formal_call_reject_reads_as_one_sentence() {
    let (o, ok) = run_src(
        "`timescale 1ns/1ps\n\
         module t;\n\
           function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction\n\
           function automatic int outer (input int a);\n\
             automatic int r, loc; r = nxt(a, loc); return r + loc; endfunction\n\
           initial begin int d; d = outer(5); $display(\"d=%0d\", d); $finish; end\n\
         endmodule\n",
    );
    assert_ne!(ok, Some(0), "must still be loud:\n{o}");
    assert!(o.contains("error[VITA-E3009]"), "expected E3009:\n{o}");
    subset_clause_attaches_to_the_construct(&o);
    assert!(
        o.contains("output/inout formal"),
        "the construct is no longer named:\n{o}"
    );
    assert!(
        o.contains("does work"),
        "the working alternatives were dropped, not moved:\n{o}"
    );
}

/// A short reason must not grow a stray sentence terminator when it has no detail — the
/// `(what, "")` arms are the majority and the template has to read correctly for them too.
#[test]
fn a_reason_with_no_detail_still_ends_cleanly() {
    let (o, _) = run_src(
        "`timescale 1ns/1ps\n\
         module t;\n\
           int gv;\n\
           function automatic int f (input int a);\n\
             gv <= a; return a; endfunction\n\
           initial begin int d; d = f(5); $display(\"d=%0d\", d); $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("error[VITA-E3009]"), "expected E3009:\n{o}");
    subset_clause_attaches_to_the_construct(&o);
    let line = o
        .lines()
        .find(|l| l.contains("VITA-E3009"))
        .expect("the diagnostic line");
    assert!(
        line.trim_end().ends_with("are supported)"),
        "a detail-free reason must end at the subset clause: {line:?}"
    );
}
