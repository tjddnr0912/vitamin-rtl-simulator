//! Operational infrastructure (ROADMAP §2-7): `--dump-filelist`, filelist
//! canonical-source dedup (the E8003 family's silent-dedup arm), and the
//! RULE-V composite_input_hash recording at vcmp/velab.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ops_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .output()
        .expect("failed to run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
fn dump_filelist_prints_effective_inputs_and_exits() {
    let d = tdir();
    let a = d.join("a.sv");
    let b = d.join("b.sv");
    std::fs::write(&a, "module a; endmodule\n").unwrap();
    std::fs::write(&b, "module b; endmodule\n").unwrap();
    let f = d.join("build.f");
    std::fs::write(
        &f,
        format!(
            "-D W=8\n+incdir+{}\n{}\n{}\n",
            d.display(),
            a.display(),
            b.display()
        ),
    )
    .unwrap();
    let (out, err, ok) = vita(&["-f", f.to_str().unwrap(), "--dump-filelist"]);
    assert!(ok, "dump must succeed; stderr:\n{err}");
    let expect = format!(
        "source {}\nsource {}\ndefine W=8\nincdir {}\n",
        a.display(),
        b.display(),
        d.display()
    );
    assert_eq!(out, expect, "deterministic effective-input dump");
}

#[test]
fn filelist_duplicate_source_dedups_to_first() {
    // The same canonical file twice in one expansion would double-compile the
    // module (a confusing downstream E-DUP-UNIT) — the expansion dedups it
    // (doc-15 E8003 silent-dedup arm; the CONFLICT arm waits on sticky ctx).
    let d = tdir();
    let a = d.join("dup.sv");
    std::fs::write(
        &a,
        "module t; initial begin $display(\"once\"); $finish; end endmodule\n",
    )
    .unwrap();
    let f = d.join("dup.f");
    std::fs::write(&f, format!("{}\n{}\n", a.display(), a.display())).unwrap();
    let (out, err, ok) = vita(&["-f", f.to_str().unwrap()]);
    assert!(ok, "dedup'd run must succeed; stderr:\n{err}");
    assert!(
        out.lines().filter(|l| *l == "once").count() == 1,
        "module must run exactly once; got:\n{out}"
    );
    assert!(
        !err.contains("E-DUP-UNIT") && !err.contains("declared"),
        "no duplicate-unit error after dedup; got:\n{err}"
    );
}

#[test]
fn composite_input_hash_is_recorded_in_vu_and_velab() {
    let d = tdir();
    let src = d.join("c.sv");
    std::fs::write(
        &src,
        "module t; initial begin $display(\"hi\"); $finish; end endmodule\n",
    )
    .unwrap();
    let vu = d.join("c.vu");
    let velab = d.join("c.velab");
    let (_, err, ok) = vita(&["vcmp", src.to_str().unwrap(), "-o", vu.to_str().unwrap()]);
    assert!(ok, "vcmp: {err}");
    let (_, err, ok) = vita(&["velab", vu.to_str().unwrap(), "-o", velab.to_str().unwrap()]);
    assert!(ok, "velab: {err}");

    // .vu records a non-zero digest of its raw input surface.
    let vu_bytes = std::fs::read(&vu).unwrap();
    let (vu_header, _) = vita_artifact::read_vu(&vu_bytes).unwrap();
    assert_ne!(
        vu_header.composite_input_hash, [0u8; 32],
        ".vu composite must be recorded"
    );

    // .velab records EXACTLY the blake3 of the .vu bytes it consumed.
    let velab_bytes = std::fs::read(&velab).unwrap();
    let (ve_header, _) = vita_artifact::read_velab(&velab_bytes).unwrap();
    assert_eq!(
        ve_header.composite_input_hash,
        *blake3::hash(&vu_bytes).as_bytes(),
        ".velab composite = blake3(consumed .vu)"
    );
}

// ── v6 ⑤: the two reserved diagnostic arms ──

#[test]
fn duplicate_source_under_differing_timescale_conflicts() {
    // doc-15 E8003 CONFLICT arm: the SAME canonical file twice, but the two
    // occurrences would inherit DIFFERENT sticky `timescale contexts (RULE S)
    // — silent dedup would drop a semantically distinct compilation → loud.
    let d = tdir();
    let ts1 = d.join("ts1.sv");
    let ts2 = d.join("ts2.sv");
    let shared = d.join("shared.sv");
    std::fs::write(&ts1, "`timescale 1ns/1ns\nmodule a; endmodule\n").unwrap();
    std::fs::write(&ts2, "`timescale 1ps/1ps\nmodule b; endmodule\n").unwrap();
    std::fs::write(
        &shared,
        "module t; initial begin $display(\"x\"); $finish; end endmodule\n",
    )
    .unwrap();
    let f = d.join("dupctx.f");
    std::fs::write(
        &f,
        format!(
            "{}\n{}\n{}\n{}\n",
            ts1.display(),
            shared.display(),
            ts2.display(),
            shared.display()
        ),
    )
    .unwrap();
    let (_, err, ok) = vita(&["-f", f.to_str().unwrap()]);
    assert!(!ok, "differing-context duplicate must be loud");
    assert!(
        err.contains("VITA-E8003"),
        "E-FLIST-DUP-CTX-CONFLICT expected; got:\n{err}"
    );

    // SAME inherited context (both occurrences after ts1) → silent dedup, runs once.
    let f2 = d.join("dupsame.f");
    std::fs::write(
        &f2,
        format!(
            "{}\n{}\n{}\n",
            ts1.display(),
            shared.display(),
            shared.display()
        ),
    )
    .unwrap();
    let (out, err, ok) = vita(&["-f", f2.to_str().unwrap()]);
    assert!(ok, "same-context duplicate dedups silently; stderr:\n{err}");
    assert_eq!(out.lines().filter(|l| *l == "x").count(), 1);
}

#[test]
fn vrun_upstream_staleness_gate() {
    // doc-15 E9003 (RULE V): `vrun --upstream <.vu>` re-hashes the live .vu
    // and compares against the digest the .velab recorded at build time —
    // a mismatch refuses to run (exit class 2), a match runs normally.
    let d = tdir();
    let src = d.join("u.sv");
    std::fs::write(
        &src,
        "module t; initial begin $display(\"ok\"); $finish; end endmodule\n",
    )
    .unwrap();
    let vu = d.join("u.vu");
    let velab = d.join("u.velab");
    let (_, err, ok) = vita(&["vcmp", src.to_str().unwrap(), "-o", vu.to_str().unwrap()]);
    assert!(ok, "vcmp: {err}");
    let (_, err, ok) = vita(&["velab", vu.to_str().unwrap(), "-o", velab.to_str().unwrap()]);
    assert!(ok, "velab: {err}");

    // fresh upstream → runs
    let (out, err, ok) = vita(&[
        "vrun",
        velab.to_str().unwrap(),
        "--upstream",
        vu.to_str().unwrap(),
    ]);
    assert!(ok, "fresh upstream must run; stderr:\n{err}");
    assert!(out.starts_with("ok\n"), "got:\n{out}");

    // edit the source + rebuild ONLY the .vu → the .velab snapshot is stale
    std::fs::write(
        &src,
        "module t; initial begin $display(\"edited\"); $finish; end endmodule\n",
    )
    .unwrap();
    let (_, err, ok) = vita(&["vcmp", src.to_str().unwrap(), "-o", vu.to_str().unwrap()]);
    assert!(ok, "vcmp 2: {err}");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args([
            "vrun",
            velab.to_str().unwrap(),
            "--upstream",
            vu.to_str().unwrap(),
        ])
        .output()
        .expect("run vita");
    assert_eq!(out.status.code(), Some(2), "stale = artifact exit class 2");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("VITA-E9003"),
        "E-ART-STALE-UPSTREAM expected; got:\n{err}"
    );
}
