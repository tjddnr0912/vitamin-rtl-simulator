//! `Sidecars::lossy_spell`, the design-wide lossy-spelling record (§5.2 Q-LOSSYSPELL, unit
//! U2): the OR of every parsed unit's `TopItem::LossySpell` the elaboration receives, then
//! `param_xz` for a `-G` value whose text spells an unknown, complete at the head of
//! `Elaborator::run` (`crates/elaborate/src/lossy.rs`). The record is read here and by no
//! product code (home rule H4, `crates/elaborate/tests/lossy_home.rs`).
//!
//! Every pin also fixes what `vita` prints (exit code, the design's own lines, the codes of
//! its diagnostics): those are PRE's (W/mdrv2/verify/bin/vita-post, md5 54bc2791, recorded
//! in W/lossy/build/u2pins/pre ; a build with §5.2 Q-MDRVMIX's rules, since pulled out,
//! which no pinned design reaches), so a pin that moves an output fails on that half first.
//! No oracle for the bits: a bit is which spellings the design's text (and `-G`) holds,
//! read off the row's rule (W/lossy/plan2/PLAN2.txt section C, option (c) by RULINGS-plan
//! R14). Bits: P param_xz, U unsized_xz, S sign_kw_param, L let_any, F fill_xz.
//!
//! Groups: the design-wide cells of the grounding (D-SCOPE: each spelled-but-unreached form
//! is `true`) ; one bit alone per cell ; the OR over a compose's units and over `-G` ; the
//! three `-G` spellings bound silently today (premise judge 1 L6) with a clean control ;
//! M3 on b6_staged (mono `true`, staged `false` for the unit `velab -L` never decodes,
//! RULINGS-plan R9) ; the left sides on which a plain fill is refused today (premise judge
//! 2 L2) and the refused `config` use clause (L3), each LOUD with its code ; the all-false
//! control (EH1's one fill shape) ; the design-level superset over the row's cells. Unit U3
//! (the lexer's x / z literal kinds) adds: the forms the split must not move, printed as
//! U2 printed them (premise judge 3 L6) ; `let` as a type name (L7) ; the lexer's fact over
//! the preprocessed text. Fix N4 (unit U4, the right-side marks) adds: a plain fill that is
//! a whole marked right side on a left side refused today stays LOUD with its code (premise
//! judge N4 L8: a concatenation target, a part-select `force`, a concatenation `force`).

use diag::{LogEvent, LogSink};
use hdl_ast::{LossySpell, SourceUnit, TopItem};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const P: u8 = 1;
const U: u8 = 2;
const S: u8 = 4;
const L: u8 = 8;
const F: u8 = 16;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Quiet;
impl LogSink for Quiet {
    fn emit(&self, _: LogEvent) {}
}

fn mask(ls: LossySpell) -> u8 {
    [
        (ls.param_xz, P),
        (ls.unsized_xz, U),
        (ls.sign_kw_param, S),
        (ls.let_any, L),
        (ls.fill_xz, F),
    ]
    .iter()
    .filter(|(b, _)| *b)
    .fold(0, |m, (_, k)| m | k)
}

