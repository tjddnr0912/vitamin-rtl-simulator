use super::*;
use std::io::Read;

// TRAILER-PIN (ROADMAP §5.3, 2026-06-23): the `.velab` trailers ride OUTSIDE
// the SchemaHash-pinned SimIr frame, so a silent shape edit to one (e.g. a new
// field on the hand-maintained `StagedExtraSidecars`) that forgets a
// format_version bump makes old artifacts decode wrong. Pin the postcard wire
// shape of a populated `StagedExtraSidecars` fixture: any field add / remove /
// reorder / type change flips the hash. (The other trailers are plain
// sim-engine/sim-ir types under their own coverage; this is the cli-local one
// the STAGED-DROP audit flagged as the fragile, hand-maintained trailer.)
#[test]
fn staged_extra_sidecars_wire_shape_is_pinned() {
    let mut s = StagedExtraSidecars::default();
    s.two_state_nets.insert(7);
    s.class_handle_nets.insert(2);
    s.class_new_sites.insert(3, 9);
    s.class_layouts = vec![vec![(1, true, false)], vec![(2, false, true)]];
    s.class_field_inits = vec![vec![
        None,
        Some(sim_ir::BitPacked {
            val: vec![5],
            unk: vec![0],
        }),
    ]];
    s.class_vtable = vec![vec![10, 11]];
    s.class_calls.insert(5, (Some(2), 11));
    s.class_field_widths.insert(8, (16, true));
    s.assert_fire.insert(6);
    s.assert_ctl.insert(4, 2);
    s.class_rand = vec![
        vec![(0, 32, true, 1, 6, true)],
        vec![(1, 64, false, 0, 0, false)],
    ];
    s.class_constraints = vec![
        vec![vec![
            sim_ir::COp::Field(0),
            sim_ir::COp::Field(1),
            sim_ir::COp::Bin(sim_ir::CBinOp::Lt),
        ]],
        vec![vec![sim_ir::COp::Const(7), sim_ir::COp::Not]],
    ];
    s.class_dist = vec![
        vec![(0, vec![(1, 1, 10), (2, 5, 20)])],
        vec![(3, vec![(0, 0, 1)])],
    ];
    s.class_randc = vec![vec![(2, 0, 15)], vec![(4, -8, 7)]];
    s.randomize_with = vec![
        (
            vec![(0, 1, 9), (1, -3, 3)],
            vec![vec![
                sim_ir::COp::Field(0),
                sim_ir::COp::Field(1),
                sim_ir::COp::Bin(sim_ir::CBinOp::Lt),
            ]],
        ),
        (
            vec![],
            vec![vec![sim_ir::COp::SoftMarker, sim_ir::COp::Field(2)]],
        ),
    ];
    s.clocking_inputs = std::collections::BTreeSet::from([3u32, 7u32]);
    s.clocking_commit =
        std::collections::BTreeMap::from([(5u32, vec![(8u32, 3u32), (9u32, 7u32)])]);
    s.clocking_outputs = std::collections::BTreeMap::from([(6u32, vec![(4u32, 9u32)])]);
    s.ca_delays = std::collections::BTreeMap::from([(0u32, (1u32, 2u32, 3u32))]);
    s.wired_and_nets = std::collections::BTreeSet::from([11u32, 13u32]);
    s.wired_or_nets = std::collections::BTreeSet::from([17u32]);
    s.timeformat_stmts = std::collections::BTreeSet::from([19u32]);
    s.handle_copy_stmts = std::collections::BTreeMap::from([(23u32, (2u32, 5u32))]);
    s.queue_slice_stmts = std::collections::BTreeSet::from([29u32]);
    s.func_names = vec!["top.f".to_string(), "top.g".to_string()];
    s.net_decl_ranges = std::collections::BTreeMap::from([(31u32, (3i64, -2i64))]);
    s.file_directed_stmts = std::collections::BTreeSet::from([37u32]);
    s.proc_ties = vec![2u32, 0, 1];
    let bytes = postcard::to_stdvec(&s).expect("postcard encode");
    let got = blake3::hash(&bytes).to_hex().to_string();
    // REGEN_GOLDEN=1 cargo test -p cli staged_extra_sidecars_wire_shape -- --nocapture
    if std::env::var("REGEN_GOLDEN").is_ok() {
        println!("REGEN StagedExtraSidecars wire = {got}");
        return;
    }
    const EXPECTED: &str = "dd5e68aef087951ba9d39d37e7460ba79ee48fff744bbbdba43203804f98f427";
    assert_eq!(
        got, EXPECTED,
        "StagedExtraSidecars wire shape changed — a field was added / removed / \
             reordered / retyped on the hand-maintained trailer.\n\
             If intentional: bump format_version + regen with REGEN_GOLDEN=1."
    );
}

