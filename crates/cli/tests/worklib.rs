//! P2-A worklib (doc-14 §1/§3 v1 subset): `vcmp --work` emits a canonical
//! `lib.toml` manifest + content-addressed CU blob under `units/`; `velab
//! -L <lib> --top <unit>` discovers units by logical name and elaborates the
//! instantiation closure; `vrun` auto-verifies upstream freshness from the
//! recorded WorkConsumed trailer (RULE V — manifest hash, blob hash, and
//! raw source/include digests; any mismatch = `VITA-E9003`, exit class 2).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tdir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_work_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run the multicall binary; returns (stdout, stderr, exit code).
fn vita(args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .output()
        .expect("failed to run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn write(p: &std::path::Path, s: &str) {
    std::fs::write(p, s).unwrap();
}

const HELLO: &str = "module top; initial begin $display(\"hello\"); $finish; end endmodule\n";

#[test]
fn vcmp_work_emits_manifest_and_blob_deterministically() {
    let d = tdir();
    let src = d.join("top.sv");
    write(&src, HELLO);
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    let (_, err, code) = vita(&["vcmp", src.to_str().unwrap(), "--work", &work]);
    assert_eq!(code, Some(0), "vcmp --work must succeed; stderr:\n{err}");
    let manifest = std::fs::read(lib.join("lib.toml")).expect("lib.toml written");
    let text = String::from_utf8(manifest.clone()).unwrap();
    assert!(
        text.contains("name = \"work\""),
        "logical name recorded:\n{text}"
    );
    assert!(text.contains("module top"), "unit listed:\n{text}");
    assert!(text.contains("units/cu_"), "blob path recorded:\n{text}");
    // The blob it names must exist.
    let blob_rel = text
        .lines()
        .find_map(|l| l.split('"').nth(1).filter(|s| s.starts_with("units/cu_")))
        .expect("blob line");
    assert!(lib.join(blob_rel).is_file(), "blob file exists");
    // Re-running the same compile is idempotent: byte-identical manifest.
    let (_, err, code) = vita(&["vcmp", src.to_str().unwrap(), "--work", &work]);
    assert_eq!(code, Some(0), "re-vcmp: {err}");
    let manifest2 = std::fs::read(lib.join("lib.toml")).unwrap();
    assert_eq!(manifest, manifest2, "canonical manifest is deterministic");
}

#[test]
fn velab_and_vrun_from_worklib_match_direct_pipeline() {
    let d = tdir();
    let src = d.join("t.sv");
    write(
        &src,
        "module top;\n\
         reg clk; reg [3:0] q;\n\
         always @(posedge clk) q <= q + 4'd1;\n\
         initial begin\n\
           $dumpfile(\"x.vcd\"); $dumpvars;\n\
           clk = 0; q = 0;\n\
           repeat (4) begin #1 clk = 1; #1 clk = 0; end\n\
           $display(\"q=%0d\", q); $finish;\n\
         end\n\
         endmodule\n",
    );
    // Legacy explicit-path pipeline.
    let vu = d.join("t.vu");
    let ve_a = d.join("a.velab");
    let vcd_a = d.join("a.vcd");
    let (_, err, code) = vita(&["vcmp", src.to_str().unwrap(), "-o", vu.to_str().unwrap()]);
    assert_eq!(code, Some(0), "vcmp: {err}");
    let (_, err, code) = vita(&["velab", vu.to_str().unwrap(), "-o", ve_a.to_str().unwrap()]);
    assert_eq!(code, Some(0), "velab: {err}");
    let (out_a, err, code) = vita(&[
        "vrun",
        ve_a.to_str().unwrap(),
        "-o",
        vcd_a.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "vrun a: {err}");
    // Worklib pipeline.
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    let ve_b = d.join("b.velab");
    let vcd_b = d.join("b.vcd");
    let (_, err, code) = vita(&["vcmp", src.to_str().unwrap(), "--work", &work]);
    assert_eq!(code, Some(0), "vcmp --work: {err}");
    let larg = format!("work={}", lib.display());
    let (_, err, code) = vita(&[
        "velab",
        "-L",
        &larg,
        "--top",
        "top",
        "-o",
        ve_b.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "velab -L: {err}");
    let (out_b, err, code) = vita(&[
        "vrun",
        ve_b.to_str().unwrap(),
        "-o",
        vcd_b.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "vrun b: {err}");
    assert_eq!(out_a, out_b, "stdout byte-identical across pipelines");
    let a = std::fs::read(&vcd_a).unwrap();
    let b = std::fs::read(&vcd_b).unwrap();
    assert_eq!(a, b, "VCD byte-identical across pipelines");
}

#[test]
fn velab_lib_mode_requires_top_and_unknown_top_fails() {
    let d = tdir();
    let src = d.join("top.sv");
    write(&src, HELLO);
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    let (_, err, code) = vita(&["vcmp", src.to_str().unwrap(), "--work", &work]);
    assert_eq!(code, Some(0), "vcmp: {err}");
    let larg = format!("work={}", lib.display());
    // No --top in library mode: usage error.
    let (_, err, code) = vita(&["velab", "-L", &larg]);
    assert_eq!(
        code,
        Some(3),
        "lib mode without --top is a usage error:\n{err}"
    );
    assert!(
        err.contains("--top"),
        "message names the missing flag:\n{err}"
    );
    // Unknown --top unit: user error naming the unit.
    let (_, err, code) = vita(&["velab", "-L", &larg, "--top", "nosuch"]);
    assert_ne!(code, Some(0), "unknown top must fail");
    assert!(err.contains("nosuch"), "message names the unit:\n{err}");
}

#[test]
fn vrun_auto_gate_source_edit_manifest_tamper_and_refresh() {
    let d = tdir();
    let src = d.join("top.sv");
    write(&src, HELLO);
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    let larg = work.clone();
    let ve = d.join("t.velab");
    assert_eq!(
        vita(&["vcmp", src.to_str().unwrap(), "--work", &work]).2,
        Some(0)
    );
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &larg,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "fresh worklib runs: {err}");
    assert!(out.starts_with("hello\n"), "got:\n{out}");

    // (a) Source edited WITHOUT recompiling: raw-digest check catches it.
    write(
        &src,
        "module top; initial begin $display(\"EDITED\"); $finish; end endmodule\n",
    );
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(2),
        "stale source = artifact exit class 2:\n{err}"
    );
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");

    // (b) Recompiled into the lib but velab NOT re-run: manifest hash moved.
    assert_eq!(
        vita(&["vcmp", src.to_str().unwrap(), "--work", &work]).2,
        Some(0)
    );
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(2), "stale manifest = exit class 2:\n{err}");
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");

    // (c) Re-velab: fresh again, new behavior visible.
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &larg,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "refreshed runs: {err}");
    assert!(out.starts_with("EDITED\n"), "got:\n{out}");

    // (d) Manifest tampered after velab: hash mismatch without parsing.
    let mpath = lib.join("lib.toml");
    let mut m = std::fs::read(&mpath).unwrap();
    m.extend_from_slice(b"# tamper\n");
    std::fs::write(&mpath, &m).unwrap();
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(2), "tampered manifest = exit class 2:\n{err}");
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");
}

