//! The STAGED half of `-G`/`--param` (doc-14 RULE B) — split out of
//! `implicit_param_ports.rs` when that file crossed the repo's 1000-line policy.
//!
//! `velab -G` parsed the flag, reported `errors=0` and elaborated the DECLARED
//! DEFAULTS; `vcmp`/`vrun` accepted it and dropped it. The override is an
//! elaborate-stage input, so it applies in `velab` (plain and `-L` library mode) and is
//! loud on the other two. These drive **argv** through `cli::run`, not the library entry
//! points: the flag was dropped at the argv->opts boundary in `dispatch_velab`, so a
//! test that hands `VitaOpts` straight to `run_velab` cannot see the bug it guards
//! (measured -- that test passes with the fix reverted).

use std::sync::atomic::{AtomicU64, Ordering};

/// Per-test unique suffix. Each integration-test FILE is its own binary, so this
/// counter is independent of the one in `implicit_param_ports.rs` — the temp-dir names
/// also carry the pid, which is what actually keeps two binaries from colliding.
static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_ippstg_{}_{n}.{ext}", std::process::id()))
}
fn s(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}
fn opts(g: &[(&str, &str)]) -> cli::VitaOpts {
    cli::VitaOpts {
        top_params: g
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect(),
        ..cli::VitaOpts::default()
    }
}

