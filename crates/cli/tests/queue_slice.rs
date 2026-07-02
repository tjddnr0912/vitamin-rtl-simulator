//! Queue slice read `dst = src[a:b]` (IEEE 1800 §7.10.1) — hand-IEEE pinned.
//!
//! Icarus 13.0 is a BROKEN oracle for this form: it parses `q[1:2]` but
//! silently ignores the bounds (returns the whole queue) and rejects `q[2:$]`
//! as a syntax error. The teeth here are therefore (a) the LRM clamp/empty
//! rules and (b) the vita-internal equivalence `q[a:b] ≡ a manual element
//! loop` (the strongest oracle-free differential, doc'd loop pattern).
//! Bounds are runtime expressions; `$` = last index. Partial out-of-range
//! CLAMPS; reversed / fully-out / x-z bounds yield the EMPTY queue; a bounded
//! destination truncates like every other queue write.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_qsl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

const PUSH4: &str = "q.push_back(10); q.push_back(20); q.push_back(30); q.push_back(40);\n";

#[test]
fn slice_basic_dollar_and_single() {
    let (out, _, code) = run(&format!(
        "module t; int q[$]; int r[$]; initial begin\n{PUSH4}\
           r = q[1:2];\n\
           $display(\"s12=%0d [%0d %0d]\", r.size(), r[0], r[1]);\n\
           r = q[2:$];\n\
           $display(\"s2d=%0d [%0d %0d]\", r.size(), r[0], r[1]);\n\
           r = q[0:0];\n\
           $display(\"one=%0d [%0d]\", r.size(), r[0]);\n\
           $finish; end endmodule\n"
    ));
    assert_eq!(code, 0);
    assert!(out.contains("s12=2 [20 30]"), "middle slice:\n{out}");
    assert!(out.contains("s2d=2 [30 40]"), "$-bound slice:\n{out}");
    assert!(out.contains("one=1 [10]"), "single-element slice:\n{out}");
}

#[test]
fn slice_clamp_and_empty_rules() {
    // Partial out-of-range clamps to the valid tail; reversed and fully-out
    // ranges are empty; a negative low bound clamps to 0 (LRM §7.10.1).
    let (out, _, _) = run(&format!(
        "module t; int q[$]; int r[$]; int neg; initial begin\n{PUSH4}\
           r = q[2:9];\n\
           $display(\"clamp=%0d [%0d %0d]\", r.size(), r[0], r[1]);\n\
           r = q[3:1];\n\
           $display(\"rev=%0d\", r.size());\n\
           r = q[5:9];\n\
           $display(\"out=%0d\", r.size());\n\
           neg = -2;\n\
           r = q[neg:1];\n\
           $display(\"negclamp=%0d [%0d %0d]\", r.size(), r[0], r[1]);\n\
           $finish; end endmodule\n"
    ));
    assert!(out.contains("clamp=2 [30 40]"), "high clamp:\n{out}");
    assert!(out.contains("rev=0"), "reversed empty:\n{out}");
    assert!(out.contains("out=0"), "fully-out empty:\n{out}");
    assert!(
        out.contains("negclamp=2 [10 20]"),
        "negative low clamps to 0:\n{out}"
    );
}

#[test]
fn slice_runtime_bounds_and_self() {
    // Bounds are runtime expressions; a self-slice `q = q[a:b]` clones the
    // subrange BEFORE replacing q (no read-after-wipe hazard).
    let (out, _, _) = run(&format!(
        "module t; int q[$]; int r[$]; int a; int b; initial begin\n{PUSH4}\
           a = 1; b = 2;\n\
           r = q[a:b+0];\n\
           $display(\"rt=%0d [%0d %0d]\", r.size(), r[0], r[1]);\n\
           q = q[1:2];\n\
           $display(\"self=%0d [%0d %0d]\", q.size(), q[0], q[1]);\n\
           $finish; end endmodule\n"
    ));
    assert!(out.contains("rt=2 [20 30]"), "runtime bounds:\n{out}");
    assert!(out.contains("self=2 [20 30]"), "self-slice:\n{out}");
}

#[test]
fn slice_xz_bound_is_empty_with_warning() {
    let (out, err, _) = run(&format!(
        "module t; int q[$]; int r[$]; logic [3:0] xb; initial begin\n{PUSH4}\
           r = q[xb:2];\n\
           $display(\"xz=%0d\", r.size());\n\
           $finish; end endmodule\n"
    ));
    assert!(out.contains("xz=0"), "x bound → empty:\n{out}");
    assert!(err.contains("X/Z"), "x bound warns:\n{err}");
}

#[test]
fn slice_is_deep_and_equivalent_to_manual_loop() {
    // The strongest oracle-free teeth: the slice must equal the manual
    // element loop, and be value-independent of the source afterwards.
    let (out, _, code) = run(&format!(
        "module t; int q[$]; int r[$]; int m[$]; int i; initial begin\n{PUSH4}\
           for (i = 1; i <= 2; i = i + 1) m.push_back(q[i]);\n\
           r = q[1:2];\n\
           if (r.size() != m.size() || r[0] != m[0] || r[1] != m[1])\n\
             $fatal(1, \"slice != manual loop\");\n\
           q[1] = 99;\n\
           if (r[0] != 20) $fatal(1, \"slice not deep\");\n\
           $display(\"equiv-deep ok\");\n\
           $finish; end endmodule\n"
    ));
    assert_eq!(code, 0, "equivalence must hold:\n{out}");
    assert!(out.contains("equiv-deep ok"), "{out}");
}

#[test]
fn slice_into_bounded_queue_truncates() {
    // The result lands through the same bounded-queue post-op as every other
    // queue write ([$:1] keeps at most 2 elements).
    let (out, _, _) = run(&format!(
        "module t; int q[$]; int b1[$:1]; initial begin\n{PUSH4}\
           b1 = q[0:3];\n\
           $display(\"b1=%0d [%0d %0d]\", b1.size(), b1[0], b1[1]);\n\
           $finish; end endmodule\n"
    ));
    assert!(
        out.contains("b1=2 [10 20]"),
        "bounded dst truncates:\n{out}"
    );
}

#[test]
fn slice_empty_source_and_mismatches_loud() {
    // An untouched src slices to empty; elem-type and non-queue targets loud.
    let (out, _, _) = run("module t; int q[$]; int r[$]; initial begin\n\
           r.push_back(7);\n\
           r = q[0:2];\n\
           $display(\"es=%0d\", r.size());\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("es=0"),
        "empty src → empty (wipes dst):\n{out}"
    );
    let (_, err, code) = run("module t; int q[$]; byte b[$]; initial begin\n\
           q.push_back(1); b = q[0:0];\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(err.contains("matching element types"), "elem loud:\n{err}");
    let (_, err, code) = run("module t; int q[$]; int x; initial begin\n\
           q.push_back(1); x = q[0:0];\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(!err.is_empty(), "non-queue target loud:\n{err}");
}

#[test]
fn staged_queue_slice_parity() {
    // vcmp→velab→vrun must carry the queue_slice_stmts sidecar — a dropped
    // marker would leave r untouched and the $fatal guard fires.
    let src = "module t; int q[$]; int r[$]; initial begin\n\
           q.push_back(10); q.push_back(20); q.push_back(30);\n\
           r = q[1:2];\n\
           if (r.size() != 2 || r[0] != 20 || r[1] != 30)\n\
             $fatal(1, \"staged queue slice dropped\");\n\
           $finish;\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_qsls_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, src).unwrap();
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged queue slice dropped (queue_slice_stmts sidecar)"
    );
}