#[test]
fn vrun_gate_catches_include_edit() {
    let d = tdir();
    let inc = d.join("h.vh");
    write(&inc, "`define MSG \"v1\"\n");
    let src = d.join("top.sv");
    write(
        &src,
        "`include \"h.vh\"\nmodule top; initial begin $display(`MSG); $finish; end endmodule\n",
    );
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    let ve = d.join("t.velab");
    assert_eq!(
        vita(&[
            "vcmp",
            src.to_str().unwrap(),
            "-I",
            d.to_str().unwrap(),
            "--work",
            &work
        ])
        .2,
        Some(0)
    );
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &work,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, _, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("v1\n"), "got:\n{out}");
    // Editing ONLY the include file must trip the gate (closure is recorded).
    write(&inc, "`define MSG \"v2\"\n");
    let (_, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(2), "stale include = exit class 2:\n{err}");
    assert!(err.contains("VITA-E9003"), "E9003 expected:\n{err}");
}

#[test]
fn cross_lib_resolution_first_l_wins() {
    let d = tdir();
    let top = d.join("top.sv");
    write(
        &top,
        "module top; leaf u(); initial begin #1 $finish; end endmodule\n",
    );
    let leaf_a = d.join("leaf_a.sv");
    write(&leaf_a, "module leaf; initial $display(\"A\"); endmodule\n");
    let leaf_b = d.join("leaf_b.sv");
    write(&leaf_b, "module leaf; initial $display(\"B\"); endmodule\n");
    let liba = format!("liba={}", d.join("la").display());
    let libb = format!("libb={}", d.join("lb").display());
    assert_eq!(
        vita(&["vcmp", top.to_str().unwrap(), "--work", &liba]).2,
        Some(0)
    );
    assert_eq!(
        vita(&["vcmp", leaf_a.to_str().unwrap(), "--work", &liba]).2,
        Some(0)
    );
    assert_eq!(
        vita(&["vcmp", leaf_b.to_str().unwrap(), "--work", &libb]).2,
        Some(0)
    );
    let ve = d.join("t.velab");
    // liba first → its leaf shadows libb's.
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &liba,
            "-L",
            &libb,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, _, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("A\n"), "first -L wins; got:\n{out}");
    // libb first → its leaf wins; top still resolves from liba.
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &libb,
            "-L",
            &liba,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, _, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(
        out.starts_with("B\n"),
        "search order is the -L order; got:\n{out}"
    );
}

