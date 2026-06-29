//! Staged-flow integration tests (vcmp → velab → vrun). The CLI lib is the SUT.
//! Temp files use a per-call nonce for parallel safety.
// `VitaOpts { .., ..Default::default() }` is written verbatim from the spec as a
// forward-compat hedge; with a single field today clippy's needless-update fires.
#![allow(clippy::needless_update)]
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_staged_{}_{n}.{ext}", std::process::id()))
}
fn write(p: &std::path::Path, s: &str) {
    std::fs::write(p, s).unwrap();
}
fn s(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

const CLEAN_TB: &str =
    "module tb; reg a; initial begin a=1; $display(\"a=%b\",a); #5 $finish; end endmodule";

/// Local copy of the §1.4 `.vu` header builder (the production one is private).
fn forge_vu_header(schema: [u8; 32]) -> vita_artifact::VelabHeader {
    vita_artifact::VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash: schema,
        composite_input_hash: [0u8; 32],
        global_time_precision: 0,
        consumed: Vec::new(),
        worklib_manifest_hash: [0u8; 32],
        uses_dump: false,
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        provenance: vita_artifact::Provenance::capture(),
    }
}

// TEST 3: `.vu` round-trip — vcmp writes a file whose decoded SourceUnit equals
//          the production-frontend SourceUnit.
#[test]
fn vu_roundtrip_sourceunit_byte_equal() {
    let src = tmp("sv");
    write(&src, CLEAN_TB);
    let vu = tmp("vu");
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );

    // decode the `.vu` body back to a SourceUnit
    let bytes = std::fs::read(&vu).unwrap();
    let (_h, body) = vita_artifact::read_vu(&bytes).expect("read_vu");
    let decoded: hdl_ast::SourceUnit = postcard::from_bytes(body).expect("decode SourceUnit");

    // Build the reference through the SAME preprocess→lex→parse path run_vcmp uses.
    let reference =
        cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).expect("reference frontend parse");
    assert_eq!(
        decoded, reference,
        "round-tripped SourceUnit must equal the production frontend's SourceUnit"
    );

    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&vu);
}

// TEST 4: `.velab` round-trip — SimIr AND ForkModeTable survive the body layout.
#[test]
fn velab_roundtrip_simir_and_forktable() {
    let src = tmp("sv");
    write(&src, CLEAN_TB);
    let vu = tmp("vu");
    let velab = tmp("velab");
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );

    let bytes = std::fs::read(&velab).unwrap();
    let (h, body) = vita_artifact::read_velab(&bytes).expect("read_velab");
    assert_eq!(
        h.schema_hash,
        vita_schema::schema_hash::<sim_ir::SimIr>(),
        "header carries SimIr hash"
    );
    // split SimIr frame from trailer exactly as vrun does
    let (_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).expect("SimIr frame");
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).expect("fork trailer");
    assert!(
        modes.is_empty(),
        "fork-free design → empty trailer (one varint byte)"
    );

    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
}

// TEST 5: END-TO-END staged chain produces the SAME $display output as one-shot.
#[test]
fn staged_chain_matches_oneshot_display() {
    let src = tmp("sv");
    write(&src, CLEAN_TB);
    let vu = tmp("vu");
    let velab = tmp("velab");

    // one-shot reference output via simulate_capture — parse through the SAME front-end.
    let ref_unit = cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).unwrap();
    let ref_ir = elaborate::elaborate(&ref_unit, &cli::StderrSink::new()).unwrap();
    let (ref_res, ref_out) = sim_engine::simulate_capture(&ref_ir, sim_engine::SimOpts::default());

    // staged chain
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );

    // vrun via the public path returns the same exit class as one-shot
    let code = cli::run_vrun(&s(&velab), &cli::VitaOpts::default());
    assert_eq!(code, cli::EXIT_OK);

    // and the staged SimIr produces byte-identical $display text as the reference
    let bytes = std::fs::read(&velab).unwrap();
    let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
    let (staged_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
    let (staged_res, staged_out) = sim_engine::simulate_capture(
        &staged_ir,
        sim_engine::SimOpts {
            fork_modes: modes,
            ..Default::default()
        },
    );
    assert_eq!(
        staged_out, ref_out,
        "staged $display transcript must equal one-shot"
    );
    assert!(ref_out.contains("a=1") && staged_out.contains("a=1"));
    assert_eq!(staged_res.exit_class, ref_res.exit_class);

    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&vu);
    let _ = std::fs::remove_file(&velab);
}