fn tdir() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_lossy_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A design on disk: `files` are passed on the command line in order, `extra` (an
/// included header) only written.
fn write(d: &Path, files: &[(&str, &str)], extra: &[(&str, &str)]) {
    for (n, t) in files.iter().chain(extra) {
        std::fs::write(d.join(n), t).unwrap();
    }
}

fn gpairs(g: &[(&str, &str)]) -> Vec<(String, String)> {
    g.iter()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
}

/// The unit one-shot `vita` hands to elaborate: the files preprocessed as ONE unit
/// (`cli::frontend_sources_mapped`, includes from the first file's directory). `None` when
/// the front end refuses the design (then `vita` never elaborates).
fn unit_of(
    d: &Path,
    files: &[(&str, &str)],
) -> Option<(SourceUnit, hdl_preprocess::ResolvedTimescales)> {
    let sources: Vec<(String, String)> = files
        .iter()
        .map(|(n, t)| (d.join(n).to_string_lossy().into_owned(), t.to_string()))
        .collect();
    cli::frontend_sources_mapped(&sources, &Quiet, &hdl_preprocess::PreOpts::default())
        .map(|((u, rt, _), _)| (u, rt))
}

/// `Sidecars::lossy_spell` of a unit, through the entry one-shot `vita` calls.
fn record(
    unit: &SourceUnit,
    rt: Option<&hdl_preprocess::ResolvedTimescales>,
    g: &[(&str, &str)],
) -> u8 {
    let empty = std::collections::BTreeMap::new();
    let (ue, pe, gp) = rt.map_or((&empty, &empty, -9), |r| {
        (&r.unit_exp, &r.prec_exp, r.global_prec_exp)
    });
    let (_, sc) =
        elaborate::elaborate_located_params(unit, &Quiet, ue, pe, gp, None, None, &gpairs(g));
    mask(sc.lossy_spell)
}

/// The design record of `files` with `-G` values `g`, as one-shot `vita` computes it.
fn design(files: &[(&str, &str)], extra: &[(&str, &str)], g: &[(&str, &str)]) -> Option<u8> {
    let d = tdir();
    write(&d, files, extra);
    let r = unit_of(&d, files).map(|(u, rt)| record(&u, Some(&rt), g));
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// What `vita` prints: (exit code, the design's own lines, the sorted distinct diagnostic
/// codes other than W1017).
fn vita_in(d: &Path, args: &[&str]) -> (i32, Vec<String>, Vec<String>) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let design: Vec<String> = text
        .lines()
        .filter(|l| {
            let mut c = l.chars();
            c.next().is_some_and(|a| a.is_ascii_uppercase()) && c.next() == Some(' ')
        })
        .map(str::to_string)
        .collect();
    let mut codes: Vec<String> = text
        .match_indices("[VITA-")
        .map(|(at, _)| text[at + 6..at + 11].to_string())
        .filter(|c| c != "W1017")
        .collect();
    codes.sort();
    codes.dedup();
    (out.status.code().expect("exit code"), design, codes)
}

fn vita(
    files: &[(&str, &str)],
    extra: &[(&str, &str)],
    args: &[&str],
) -> (i32, Vec<String>, Vec<String>) {
    let d = tdir();
    write(&d, files, extra);
    let mut a: Vec<&str> = args.to_vec();
    a.extend(files.iter().map(|(n, _)| *n));
    let r = vita_in(&d, &a);
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// One cell: `vita`'s exit code, design lines and codes (PRE's), then the record.
#[track_caller]
fn pin(
    files: &[(&str, &str)],
    extra: &[(&str, &str)],
    g: &[(&str, &str)],
    rc: i32,
    lines: &[&str],
    codes: &[&str],
    bits: u8,
) {
    let mut args = Vec::new();
    let gs: Vec<String> = g.iter().map(|(n, v)| format!("{n}={v}")).collect();
    for s in &gs {
        args.push("-G");
        args.push(s.as_str());
    }
    let (got_rc, got_lines, got_codes) = vita(files, extra, &args);
    assert_eq!(
        (got_rc, got_lines, got_codes),
        (
            rc,
            lines.iter().map(|s| s.to_string()).collect(),
            codes.iter().map(|s| s.to_string()).collect()
        ),
        "vita's output (PRE's)"
    );
    assert_eq!(design(files, extra, g), Some(bits), "the design record");
}

/// A refused design: `vita`'s exit code and codes ; the front end refuses it, or the
/// elaboration does (the record then exists but carries no claim here).
#[track_caller]
fn loud(src: &str, code: &str) {
    let (rc, lines, codes) = vita(&[("t.sv", src)], &[], &[]);
    assert_eq!(
        (rc, lines, codes),
        (1, Vec::<String>::new(), vec![code.to_string()]),
        "`{src}`"
    );
}

// ─────────────────────────── the design-wide cells (D-SCOPE) ───────────────────────────

const Q4_TWO_INST: &str = "module c #(parameter logic [3:0] B = 0) ();
  initial #1 $display(\"A %m b=%b\", B);
endmodule
module t;
  c #(.B(4'b1101)) u0();
  c #(.B(4'b1x01)) u1();
  initial #100 $finish;
endmodule
";
const Q4_UNINST_MOD: &str = "module z;
  localparam unsigned U = 4'sd8;
  initial $display(\"Z u=%0d\", U);
endmodule
module t;
  localparam V = 4'd8;
  initial begin #1 $display(\"A v=%0d\", V); end
  initial #100 $finish;
endmodule
";

/// W/lossy/ground/cells/q4_*: each spelled-but-unreached form counts (u0 is right, the
/// design still spells u1 ; the uninstantiated module is a second top in one-shot ; an
/// untaken generate arm, an unused package, an included header, a second file).
#[test]
fn design_wide_cells_are_true() {
    pin(
        &[("t.sv", Q4_TWO_INST)],
        &[],
        &[],
        0,
        &["A t.u0 b=1101", "A t.u1 b=1001"],
        &[],
        P,
    );
    pin(
        &[("t.sv", Q4_UNINST_MOD)],
        &[],
        &[],
        0,
        &["Z u=-8", "A v=8"],
        &["W3057"],
        S,
    );
    pin(
        &[(
            "t.sv",
            "module t;
  if (0) begin : g
    localparam unsigned U = 4'sd8;
    initial $display(\"G u=%0d\", U);
  end
  localparam V = 4'd8;
  initial begin #1 $display(\"A v=%0d gt=%b\", V, V > 4'sd0); end
  initial #100 $finish;
endmodule
",
        )],
        &[],
        &[],
        0,
        &["A v=8 gt=1"],
        &[],
        S,
    );
    pin(
        &[(
            "t.sv",
            "package p;
  localparam unsigned U = 4'sd8;
endpackage
module t;
  localparam V = 4'd8;
  initial begin #1 $display(\"A v=%0d\", V); end
  initial #100 $finish;
endmodule
",
        )],
        &[],
        &[],
        0,
        &["A v=8"],
        &[],
        S,
    );
    pin(
        &[(
            "t.sv",
            "module t;
`include \"p.svh\"
  initial begin #1 $display(\"A u=%0d gt=%b\", U, U > 4'sd0); end
  initial #100 $finish;
endmodule
",
        )],
        &[("p.svh", "  localparam unsigned U = 4'sd8;\n")],
        &[],
        0,
        &["A u=-8 gt=0"],
        &[],
        S,
    );
    pin(
        &[
            (
                "t.sv",
                "module t;
  c #(.B(4'b1x01)) u();
  initial #100 $finish;
endmodule
",
            ),
            (
                "b.sv",
                "module c #(parameter logic [3:0] B = 0) ();
  initial #1 $display(\"A b=%b\", B);
endmodule
",
            ),
        ],
        &[],
        &[],
        0,
        &["A b=1001"],
        &[],
        P,
    );
}

// ─────────────────────────── one bit alone, and the OR ───────────────────────────

const BIT_UNSIZED: &str =
    "module t; logic [7:0] v; initial begin v = 'bx; #1 $display(\"A v=%b\", v); end endmodule\n";
const BIT_LET: &str =
    "module t; let f(a) = a + 1; initial #1 $display(\"A f=%0d\", f(2)); endmodule\n";
const BIT_FILL: &str = "module t; logic c = 1; logic [7:0] w; initial begin w = c ? 'x : 8'd1; #1 \
                        $display(\"A w=%b\", w); end endmodule\n";

/// Each bit alone in its own cell, so a record that moves one bit into another's place
/// fails here.
#[test]
fn each_bit_alone_through_the_design_record() {
    assert_eq!(design(&[("t.sv", Q4_TWO_INST)], &[], &[]), Some(P));
    pin(
        &[("t.sv", BIT_UNSIZED)],
        &[],
        &[],
        0,
        &["A v=xxxxxxxx"],
        &[],
        U,
    );
    assert_eq!(design(&[("t.sv", Q4_UNINST_MOD)], &[], &[]), Some(S));
    pin(&[("t.sv", BIT_LET)], &[], &[], 0, &["A f=3"], &[], L);
    pin(
        &[("t.sv", BIT_FILL)],
        &[],
        &[],
        0,
        &["A w=xxxxxxxx"],
        &[],
        F,
    );
}

/// Parse one source alone, as `vcmp` parses one unit.
fn parsed(src: &str) -> SourceUnit {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty());
    let (su, pe, _) = hdl_parser::parse_with_warnings(&toks, src);
    assert!(pe.is_empty(), "{pe:?}");
    su.expect("unit")
}

/// A library compose (`staged.rs`) keeps every loaded unit's nameless items, so the unit
/// `run` receives can hold several records: the design's is their OR, never the first one.
#[test]
fn or_over_every_unit_of_a_compose() {
    let units = [
        parsed(Q4_UNINST_MOD),
        parsed(BIT_UNSIZED),
        parsed(BIT_LET),
        parsed(BIT_FILL),
        parsed(Q4_TWO_INST),
    ];
    let merge = |order: &[usize]| SourceUnit {
        items: order
            .iter()
            .flat_map(|&i| units[i].items.iter().cloned())
            .collect(),
        span: hdl_ast::Span { lo: 0, hi: 0 },
    };
    for order in [[0, 1, 2, 3, 4], [4, 3, 2, 1, 0], [2, 0, 4, 1, 3]] {
        let m = merge(&order);
        let n = m
            .items
            .iter()
            .filter(|i| matches!(i, TopItem::LossySpell(_)))
            .count();
        assert_eq!(n, 5, "five records in the compose");
        assert_eq!(record(&m, None, &[]), P | U | S | L | F, "order {order:?}");
    }
    assert_eq!(record(&merge(&[0, 1]), None, &[]), S | U);
    assert_eq!(
        record(&merge(&[0]), None, &[("B", "'bx")]),
        S | P,
        "the -G OR"
    );
    assert_eq!(
        record(&merge(&[1]), None, &[("B", "8'd5")]),
        U,
        "a clean -G adds nothing"
    );
}

// ─────────────────────────── -G (premise judge 1 L6, PR10) ───────────────────────────

const G_CELL: &str =
    "module t #(parameter logic [7:0] B = 8'd0) (); initial #1 $display(\"A b=%b\", B); endmodule\n";

/// The three `-G` spellings bound silently today (the unknown dropped: `A b=00000000`,
/// `10010000`, `11010000`) set `param_xz` ; a fill `'x` / `'z` (refused, E3009) does too ;
/// a value naming no parameter counts (the text decides) ; a clean sized value, the fill
/// `'1`, a decimal and a quoted string do not.
#[test]
fn g_values_through_the_cli_value_reading() {
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "8'b1101_0000")],
        0,
        &["A b=11010000"],
        &[],
        0,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "'bx")],
        0,
        &["A b=00000000"],
        &[],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "8'B1X01_0000")],
        0,
        &["A b=10010000"],
        &[],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "8'b1?01_0000")],
        0,
        &["A b=11010000"],
        &[],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "'x")],
        1,
        &[],
        &["E3009"],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "'z")],
        1,
        &[],
        &["E3009"],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "'1")],
        0,
        &["A b=11111111"],
        &[],
        0,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "8")],
        0,
        &["A b=00001000"],
        &[],
        0,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "8'd5"), ("NOPE", "'bx")],
        1,
        &[],
        &["E3002"],
        P,
    );
    pin(
        &[("t.sv", G_CELL)],
        &[],
        &[("B", "\"xz\"")],
        0,
        &["A b=01111010"],
        &[],
        0,
    );
}