#[test]
fn dup_unit_same_lib_rejected_and_recompile_replaces() {
    let d = tdir();
    let f1 = d.join("x1.sv");
    write(
        &f1,
        "module x; initial begin $display(\"v1\"); $finish; end endmodule\n",
    );
    let f2 = d.join("x2.sv");
    write(
        &f2,
        "module x; initial begin $display(\"other\"); $finish; end endmodule\n",
    );
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    assert_eq!(
        vita(&["vcmp", f1.to_str().unwrap(), "--work", &work]).2,
        Some(0)
    );
    let before = std::fs::read(lib.join("lib.toml")).unwrap();
    // A DIFFERENT source defining the same unit name: E-DUP-UNIT, no mutation.
    let (_, err, code) = vita(&["vcmp", f2.to_str().unwrap(), "--work", &work]);
    assert_eq!(code, Some(1), "dup unit is a user error:\n{err}");
    assert!(err.contains("VITA-E2001"), "E-DUP-UNIT expected:\n{err}");
    let after = std::fs::read(lib.join("lib.toml")).unwrap();
    assert_eq!(before, after, "rejected vcmp must not mutate the manifest");
    // Recompiling the SAME source path replaces its units (incremental flow).
    write(
        &f1,
        "module x; initial begin $display(\"v3\"); $finish; end endmodule\n",
    );
    assert_eq!(
        vita(&["vcmp", f1.to_str().unwrap(), "--work", &work]).2,
        Some(0)
    );
    let ve = d.join("x.velab");
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &work,
            "--top",
            "x",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, _, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert!(out.starts_with("v3\n"), "replacement visible; got:\n{out}");
}

#[test]
fn closure_loads_transitive_units_and_skips_unrelated() {
    let d = tdir();
    let top = d.join("top.sv");
    write(
        &top,
        "module top; mid m(); initial begin #1 $finish; end endmodule\n",
    );
    let mid = d.join("mid.sv");
    write(&mid, "module mid; leaf l(); endmodule\n");
    let leaf = d.join("leaf.sv");
    write(
        &leaf,
        "module leaf; initial $display(\"leaf-here\"); endmodule\n",
    );
    let junk = d.join("junk.sv");
    write(
        &junk,
        "module junk; initial $display(\"junk-must-not-print\"); endmodule\n",
    );
    let lib = d.join("w");
    let work = format!("work={}", lib.display());
    for f in [&top, &mid, &leaf, &junk] {
        assert_eq!(
            vita(&["vcmp", f.to_str().unwrap(), "--work", &work]).2,
            Some(0)
        );
    }
    let ve = d.join("t.velab");
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &work,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "vrun: {err}");
    assert!(
        out.contains("leaf-here"),
        "transitive unit elaborated; got:\n{out}"
    );
    assert!(
        !out.contains("junk-must-not-print"),
        "units outside the closure must not become roots; got:\n{out}"
    );
}