// TEST 6: END-TO-END VCD parity — staged path writes a VCD byte-equal to one-shot vita -o.
#[test]
fn staged_chain_matches_oneshot_vcd() {
    let dump = "module tb; reg a; initial begin $dumpfile(\"IGNORED\"); $dumpvars(0,tb); a=1; #5 $finish; end endmodule";
    let src = tmp("sv");
    write(&src, dump);
    let vu = tmp("vu");
    let velab = tmp("velab");
    let vcd_oneshot = tmp("vcd");
    let vcd_staged = tmp("vcd");

    // one-shot
    assert_eq!(
        cli::run_vita(
            &[s(&src)],
            &cli::VitaOpts {
                vcd_path_override: Some(s(&vcd_oneshot)),
                ..Default::default()
            }
        ),
        cli::EXIT_OK
    );
    // staged
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_vrun(
            &s(&velab),
            &cli::VitaOpts {
                vcd_path_override: Some(s(&vcd_staged)),
                ..Default::default()
            }
        ),
        cli::EXIT_OK
    );

    let a = std::fs::read_to_string(&vcd_oneshot).unwrap();
    let b = std::fs::read_to_string(&vcd_staged).unwrap();
    assert!(a.contains("$enddefinitions"));
    assert_eq!(a, b, "staged VCD must be byte-identical to one-shot VCD");

    for p in [&src, &vu, &velab, &vcd_oneshot, &vcd_staged] {
        let _ = std::fs::remove_file(p);
    }
}

// TEST 6b: a `timescale design threads through the staged path identically to the
//          one-shot path — proving `.vu`/`.velab` carry the resolved timescale.
#[test]
fn staged_chain_matches_oneshot_timescaled() {
    let dump = "`timescale 1ns/1ps\nmodule top; reg clk; \
                initial begin $dumpfile(\"IGNORED\"); $dumpvars(0,top); \
                  clk=0; #5 clk=1; #5 $finish; end endmodule\n";
    let src = tmp("sv");
    write(&src, dump);
    let vu = tmp("vu");
    let velab = tmp("velab");
    let vcd_oneshot = tmp("vcd");
    let vcd_staged = tmp("vcd");

    assert_eq!(
        cli::run_vita(
            &[s(&src)],
            &cli::VitaOpts {
                vcd_path_override: Some(s(&vcd_oneshot)),
                ..Default::default()
            }
        ),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_vrun(
            &s(&velab),
            &cli::VitaOpts {
                vcd_path_override: Some(s(&vcd_staged)),
                ..Default::default()
            }
        ),
        cli::EXIT_OK
    );

    let a = std::fs::read_to_string(&vcd_oneshot).unwrap();
    let b = std::fs::read_to_string(&vcd_staged).unwrap();
    // staged must equal one-shot, AND both must reflect the 1ns/1ps scaling:
    // global precision 1ps in the preamble and the clk toggle at 5ns = tick 5000.
    assert_eq!(a, b, "staged timescaled VCD must match one-shot");
    assert!(
        a.contains("$timescale 1ps $end"),
        "preamble precision:\n{a}"
    );
    assert!(a.contains("#5000"), "clk toggle at scaled tick 5000:\n{a}");

    for p in [&src, &vu, &velab, &vcd_oneshot, &vcd_staged] {
        let _ = std::fs::remove_file(p);
    }
}

// TEST 7: a schema-mismatch `.vu` is rejected with E-ART-SCHEMA-MISMATCH.
#[test]
fn velab_rejects_stale_vu_schema_mismatch() {
    // valid SourceUnit body, but header schema_hash deliberately corrupted.
    let (toks, _) = hdl_lexer::lex(CLEAN_TB);
    let (unit, _) = hdl_parser::parse(&toks, CLEAN_TB);
    let body = postcard::to_stdvec(&unit.unwrap()).unwrap();
    let mut h = forge_vu_header(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    h.schema_hash[0] ^= 0xFF; // wrong shape signature
    let bytes = vita_artifact::write_vu(&h, &body);
    let vu = tmp("vu");
    std::fs::write(&vu, &bytes).unwrap();

    // run_velab must reject at the gate → exit 2 (class 2: rebuild upstream).
    let velab = tmp("velab");
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_STALE
    );
    assert!(!velab.exists(), "rejected .vu must not produce a .velab");

    // direct gate assertion proves the exact code.
    let (got, _) = vita_artifact::read_vu(&bytes).unwrap();
    let tool = vita_artifact::ToolContext::new(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    let err = vita_artifact::verify_header(&got, &tool).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtSchemaMismatch);

    let _ = std::fs::remove_file(&vu);
}