// ─────────────────────────── M3: b6_staged (RULINGS-plan R9) ───────────────────────────

const B6_A: &str = "module t; c u(); initial #100 $finish; endmodule
module c; initial #1 $display(\"A c ran\"); endmodule
";
const B6_B: &str =
    "module z; localparam unsigned U = 4'sd8; initial #2 $display(\"Z u=%0d gt=%0d\", U, U > 0); endmodule\n";

/// The blob `vcmp --work` wrote for `src` (the manifest's `[[cu]]` whose sources name it).
fn blob_of(lib: &Path, src: &str) -> PathBuf {
    let toml = std::fs::read_to_string(lib.join("lib.toml")).unwrap();
    let mut blob = None;
    for l in toml.lines() {
        if let Some(b) = l.strip_prefix("blob = \"") {
            blob = Some(b.trim_end_matches('"').to_string());
        }
        if l.trim().ends_with(&format!("  {src}\",")) {
            return lib.join(blob.expect("blob before sources"));
        }
    }
    panic!("no unit for {src}")
}

/// The unit a `.vu` blob carries.
fn vu_unit(path: &Path) -> SourceUnit {
    let bytes = std::fs::read(path).unwrap();
    let (_h, body) = vita_artifact::read_vu(&bytes).unwrap();
    let (u, _): (SourceUnit, &[u8]) = postcard::take_from_bytes(body).unwrap();
    u
}

