//! Whole-handle copy `dst = src` for dynamic storage (IEEE 1800 §7.5.1 dyn /
//! §7.9 assoc / §7.10 queue) — VALUE semantics: a DEEP copy, so later writes
//! to either side never show through. iverilog-pinned for dyn/queue; assoc is
//! hand-IEEE (iverilog rejects assoc declarations). Lowered as a no-op
//! Display with a `handle_copy_stmts` marker (the timeformat pattern); the
//! engine deep-clones `dyn_heap[src]` into `dyn_heap[dst]`.
//!
//! NOTE: iverilog 13.0 also PARSES the queue-slice form `q[1:2]` but silently
//! ignores the bounds (returns the whole queue) and rejects `q[2:$]` — a
//! self-inconsistent oracle, so the slice form stays honest-loud in vita
//! (recorded follow-on; §1(e) rule: vita targets the spec, not a broken
//! oracle).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hcp_{}_{n}", std::process::id()));
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

#[test]
fn queue_copy_is_deep() {
    // iverilog-pinned: r = q copies the VALUE — a later q.push_back and a
    // later r[0] write are both invisible to the other side.
    let (out, _, code) = run("module t; int q[$]; int r[$]; initial begin\n\
           q.push_back(10); q.push_back(20);\n\
           r = q;\n\
           q.push_back(30);\n\
           $display(\"deep r=%0d q=%0d r0=%0d\", r.size(), q.size(), r[0]);\n\
           r[0] = 99;\n\
           $display(\"indep q0=%0d r0=%0d\", q[0], r[0]);\n\
           q = r;\n\
           $display(\"back q=%0d q0=%0d\", q.size(), q[0]);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("deep r=2 q=3 r0=10"), "deep copy:\n{out}");
    assert!(out.contains("indep q0=10 r0=99"), "independence:\n{out}");
    assert!(out.contains("back q=2 q0=99"), "back copy:\n{out}");
}

#[test]
fn dyn_array_copy_is_deep() {
    // iverilog-pinned: b = a on dynamic arrays deep-copies too.
    let (out, _, code) = run("module t; int a[]; int b[]; initial begin\n\
           a = new[2]; a[0]=5; a[1]=6;\n\
           b = a;\n\
           a[0] = 77;\n\
           $display(\"dyn b=%0d b0=%0d a0=%0d\", b.size(), b[0], a[0]);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("dyn b=2 b0=5 a0=77"), "dyn deep copy:\n{out}");
}

#[test]
fn assoc_copy_is_deep_hand_ieee() {
    // §7.9 (hand-IEEE — iverilog rejects assoc declarations): same deep-copy
    // value semantics through the uniform heap clone.
    let (out, _, code) = run("module t; integer aa[integer]; integer bb[integer]; initial begin\n\
           aa[5]=50; aa[7]=70;\n\
           bb = aa;\n\
           aa[5]=99;\n\
           $display(\"assoc bb5=%0d bbn=%0d aa5=%0d ex7=%0d\", bb[5], bb.num(), aa[5], bb.exists(7));\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("assoc bb5=50 bbn=2 aa5=99 ex7=1"),
        "assoc deep copy:\n{out}"
    );
}

#[test]
fn copy_of_untouched_source_is_empty() {
    // A never-touched src (lazy None heap slot) copies as EMPTY — and wipes
    // any previous dst contents (value semantics, not a merge).
    let (out, _, code) = run("module t; int q[$]; int r[$]; initial begin\n\
           r.push_back(1); r.push_back(2);\n\
           r = q;\n\
           $display(\"empty r=%0d q=%0d\", r.size(), q.size());\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("empty r=0 q=0"),
        "empty-src copy wipes dst:\n{out}"
    );
}

#[test]
fn self_copy_is_noop() {
    let (out, _, code) = run("module t; int q[$]; initial begin\n\
           q.push_back(1);\n\
           q = q;\n\
           $display(\"self q=%0d q0=%0d\", q.size(), q[0]);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("self q=1 q0=1"), "self-copy no-op:\n{out}");
}

#[test]
fn bounded_queue_copy_truncates_to_bound() {
    // §7.10.2 (R1 both-lens converged finding): a whole-copy into a BOUNDED
    // queue truncates to bound+1 like every push/insert post-op — the raw
    // clone used to overfill silently (iverilog-pinned: [$:2] src-4 → 3,
    // [$:0] src-3 → 1; the follow-up push behaves as if the copy truncated).
    let (out, _, code) = run(
        "module t; int q[$]; int b2[$:2]; int b0[$:0]; int b1[$:1]; initial begin\n\
           q.push_back(10); q.push_back(20); q.push_back(30); q.push_back(40);\n\
           b2 = q;\n\
           $display(\"b2=%0d last=%0d\", b2.size(), b2[2]);\n\
           b0 = q;\n\
           $display(\"b0=%0d only=%0d\", b0.size(), b0[0]);\n\
           b1 = q;\n\
           b1.push_back(99);\n\
           $display(\"b1=%0d\", b1.size());\n\
           $finish; end endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(out.contains("b2=3 last=30"), "[$:2] truncates to 3:\n{out}");
    assert!(out.contains("b0=1 only=10"), "[$:0] truncates to 1:\n{out}");
    assert!(
        out.contains("b1=2"),
        "post-copy push keeps the bound:\n{out}"
    );
}

#[test]
fn kind_and_elem_type_mismatches_are_loud() {
    // queue → dyn (kind mismatch) and int-queue → byte-queue (element type)
    // must both fail loud — a raw clone would carry uncoerced bits.
    let (_, err, code) = run("module t; int q[$]; int d[]; initial begin\n\
           q.push_back(1); d = q;\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(
        err.contains("SAME dynamic-storage kind"),
        "kind loud:\n{err}"
    );
    let (_, err, code) = run("module t; int q[$]; byte b[$]; initial begin\n\
           q.push_back(1); b = q;\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(
        err.contains("matching element types"),
        "elem-type loud:\n{err}"
    );
}

#[test]
fn non_handle_targets_stay_loud() {
    // `x = q` keeps the pre-existing whole-value-surface loud. (The queue
    // slice `r = q[1:2]` graduated to a supported §7.10.1 read — see
    // queue_slice.rs — so its old loud assert moved there as positives.)
    let (_, err, code) = run("module t; int q[$]; int x; initial begin\n\
           q.push_back(1); x = q;\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(err.contains("no whole-value surface"), "read loud:\n{err}");
}

#[test]
fn nba_handle_copy_stays_loud() {
    // Only the BLOCKING form is a §7.10 copy statement here; `r <= q` keeps
    // its existing loud path (no silent half-support).
    let (_, err, code) = run("module t; int q[$]; int r[$]; initial begin\n\
           q.push_back(1); r <= q;\n\
           end endmodule\n");
    assert_ne!(code, 0);
    assert!(!err.is_empty(), "NBA copy loud:\n{err}");
}

#[test]
fn staged_handle_copy_parity() {
    // vcmp→velab→vrun must carry the handle_copy_stmts sidecar (17th-field
    // append) — a dropped marker would print an empty Display and leave r
    // empty → the $fatal guard fires.
    let src = "module t; int q[$]; int r[$]; initial begin\n\
           q.push_back(42);\n\
           r = q;\n\
           if (r.size() != 1 || r[0] != 42) $fatal(1, \"staged handle copy dropped\");\n\
           $finish;\n\
         end endmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_hcps_{}_{n}", std::process::id()));
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
        "staged handle copy dropped (handle_copy_stmts sidecar)"
    );
}