// TEST 8: a bad-magic file is rejected with E-ART-FORMAT-MISMATCH (vrun exit 2).
#[test]
fn vrun_rejects_bad_magic() {
    let junk = tmp("velab");
    std::fs::write(&junk, b"NOTVELAB....garbage").unwrap();
    assert_eq!(
        cli::run_vrun(&s(&junk), &cli::VitaOpts::default()),
        cli::EXIT_STALE
    );
    let err = vita_artifact::read_velab(b"NOTVELAB....garbage").unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
    let _ = std::fs::remove_file(&junk);
}

// TEST 9: a `.velab` whose SimIr header hash != vrun expected → E-ART-SCHEMA-MISMATCH.
#[test]
fn vrun_rejects_stale_velab_schema_mismatch() {
    let src = tmp("sv");
    write(&src, CLEAN_TB);
    let vu = tmp("vu");
    let velab = tmp("velab");
    cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default());
    cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default());

    // corrupt the header schema_hash in place, re-stitch, and feed to vrun.
    let bytes = std::fs::read(&velab).unwrap();
    let (mut h, body) = vita_artifact::read_velab(&bytes).unwrap();
    h.schema_hash[0] ^= 0xFF;
    let bad = vita_artifact::write_velab(&h, body);
    let stale = tmp("velab");
    std::fs::write(&stale, &bad).unwrap();
    assert_eq!(
        cli::run_vrun(&s(&stale), &cli::VitaOpts::default()),
        cli::EXIT_STALE
    );

    let tool = vita_artifact::ToolContext::current();
    let (got, _) = vita_artifact::read_velab(&bad).unwrap();
    assert_eq!(
        vita_artifact::verify_header(&got, &tool).unwrap_err().code,
        diag::MsgCode::ArtSchemaMismatch
    );

    for p in [&src, &vu, &velab, &stale] {
        let _ = std::fs::remove_file(p);
    }
}

// TEST 10: missing input file → CLI/usage error (exit 3) for all three applets.
#[test]
fn missing_input_exits_three() {
    let nope = "/nonexistent/path/staged_xyz.vu".to_string();
    assert_eq!(
        cli::run_velab(&nope, "/tmp/x.velab", &cli::VitaOpts::default()),
        cli::EXIT_CLI_ERROR
    );
    assert_eq!(
        cli::run_vrun(
            "/nonexistent/path/staged_xyz.velab",
            &cli::VitaOpts::default()
        ),
        cli::EXIT_CLI_ERROR
    );
    assert_eq!(
        cli::run_vcmp(
            &["/nonexistent/path/x.sv".into()],
            Some("/tmp/x.vu"),
            &cli::VitaOpts::default()
        ),
        cli::EXIT_CLI_ERROR
    );
}

// TEST 11: corrupt body behind a VALID header → E-ART-FORMAT-MISMATCH.
#[test]
fn vrun_rejects_corrupt_body() {
    let src = tmp("sv");
    write(&src, CLEAN_TB);
    let vu = tmp("vu");
    let velab = tmp("velab");
    cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default());
    cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default());

    // keep the header+magic, append only 1 body byte → undecodable SimIr frame.
    let bytes = std::fs::read(&velab).unwrap();
    let (h, _body) = vita_artifact::read_velab(&bytes).unwrap();
    let truncated = vita_artifact::write_velab(&h, &[0x01]); // 1-byte bogus body
    let bad = tmp("velab");
    std::fs::write(&bad, &truncated).unwrap();
    assert_eq!(
        cli::run_vrun(&s(&bad), &cli::VitaOpts::default()),
        cli::EXIT_STALE
    );

    // Pin the failure to the BODY-DECODE boundary: the header gate must PASS
    // (hash unchanged), and only the SimIr frame decode must fail.
    let (got, body) = vita_artifact::read_velab(&truncated).expect("magic+header still decode");
    assert!(
        vita_artifact::verify_header(&got, &vita_artifact::ToolContext::current()).is_ok(),
        "header gate must PASS — corruption is in the body, not the header"
    );
    assert!(
        postcard::take_from_bytes::<sim_ir::SimIr>(body).is_err(),
        "the 1-byte SimIr frame must fail to decode (E-ART-FORMAT-MISMATCH boundary)"
    );

    for p in [&src, &vu, &velab, &bad] {
        let _ = std::fs::remove_file(p);
    }
}