// The `%t` M-saturation fix widened `proc_multipliers` from Vec<u32> to
// Vec<u64> WITHOUT a format_version bump — legal ONLY because postcard
// varint-encodes both identically for every value < 2^32, so an old
// artifact's u32-encoded timescale trailer decodes byte-exactly as u64.
// This pins that load-bearing wire-compat invariant (if postcard ever
// switched to fixed-width ints, this fails and a bump is required).
#[test]
fn timescale_trailer_u32_to_u64_wire_compat() {
    let old: (Vec<u32>, i8) = (vec![1, 1000, 1_000_000_000, u32::MAX], -12);
    let bytes = postcard::to_stdvec(&old).expect("encode u32 trailer");
    let (new, rest): ((Vec<u64>, i8), &[u8]) =
        postcard::take_from_bytes(&bytes).expect("decode as u64 trailer");
    assert!(rest.is_empty());
    assert_eq!(new.1, -12);
    assert_eq!(new.0, vec![1u64, 1000, 1_000_000_000, u32::MAX as u64]);
}

// RULEV-MTIME wire pin: the 15th `WorkStamps` trailer is also a hand-maintained
// cli-local struct riding outside the SimIr frame. A field add / reorder / type
// change to it (or to `FileStamp`) silently mis-decodes old artifacts unless the
// format_version bumps. Pin a populated fixture's postcard shape the same way.
#[test]
fn work_stamps_wire_shape_is_pinned() {
    let s = WorkStamps {
        libs: vec![Some((1_700_000_000, 123, 4096))],
        blobs: vec![None, Some((1_700_000_001, 0, 64))],
        files: vec![Some((1_700_000_002, 999_999_999, 1))],
    };
    let bytes = postcard::to_stdvec(&s).expect("postcard encode");
    let got = blake3::hash(&bytes).to_hex().to_string();
    // REGEN_GOLDEN=1 cargo test -p cli work_stamps_wire_shape -- --nocapture
    if std::env::var("REGEN_GOLDEN").is_ok() {
        println!("REGEN WorkStamps wire = {got}");
        return;
    }
    const EXPECTED: &str = "923a29a56aa0974671e5453f2e9bab0dcd96e57662cd63aed6c858113e6efb38";
    assert_eq!(
        got, EXPECTED,
        "WorkStamps wire shape changed — a 15th-trailer field moved.\n\
             If intentional: bump format_version + regen with REGEN_GOLDEN=1."
    );
}

/// Run `run_vita` against an on-disk temp file holding `src`; return the exit
/// code. The temp path is unique per call so tests stay parallel-safe.
fn run_on_temp(src: &str, opts: &VitaOpts) -> (i32, String) {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!("vita_cli_test_{pid}_{nonce}.sv"));
    std::fs::write(&path, src).unwrap();
    let code = run_vita(&[path.to_string_lossy().into_owned()], opts);
    let p = path.to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&path);
    (code, p)
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const CLEAN_TB: &str =
    "module tb; reg a; initial begin a=1; $display(\"a=%b\",a); #5 $finish; end endmodule";