#[test]
fn top_overrides_roots_in_legacy_positional_mode() {
    let d = tdir();
    let src = d.join("two.sv");
    write(
        &src,
        "module a; initial begin $display(\"a\"); $finish; end endmodule\n\
         module b; initial begin $display(\"b\"); $finish; end endmodule\n",
    );
    let vu = d.join("two.vu");
    let ve = d.join("two.velab");
    assert_eq!(
        vita(&["vcmp", src.to_str().unwrap(), "-o", vu.to_str().unwrap()]).2,
        Some(0)
    );
    // Without --top BOTH uninstantiated modules are roots; with --top only `a`.
    assert_eq!(
        vita(&[
            "velab",
            vu.to_str().unwrap(),
            "--top",
            "a",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "vrun: {err}");
    assert!(out.contains("a\n"), "selected root runs; got:\n{out}");
    assert!(
        !out.contains("b\n"),
        "unselected module is not a root; got:\n{out}"
    );
}

#[test]
fn malformed_manifest_is_loud_e9005() {
    let d = tdir();
    let lib = d.join("w");
    std::fs::create_dir_all(&lib).unwrap();
    write(&lib.join("lib.toml"), "this is not a manifest\n");
    let larg = format!("work={}", lib.display());
    let (_, err, code) = vita(&["velab", "-L", &larg, "--top", "x"]);
    assert_eq!(
        code,
        Some(2),
        "invalid manifest = artifact exit class 2:\n{err}"
    );
    assert!(
        err.contains("VITA-E9005"),
        "E-WORK-MANIFEST expected:\n{err}"
    );
}

#[test]
fn passenger_definition_does_not_beat_search_order() {
    // `util` is defined in BOTH libs. liba is first, so its `util` must win —
    // even though libb's CU (loaded for `leaf`) carries a `util` definition as
    // a passenger and happens to load EARLIER in the closure walk.
    let d = tdir();
    let top = d.join("top.sv");
    write(
        &top,
        "module top; leaf l(); util u(); initial begin #1 $finish; end endmodule\n",
    );
    let pair = d.join("pair.sv");
    write(
        &pair,
        "module leaf; initial $display(\"leaf-B\"); endmodule\n\
         module util; initial $display(\"UTIL-B\"); endmodule\n",
    );
    let util_a = d.join("util_a.sv");
    write(
        &util_a,
        "module util; initial $display(\"UTIL-A\"); endmodule\n",
    );
    let liba = format!("liba={}", d.join("la").display());
    let libb = format!("libb={}", d.join("lb").display());
    assert_eq!(
        vita(&["vcmp", top.to_str().unwrap(), "--work", &liba]).2,
        Some(0)
    );
    assert_eq!(
        vita(&["vcmp", util_a.to_str().unwrap(), "--work", &liba]).2,
        Some(0)
    );
    assert_eq!(
        vita(&["vcmp", pair.to_str().unwrap(), "--work", &libb]).2,
        Some(0)
    );
    let ve = d.join("t.velab");
    assert_eq!(
        vita(&[
            "velab",
            "-L",
            &liba,
            "-L",
            &libb,
            "--top",
            "top",
            "-o",
            ve.to_str().unwrap()
        ])
        .2,
        Some(0)
    );
    let (out, err, code) = vita(&["vrun", ve.to_str().unwrap()]);
    assert_eq!(code, Some(0), "vrun: {err}");
    assert!(
        out.contains("UTIL-A"),
        "first -L wins for util; got:\n{out}"
    );
    assert!(
        !out.contains("UTIL-B"),
        "passenger must not shadow; got:\n{out}"
    );
    assert!(
        out.contains("leaf-B"),
        "leaf resolves from libb; got:\n{out}"
    );
}