// TEST 12 (ACTIVE): the fork trailer survives the staged path. A fork design run
//          via the staged chain interleaves concurrently, exactly matching one-shot.
//          Fork-join elaboration has landed (commit e0fdc94): `elaborate_with_modes`
//          emits a real `Terminator::Fork` + a non-empty `ForkModeTable`, and the
//          engine runs the children concurrently. `fork #5 a=1; #3 b=1; join` joins
//          at t=5 (parent waits for BOTH children). If the trailer were dropped, the
//          fork modes would default and the result would differ — so this genuinely
//          proves the trailer survives the .velab body layout.
#[test]
fn fork_trailer_survives_staged_path() {
    const FORK_TB: &str = "module tb; reg a; reg b; initial begin \
        fork #5 a=1; #3 b=1; join \
        $display(\"a=%b b=%b t=%0d\", a, b, $time); $finish; end endmodule";
    let src = tmp("sv");
    write(&src, FORK_TB);
    let vu = tmp("vu");
    let velab = tmp("velab");
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );
    assert_eq!(
        cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()),
        cli::EXIT_OK
    );

    // the trailer MUST be non-empty (the join carries a JoinMode::All entry).
    let bytes = std::fs::read(&velab).unwrap();
    let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
    let (staged_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
    assert!(
        !modes.is_empty(),
        "fork design must persist a non-empty ForkModeTable trailer"
    );

    // staged run interleaves both children concurrently and joins at t=5 — identical to one-shot.
    let (staged_res, staged_out) = sim_engine::simulate_capture(
        &staged_ir,
        sim_engine::SimOpts {
            fork_modes: modes,
            ..Default::default()
        },
    );
    let ref_unit = cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).unwrap();
    let (ir, fm) = elaborate::elaborate_with_modes(&ref_unit, &cli::StderrSink::new());
    let (ref_res, ref_out) = sim_engine::simulate_capture(
        &ir.unwrap(),
        sim_engine::SimOpts {
            fork_modes: fm,
            ..Default::default()
        },
    );
    assert_eq!(staged_out, ref_out, "staged transcript must equal one-shot");
    assert!(
        staged_out.contains("a=1 b=1 t=5"),
        "join waits for BOTH children → a=1 b=1 at t=5 (concurrent); got {staged_out:?}"
    );
    assert_eq!(staged_res.exit_class, ref_res.exit_class);

    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