#[test]
fn clean_testbench_exits_zero_and_prints() {
    // The capture API proves the $display text; the exit code proves the flow.
    let (toks, lex_errs) = hdl_lexer::lex(CLEAN_TB);
    assert!(lex_errs.is_empty(), "lex: {lex_errs:?}");
    let (unit, perrs) = hdl_parser::parse(&toks, CLEAN_TB);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let sink = StderrSink::new();
    let ir = elaborate::elaborate(&unit.unwrap(), &sink).expect("elaborate");
    let (result, stdout) = sim_engine::simulate_capture(&ir, SimOpts::default());
    assert!(stdout.contains("a=1"), "stdout was: {stdout:?}");
    assert_eq!(sim_exit_code(&result), EXIT_OK);

    // And the full run_vita path returns 0.
    let (code, _) = run_on_temp(CLEAN_TB, &VitaOpts::default());
    assert_eq!(code, EXIT_OK);
}

#[test]
fn parse_error_exits_one() {
    // A `$display` with a missing `)` / `;` — guaranteed parse error.
    let bad = "module m; initial $display(\"x\" ; endmodule";
    let (code, _) = run_on_temp(bad, &VitaOpts::default());
    assert_eq!(code, EXIT_USER_ERROR);
}

#[test]
fn lex_error_exits_one() {
    let bad = "module m; reg a; initial a = \"unterminated; endmodule";
    let (code, _) = run_on_temp(bad, &VitaOpts::default());
    assert_eq!(code, EXIT_USER_ERROR);
}

#[test]
fn no_source_files_exits_three() {
    assert_eq!(run_vita(&[], &VitaOpts::default()), EXIT_CLI_ERROR);
}

#[test]
fn missing_file_exits_three() {
    let missing = "/nonexistent/path/that/does/not/exist_vita.sv".to_string();
    assert_eq!(run_vita(&[missing], &VitaOpts::default()), EXIT_CLI_ERROR);
}

#[test]
fn vcmp_missing_source_via_run_exits_three() {
    // `vcmp` is now real: `vcmp <missing>.sv` routes to dispatch_vcmp, which
    // fails on the missing-file READ path → exit 3 (CLI/usage error, not a
    // stub). The path is deliberately one that cannot exist.
    let missing = "/nonexistent/path/unknown_applet_top.sv".to_string();
    let argv = vec!["/usr/local/bin/vcmp".to_string(), missing.clone()];
    assert_eq!(run(&argv), EXIT_CLI_ERROR);
    // explicit `vita vcmp …` form routes the same way.
    let argv = vec!["vita".to_string(), "vcmp".to_string(), missing];
    assert_eq!(run(&argv), EXIT_CLI_ERROR);
}

#[test]
fn unknown_flag_to_staged_applet_exits_three() {
    // A genuinely-unknown flag to a staged applet is a CLI/usage error (exit 3)
    // — proves the arg parser rejects, not the stub.
    let argv = vec![
        "/usr/local/bin/vcmp".to_string(),
        "--bogus-flag".to_string(),
        "x.sv".to_string(),
    ];
    assert_eq!(run(&argv), EXIT_CLI_ERROR);
}

#[test]
fn vita_basename_resolves_to_one_shot() {
    let argv = vec!["/usr/local/bin/vita".to_string(), "x.sv".to_string()];
    let (applet, rest) = resolve_applet(&argv);
    assert_eq!(applet, Applet::Vita);
    assert_eq!(rest, vec!["x.sv".to_string()]);
}

#[test]
fn dumpvars_writes_vcd_with_enddefinitions() {
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let nonce = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let vcd = dir.join(format!("vita_cli_test_{pid}_{nonce}.vcd"));
    let vcd_str = vcd.to_string_lossy().into_owned();
    let src = format!(
            "module tb; reg a; initial begin $dumpfile(\"{}\"); $dumpvars(0, tb); a=1; #5 $finish; end endmodule",
            vcd_str.replace('\\', "\\\\")
        );
    let opts = VitaOpts {
        vcd_path_override: Some(vcd_str.clone()),
        ..VitaOpts::default()
    };
    let (code, _) = run_on_temp(&src, &opts);
    assert_eq!(code, EXIT_OK);
    assert!(vcd.exists(), "VCD not written at {vcd_str}");
    let mut contents = String::new();
    std::fs::File::open(&vcd)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert!(
        contents.contains("$enddefinitions"),
        "VCD missing $enddefinitions:\n{contents}"
    );
    let _ = std::fs::remove_file(&vcd);
}