/// Elaborate `src` through vcmp → velab (with `-G g`), then RUN the artifact and
/// return its `$display` transcript. Reading the transcript is what makes this a
/// gate: `velab` reported `errors=0` while producing an artifact built from the
/// declared defaults, so an exit-code assertion sees nothing.
fn staged_transcript(src: &str, g: &[(&str, &str)]) -> Result<String, i32> {
    let f = tmp("sv");
    let vu = tmp("vu");
    let velab = tmp("velab");
    std::fs::write(&f, src).unwrap();
    assert_eq!(
        cli::run_vcmp(&[s(&f)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    let code = cli::run_velab(&s(&vu), &s(&velab), &opts(g));
    if code != cli::EXIT_OK {
        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_file(&vu);
        return Err(code);
    }
    let out = transcript_of(&velab);
    let _ = std::fs::remove_file(&f);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
    Ok(out)
}

/// Run an already-written `.velab` and return its `$display` transcript. Reading
/// the transcript is what gives these tests teeth: `velab` reported `errors=0`
/// while writing an artifact built from the declared defaults, so an exit-code
/// assertion sees nothing.
fn transcript_of(velab: &std::path::Path) -> String {
    let bytes = std::fs::read(velab).unwrap();
    let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
    let (ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
    let (_res, out) = sim_engine::simulate_capture(
        &ir,
        sim_engine::SimOpts {
            fork_modes: modes,
            ..Default::default()
        },
    );
    out
}

const BODY: &str =
    "module tb; parameter CYCLES = 5; initial $display(\"CYCLES=%0d\", CYCLES); endmodule";
const ANSI: &str = "module tb #(parameter CYCLES = 5); initial $display(\"CYCLES=%0d\", \
                    CYCLES); endmodule";

/// `velab -G` applies — for the body spelling AND the ANSI one. Both printed
/// `CYCLES=5` (the declared default) at `errors=0` before this.
#[test]
fn velab_applies_the_override_to_both_spellings() {
    assert!(staged_transcript(BODY, &[("CYCLES", "7")])
        .unwrap()
        .contains("CYCLES=7"));
    assert!(staged_transcript(ANSI, &[("CYCLES", "7")])
        .unwrap()
        .contains("CYCLES=7"));
    // and with no `-G` the default still stands (the branch is not unconditional)
    assert!(staged_transcript(BODY, &[]).unwrap().contains("CYCLES=5"));
}

/// A `-G` naming nothing is loud in `velab` too — the one-shot path already was,
/// and a stage that silently ignores a typo'd override is the same wrong design.
#[test]
fn velab_is_loud_about_an_unmatched_override() {
    assert!(staged_transcript(BODY, &[("NOPE", "7")]).is_err());
}

/// The velab composite digest is the RULE-V UPSTREAM digest — `blake3` of the
/// `.vu` bytes and nothing else. Mixing the overrides into it (the first cut of
/// this slice did, reading doc-14 RULE B as if it named this field) made
/// `vrun --upstream` re-hash the live `.vu`, get a different answer, and report
/// `E9003: digest changed since the .velab snapshot` about a file that had not
/// changed — so the whole staged `-G` flow failed the gate it is supposed to pass.
#[test]
fn a_g_built_artifact_still_passes_the_upstream_gate() {
    let f = tmp("sv");
    let vu = tmp("vu");
    let velab = tmp("velab");
    std::fs::write(&f, BODY).unwrap();
    assert_eq!(
        cli::run_vcmp(&[s(&f)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &opts(&[("CYCLES", "7")])),
        cli::EXIT_OK
    );
    // the digest a `-G` build records must still be reproducible from the `.vu`
    let bytes = std::fs::read(&velab).unwrap();
    let (h, _b) = vita_artifact::read_velab(&bytes).unwrap();
    let vu_bytes = std::fs::read(&vu).unwrap();
    assert_eq!(
        h.composite_input_hash,
        *blake3::hash(&vu_bytes).as_bytes(),
        "the composite is the upstream digest; an override must not enter it"
    );
    // and the gate that re-hashes it must pass
    assert_eq!(
        cli::run_vrun(
            &s(&velab),
            &cli::VitaOpts {
                upstream: Some(s(&vu)),
                ..cli::VitaOpts::default()
            }
        ),
        cli::EXIT_OK
    );
    let _ = std::fs::remove_file(&f);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
}

// ── argv-level: the boundary the bug actually lived at ──────────────────

/// `velab -G` through ARGV. The three tests above hand a `VitaOpts` straight to
/// `run_velab`, which skips `dispatch_velab` — and `dispatch_velab` dropping
/// `io.top_params` on the floor IS the bug. Measured: with the one-line fix
/// reverted, every library-level test still passes and only this one fails.
#[test]
fn velab_argv_carries_the_override_through_dispatch() {
    let f = tmp("sv");
    let vu = tmp("vu");
    let velab = tmp("velab");
    std::fs::write(&f, BODY).unwrap();
    assert_eq!(
        cli::run(&["vcmp".into(), s(&f), "-o".into(), s(&vu)]),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run(&[
            "velab".into(),
            s(&vu),
            "-o".into(),
            s(&velab),
            "-G".into(),
            "CYCLES=7".into()
        ]),
        cli::EXIT_OK
    );
    assert!(transcript_of(&velab).contains("CYCLES=7"));
    let _ = std::fs::remove_file(&f);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
}

/// `-G` is an elaborate-stage flag: `vcmp` and `vrun` must refuse it rather than
/// accept-and-drop. Both accepted it silently before.
#[test]
fn vcmp_and_vrun_refuse_an_elaborate_stage_override() {
    let f = tmp("sv");
    let vu = tmp("vu");
    let velab = tmp("velab");
    std::fs::write(&f, BODY).unwrap();
    assert_eq!(
        cli::run(&["vcmp".into(), s(&f), "-o".into(), s(&vu)]),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run(&["velab".into(), s(&vu), "-o".into(), s(&velab)]),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run(&[
            "vcmp".into(),
            s(&f),
            "-o".into(),
            s(&vu),
            "-G".into(),
            "CYCLES=7".into()
        ]),
        cli::EXIT_CLI_ERROR
    );
    assert_eq!(
        cli::run(&["vrun".into(), s(&velab), "-G".into(), "CYCLES=7".into()]),
        cli::EXIT_CLI_ERROR
    );
    let _ = std::fs::remove_file(&f);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
}

/// The `-L` worklib compose path is a SECOND `velab` entry point with its own
/// elaborate call; it had no `-G` coverage at all (measured: passing `&[]` there
/// instead of the real overrides passed the whole suite).
#[test]
fn velab_library_mode_carries_the_override() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_ipplib_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.sv");
    std::fs::write(&f, BODY).unwrap();
    let libdir = dir.join("mylib");
    let velab = dir.join("out.velab");
    assert_eq!(
        cli::run(&[
            "vcmp".into(),
            f.to_string_lossy().into_owned(),
            "--work".into(),
            format!("mylib={}", libdir.to_string_lossy()),
        ]),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run(&[
            "velab".into(),
            "-L".into(),
            format!("mylib={}", libdir.to_string_lossy()),
            "--top".into(),
            "tb".into(),
            "-o".into(),
            velab.to_string_lossy().into_owned(),
            "-G".into(),
            "CYCLES=7".into(),
        ]),
        cli::EXIT_OK
    );
    assert!(transcript_of(&velab).contains("CYCLES=7"));
    let _ = std::fs::remove_dir_all(&dir);
}