// P2-7: artifact writes are atomic (temp + rename) — a successful write leaves
// no `.tmp.` residue next to the artifact, so a crash mid-write can never leave
// a partial file that the staleness gate would misreport as format-mismatch.
#[test]
fn artifact_write_leaves_no_tmp_residue() {
    let dir = std::env::temp_dir().join(format!(
        "vita_atomic_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("tb.sv");
    write(&src, CLEAN_TB);
    let vu = dir.join("tb.vu");
    let velab = dir.join("tb.velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert!(vu.exists() && velab.exists());
    let residue: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".tmp."))
        .collect();
    assert!(residue.is_empty(), "tmp residue left behind: {residue:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

// P1-7: a `.velab` whose fork-mode trailer is missing its entries (the trailer
// rides OUTSIDE the schema gate, so hand-truncation can produce this) must end
// in a graceful FATAL diagnostic + exit 1 — not a process abort (was: panic!).
#[test]
fn corrupted_fork_trailer_fails_gracefully() {
    let src = tmp("sv");
    write(
        &src,
        "module tb; reg a; initial begin fork a = 1; join $finish; end endmodule",
    );
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);

    // Surgically EMPTY the ForkModeTable trailer, leaving everything else intact.
    let bytes = std::fs::read(&velab).unwrap();
    let (header, body) = vita_artifact::read_velab(&bytes).expect("read_velab");
    let (_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).expect("SimIr frame");
    let ir_len = body.len() - rest.len();
    let (fm, rest2): (sim_engine::ForkModeTable, &[u8]) =
        postcard::take_from_bytes(rest).expect("fork trailer");
    assert!(!fm.is_empty(), "fixture must actually have a fork entry");
    let mut new_body = body[..ir_len].to_vec();
    new_body.extend_from_slice(&postcard::to_stdvec(&sim_engine::ForkModeTable::new()).unwrap());
    new_body.extend_from_slice(rest2);
    std::fs::write(&velab, vita_artifact::write_velab(&header, &new_body)).unwrap();

    // Must be a clean nonzero exit (Fatal diagnostic), not a panic/abort.
    assert_eq!(cli::run_vrun(&s(&velab), &opts), cli::EXIT_USER_ERROR);
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

/// The assign-rank sidecar must survive the `.velab` trailer: without it the
/// staged `vrun` would treat the proc-assign as a STRONG force — `release`
/// would then unpin (value holds 20) instead of resuming the assign (10), and
/// the design $fatals (exit 1).
#[test]
fn staged_assign_rank_trailer_roundtrip() {
    let src = tmp("sv");
    write(
        &src,
        "module tb; reg [7:0] q; \
           initial begin \
             assign q = 8'd10; \
             force q = 8'd20; \
             release q; \
             if (q != 8'd10) $fatal(1, \"assign rank lost across the trailer\"); \
             $finish; \
           end \
         endmodule",
    );
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert_eq!(
        cli::run_vrun(&s(&velab), &opts),
        cli::EXIT_OK,
        "release must resume the assign (rank table round-trips)"
    );
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

/// STAGED-DROP (2026-06-22 audit): the N7 class sidecars (layouts/vtable/field
/// inits/new-sites/handle-nets/field-widths) must survive the `.velab` trailer.
/// Without them the staged `vrun` reads `c.x` as the bare default (0/X) instead
/// of the ctor-assigned 42 — previously a SILENT-WRONG: one-shot printed 42 but
/// staged printed 0 with exit 0. The self-checking `$fatal` turns that into a
/// loud staged-vs-one-shot divergence (exit 1) the round-trip must prevent.
#[test]
fn staged_class_sidecars_trailer_roundtrip() {
    let src = tmp("sv");
    write(
        &src,
        "class C; int x; function new(); x = 42; endfunction endclass\n\
         module tb; C c;\n\
           initial begin\n\
             c = new;\n\
             if (c.x !== 32'd42) $fatal(1, \"class sidecars lost across the .velab trailer\");\n\
             $finish;\n\
           end\n\
         endmodule",
    );
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert_eq!(
        cli::run_vrun(&s(&velab), &opts),
        cli::EXIT_OK,
        "staged vrun must read c.x=42 (class layout/field-init sidecars round-trip)"
    );
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}

/// STAGED-DROP (2026-06-22 audit): the B-track frame-call sidecars (func_table +
/// task-call binding tables) must survive the `.velab` trailer. Without them a
/// recursive automatic function silently returns X across the staged path —
/// `fac(5)` was 120 one-shot but X staged with exit 0 (fully silent-wrong, the
/// worst defect class). The `$fatal` makes the divergence loud.
#[test]
fn staged_frame_call_sidecars_trailer_roundtrip() {
    let src = tmp("sv");
    write(
        &src,
        "module tb;\n\
           function automatic int fac(int n);\n\
             if (n <= 1) fac = 1; else fac = n * fac(n - 1);\n\
           endfunction\n\
           initial begin\n\
             if (fac(5) !== 32'd120) $fatal(1, \"frame-call sidecars lost across the .velab trailer\");\n\
             $finish;\n\
           end\n\
         endmodule",
    );
    let vu = tmp("vu");
    let velab = tmp("velab");
    let opts = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&src)], Some(&*s(&vu)), &opts),
        cli::EXIT_OK
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &opts), cli::EXIT_OK);
    assert_eq!(
        cli::run_vrun(&s(&velab), &opts),
        cli::EXIT_OK,
        "staged vrun must compute fac(5)=120 (frame-call sidecars round-trip)"
    );
    for p in [&src, &vu, &velab] {
        let _ = std::fs::remove_file(p);
    }
}