/// Mono parses both files as one unit and elaborates `z` as a second top, where it is
/// silently wrong (`Z u=-8 gt=0`, W3057): the record is `true` there, as it must be.
/// Staged, `velab -L --top t` decodes only the units reachable from `t` by instance names
/// (`staged.rs`): `b.sv`'s unit is never read — shown here by deleting its blob before
/// `velab` — so nothing of it reaches `run`, and the record of the unit `run` receives (t's
/// unit, the only one loaded) is `false`. Each is right for the design that mode elaborates.
#[test]
fn m3_b6_staged_mono_true_staged_false() {
    pin(
        &[("a.sv", B6_A), ("b.sv", B6_B)],
        &[],
        &[],
        0,
        &["A c ran", "Z u=-8 gt=0"],
        &["W3057"],
        S,
    );

    let d = tdir();
    write(&d, &[("a.sv", B6_A), ("b.sv", B6_B)], &[]);
    for f in ["a.sv", "b.sv"] {
        let (rc, _, codes) = vita_in(&d, &["vcmp", "--work", "L=lib", f]);
        assert_eq!((rc, codes), (0, Vec::<String>::new()), "vcmp {f}");
    }
    let (ua, ub) = (
        blob_of(&d.join("lib"), "a.sv"),
        blob_of(&d.join("lib"), "b.sv"),
    );
    let unit_a = vu_unit(&ua);
    let rec = |u: &SourceUnit| -> Vec<u8> {
        u.items
            .iter()
            .filter_map(|i| match i {
                TopItem::LossySpell(ls) => Some(mask(*ls)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(rec(&unit_a), Vec::<u8>::new(), "a.sv spells none");
    assert_eq!(
        rec(&vu_unit(&ub)),
        vec![S],
        "b.sv's unit carries its record"
    );
    std::fs::remove_file(&ub).unwrap();
    let (rc, _, codes) = vita_in(&d, &["velab", "-L", "L=lib", "--top", "t", "-o", "t.velab"]);
    assert_eq!(
        (rc, codes),
        (0, Vec::<String>::new()),
        "velab never reads b.sv's blob"
    );
    let (rc, lines, codes) = vita_in(&d, &["vrun", "t.velab"]);
    assert_eq!((rc, lines, codes), (0, vec!["A c ran".to_string()], vec![]));
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(record(&unit_a, None, &[]), 0, "the staged design's record");
}

// ─────────────────────────── LOUD today (premise judge 2 L2, L3) ───────────────────────────

/// A plain-shape fill on these left sides is refused today. PR16 holds here only while
/// refused ; opening it re-measures the plain fill there (T7). `wor` is not refused: it is
/// RIGHT (vita = iverilog 13.0 `A w=xxxxxxxxxxxxxxxxxx w2=zzzzzzzzzzzzzzzzzz`), where
/// premise judge 2 L2 listed it beside `tri1` as E3009 (W/lossy/build/NOTES.txt ROUND 2).
#[test]
fn plain_fill_on_a_refused_left_side_stays_loud() {
    loud(
        "module t; logic [69:0] a, b; initial begin {>>{a, b}} = 'x; #1 $display(\"A a=%h\", a); end endmodule\n",
        "E2002",
    );
    loud(
        "module t; typedef struct { logic [69:0] a = 'x; int b; } us_t; us_t us; initial #1 $display(\"A a=%h\", us.a); endmodule\n",
        "E2002",
    );
    loud(
        "class C; static logic [69:0] f = 'x; endclass module t; initial #1 $display(\"A f=%h\", C::f); endmodule\n",
        "E2002",
    );
    loud(
        "module t; var logic [69:0] v = 'x; initial #1 $display(\"A v=%h\", v); endmodule\n",
        "E2002",
    );
    loud(
        "module t; const logic [69:0] c = 'x; initial #1 $display(\"A c=%h\", c); endmodule\n",
        "E2002",
    );
    loud(
        "module t; tri1 [69:0] n = 'z; initial #1 $display(\"A n=%h\", n); endmodule\n",
        "E3009",
    );
    pin(
        &[(
            "t.sv",
            "module t; wor [69:0] w = 'x; wor [69:0] w2 = 'z; initial #1 $display(\"A w=%h w2=%h\", w, w2); endmodule\n",
        )],
        &[],
        &[],
        0,
        &["A w=xxxxxxxxxxxxxxxxxx w2=zzzzzzzzzzzzzzzzzz"],
        &[],
        0,
    );
}

/// Premise judge L8 of fix N4 (§5.2 Q-LOSSYSPELL U4): a plain fill the parser marks at the
/// procedural `assign` (S3) and `force` (S4) sites is exempt, and on these left sides it is
/// right only because vita refuses them (E3009 ; IEEE 1800-2017 10.6.1 / 10.6.2 allow them,
/// iverilog runs them: RULINGS L1-20, a pre-existing loud refusal). PR16 holds here only
/// while refused ; opening one re-measures the plain fill there (T7). Cells
/// W/lossy/fixn4/premise/cells e1_pasg, e2_force (E3009 on PRE and POST).
#[test]
fn n4_marked_fill_on_a_refused_target_stays_loud() {
    loud(
        "module t; logic [34:0] c1, c0; initial begin assign {c1, c0} = 'z; #1 $display(\"A c=%h%h\", c1, c0); end endmodule\n",
        "E3009",
    );
    loud(
        "module t; wire [69:0] n = 70'h5; initial begin force n[39:0] = 'x; #1 $display(\"A n=%h\", n); end endmodule\n",
        "E3009",
    );
    loud(
        "module t; wire [34:0] a1, a0; assign a1 = '0; assign a0 = '1; initial begin force {a1, a0} = 'x; #1 $display(\"A a=%h%h\", a1, a0); end endmodule\n",
        "E3009",
    );
}

/// Annex A's one parameter value outside both regions, a config's `use .P(v)`
/// (ieee1800-2017.txt:41388-41391): vita refuses `config` (E2002). A row that makes
/// `config` parse adds its use clause to scan_lossy's regions in the same commit.
#[test]
fn config_use_clause_stays_loud() {
    loud(
        "module sub #(parameter logic [7:0] W = 8'd1); initial #1 $display(\"A w=%b\", W); endmodule
module t; sub u(); initial #100 $finish; endmodule
config cfg; design t; instance t.u use #(.W(8'b1x01_0000)); endconfig
",
        "E2002",
    );
}

// ─────────────────────────── the all-false control ───────────────────────────

/// EH1's one fill shape, `Q <= 'x;` (41 of 41 in eh1_noassert): the plain shape, right
/// (= iverilog), counted by no bit.
#[test]
fn all_false_control() {
    pin(
        &[(
            "t.sv",
            "module t; logic clk = 0; logic [31:0] Q; always @(posedge clk) Q <= 'x; initial begin #1 clk = 1; #1 $display(\"A q=%h\", Q); end endmodule\n",
        )],
        &[],
        &[],
        0,
        &["A q=xxxxxxxx"],
        &[],
        0,
    );
}

// ─────────────────────────── H2 (d) at the design level ───────────────────────────

/// How many `Source` x / z literals sit inside a parameter holder of a unit's `Debug` text
/// (the walk of hdl-parser `src/tests/lossy_spell.rs`, `holder_xz_literals`).
fn holder_xz_literals(dbg: &str) -> usize {
    const HOLDERS: [&str; 4] = [
        "params: [",
        "Param(ParamDecl {",
        "param_overrides: [",
        "Defparam(DefparamItem {",
    ];
    let mut n = 0;
    for h in HOLDERS {
        let mut from = 0;
        while let Some(at) = dbg[from..].find(h) {
            let open = from + at + h.len() - 1;
            let close = balanced_end(dbg, open);
            n += xz_source_literals(&dbg[open..close]);
            from = open + 1;
        }
    }
    n
}

fn balanced_end(s: &str, open: usize) -> usize {
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (i, c) in s[open..].char_indices() {
        if in_str {
            match (esc, c) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' | '{' | '(' => depth += 1,
            ']' | '}' | ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + i + 1;
                }
            }
            _ => {}
        }
    }
    s.len()
}

fn xz_source_literals(s: &str) -> usize {
    s.match_indices("IntLit { kind: ")
        .filter(|(at, _)| {
            let rest = &s[*at..];
            let end = rest.find('}').unwrap_or(rest.len());
            let lit = &rest[..end];
            let raw = lit
                .split("raw: \"")
                .nth(1)
                .and_then(|r| r.split('"').next());
            lit.contains("origin: Source")
                && raw.is_some_and(|r| {
                    let after = r.split_once('\'').map_or("", |(_, a)| a);
                    let after = after.strip_prefix(['s', 'S']).unwrap_or(after);
                    let digits = match after.chars().next() {
                        Some('d' | 'D' | 'b' | 'B' | 'o' | 'O' | 'h' | 'H') => &after[1..],
                        _ => after,
                    };
                    digits.contains(['x', 'X', 'z', 'Z', '?'])
                })
        })
        .count()
}

/// Over the row's measured cells (hdl-parser `tests/testdata/lossy_cells`), each the
/// front end accepts: the design record (no `-G`) equals its unit's own record, and a
/// `Source` x / z literal in a parameter holder means `param_xz`. Blind spot (premise
/// judge 2 L6): a class specialization's actuals are substituted into the monomorphized
/// body by name and a class's own `#( )` defaults are kept out of the unit, so neither
/// reaches a holder this walk lists — `class_paths_set_param_xz` is those paths' guard.
#[test]
fn design_superset_over_the_cells() {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdl-parser/tests/testdata/lossy_cells");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "sv"))
        .collect();
    names.sort();
    let (mut seen, mut with_holder) = (0, 0);
    for p in &names {
        let src = std::fs::read_to_string(p).unwrap();
        let d = tdir();
        write(&d, &[("t.sv", &src)], &[]);
        let Some((u, rt)) = unit_of(&d, &[("t.sv", &src)]) else {
            let _ = std::fs::remove_dir_all(&d);
            continue;
        };
        let _ = std::fs::remove_dir_all(&d);
        seen += 1;
        let own: u8 = u
            .items
            .iter()
            .filter_map(|i| match i {
                TopItem::LossySpell(ls) => Some(mask(*ls)),
                _ => None,
            })
            .fold(0, |a, m| a | m);
        let got = record(&u, Some(&rt), &[]);
        assert_eq!(got, own, "{}: design record vs the unit's", p.display());
        let n = holder_xz_literals(&format!("{u:?}"));
        if n > 0 {
            with_holder += 1;
            assert!(
                got & P != 0,
                "{}: {n} holder x / z literal(s), param_xz false",
                p.display()
            );
        }
    }
    assert!(seen >= 100, "cells accepted by the front end: {seen}");
    assert!(
        with_holder >= 10,
        "cells with a holder x / z literal: {with_holder}"
    );
}

/// The two class paths the superset walk cannot see (premise judge 1 L1, judge 2 L6).
#[test]
fn class_paths_set_param_xz() {
    let collide = "module t;
  class C #(parameter logic [7:0] P = 0); function logic [7:0] v(); return P; endfunction endclass
  C #(8'b0000101) a; C #(8'b0000_1?01) b;
  initial begin a = new; b = new; #1 $display(\"A a=%b b=%b\", a.v(), b.v()); end
  initial #100 $finish;
endmodule
";
    assert_eq!(design(&[("t.sv", collide)], &[], &[]), Some(P));
    let default = "class C #(parameter logic [7:0] R = 8'b1x01_0000); function logic [7:0] v(); \
                   return R; endfunction endclass
module t; C c; initial begin c = new; #1 $display(\"A r=%b\", c.v()); end endmodule
";
    assert_eq!(design(&[("t.sv", default)], &[], &[]), Some(P));
}

// ───────────────────── U3: the lexer's x / z literal kinds (§5.2 Q-LOSSYSPELL U3) ─────────────────────

/// What `vita t.sv` prints, stdout and stderr into ONE file as the cells' watchdog
/// (W/tools/wd.pl: STDERR dup'd onto STDOUT) wrote U2's outputs, and its exit code.
fn vita_merged(src: &str) -> (i32, String) {
    let d = tdir();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let path = d.join("merged.out");
    let f = std::fs::File::create(&path).unwrap();
    let st = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .stdout(f.try_clone().unwrap())
        .stderr(f)
        .status()
        .expect("run vita");
    let text = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    (st.code().expect("exit code"), text)
}

/// Premise judge 3 L6: the split moves no lexeme. Three sources over every literal form the
/// judge listed (a1: 43 legal forms, `?`, upper case, `_`, §5.7.1 spaces, tab, signed,
/// decimal x / z, fills, casez / casex, a parameter override ; a2: illegal forms the parser
/// refuses ; a3: forms elaborate refuses, `8'd1x` one token) print byte for byte what U2's
/// release `vita` printed on them (W/lossy/premise3/out/<cell>.base.out, md5 fc0135e7,
/// d42d7fbb, 334adbc0 ; rc 0, 1, 1). Run on this test's debug `vita`, so `parse_lexed`'s
/// `debug_assert` (the lexer's fact against the token vector's) runs on each.
#[test]
fn split_forms_print_what_u2_printed() {
    for (id, src, want, rc) in [
        (
            "a1_forms2",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a1_forms2.sv"),
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a1_forms2.out"),
            0,
        ),
        (
            "a2_iparse",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a2_iparse.sv"),
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a2_iparse.out"),
            1,
        ),
        (
            "a3_ielab",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a3_ielab.sv"),
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/a3_ielab.out"),
            1,
        ),
    ] {
        assert_eq!(vita_merged(src), (rc, want.to_string()), "{id}");
    }
}

/// Premise judge 3 L7, RULINGS-plan R23: `let_any` is the parser's note at `parse_let_decl`,
/// so a unit that uses `let` as a type name and builds no `LetDecl` reads false. U2 (the
/// token scan's `let` arm) printed on each, W/lossy/premise3/out/k*.lb_base.out:
/// "BITS param_xz=0 unsized_xz=0 sign_kw_param=0 let_any=1 fill_xz=0 items=1 parse_errors=0".
/// `vita`'s output is U2's (W/lossy/plan3/cells/k*/vita.out).
#[test]
fn let_as_a_type_name_is_not_a_let_declaration() {
    for (id, src, line) in [
        (
            "k1_fnlocal",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/k1_fnlocal.sv"),
            "A 5",
        ),
        (
            "k2_port",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/k2_port.sv"),
            "B 3",
        ),
        (
            "k3_block",
            include_str!("../../hdl-parser/tests/testdata/lossy_u3/k3_block.sv"),
            "C 9",
        ),
    ] {
        let (rc, lines, codes) = vita(&[("t.sv", src)], &[], &[]);
        assert_eq!(
            (rc, lines, codes),
            (0, vec![line.to_string()], vec![]),
            "{id}"
        );
        assert_eq!(
            design(&[("t.sv", src)], &[], &[]),
            Some(0),
            "{id}: all false"
        );
    }
}

/// The lexer's fact over the preprocessed text: an x / z literal in `ifdef`-off text or in
/// a macro that is never expanded is no literal the lexer sees (false) ; an expanded one,
/// or one pasted by the preprocessor, is (true). Production lexes `pp.text` once
/// (`frontend.rs`), as here.
#[test]
fn lex_fact_reads_the_preprocessed_text() {
    let fact = |src: &str| {
        let d = tdir();
        let pp = hdl_preprocess::preprocess_sources(
            &d,
            &[("t.sv".to_string(), src.to_string())],
            &hdl_preprocess::PreOpts::default(),
        );
        let _ = std::fs::remove_dir_all(&d);
        assert!(!pp.has_errors(), "`{src}`");
        hdl_lexer::lex_facts(&pp.text).2.xz_literal()
    };
    assert!(!fact(
        "`ifdef NEVER\nlocalparam P = 4'bx;\n`endif\nmodule m; endmodule\n"
    ));
    assert!(!fact("`define X 4'bz\nmodule m; endmodule\n"));
    assert!(fact(
        "`define X 4'bz\nmodule m; wire [3:0] w = `X; endmodule\n"
    ));
    assert!(fact(
        "`define B(d) 4'b``d\nmodule m; wire [3:0] w = `B(1x01); endmodule\n"
    ));
    assert!(fact(
        "`ifndef NEVER\nmodule m; wire w = 'z; endmodule\n`endif\n"
    ));
}
