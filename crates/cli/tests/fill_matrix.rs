//! THE FILL MATRIX: an unsized fill (`'0`, `'1`, `'x`, `'z`) on the right of an assignment
//! takes the width of the WHOLE left side, or the design is refused (§5.2 Q-LOSSYSPELL
//! fix H, batch 6). One row per (site, left-side form, fill): a generated design run by
//! the built `vita`, with one assertion per row — the printed value of every target piece,
//! or the refusal's text.
//!
//! THE RULE HELD HERE (elaborate `expr_ctx.rs` `resize_rhs_for_lvalue`, `edge_gate.rs`
//! `fill_context` / `verify_use_recs`): a fill in an assignment's right side is lowered
//! ONCE, at the assignment, under `fill_context(lv)` — `Bits(w)` (every piece decided),
//! `Late` (a piece written through a deferred hierarchical name, or a part-select whose
//! width is decided only after elaboration), `NoBits` (one `real` piece) or `Unprovable`
//! (a record at the assignment's own position: one error at the end, the one constant
//! sentence that prints no value, unless the elaboration had already reported one) — and
//! an `'x` / `'z` fill lowered under `Late` leaves one record naming its own constant and
//! the whole target, which the end-phase judge re-sizes at the sum of every piece or
//! refuses.
//!
//! WHERE THE ROWS COME FROM (`fill_matrix/cells.txt`, one `%%cell` per row, the shared
//! module text as `%%block`s; every design is byte-identical to a measured cell):
//! - `g2`: fix H's grounding, 280 cells over the eight parser-marked sites S1-S8
//!   (`=`, `<=`, procedural `assign`, `force`, continuous `assign`, a declaration
//!   initializer, a typed for-initializer, a for-clause) x 132 left-side forms;
//! - `g3`: the grounding's 45 cells of the other doors (arguments, ports, parameters, case
//!   items, `let`, intra-assignment controls);
//! - `plan`: the plan's 56 + 7 cells (the IR the rule produces, hand-spelled; real
//!   pieces; negative-literal selects; the x_merge ternary; undeclared names);
//! - `judge`: the premise judge's rows (L5): reversed negative-literal part-selects,
//!   operator siblings, a late indexed width on a hierarchical target, 2-state pieces, an
//!   x_merge onto a concatenation, Q-FILLCTX's doors; and the premise-judge delta's `j2_`
//!   rows, each with its whole predicted stderr (every error line and the `errors=N`
//!   count): the target not provable (positions, the dead arm and the uncalled function)
//!   and four designs that fail for another reason first, whose lines are the reference's
//!   (no line added); and the `j3_` rows of fix H's third round: every refusal the rule
//!   adds is one constant with no reason and no number, a continuous assignment list's
//!   pairs each at their own position, and two lines at two positions where two print;
//! - `class`: a class property inside a concatenation target (the native engine's
//!   multi-chunk write, U-W7), checked on all three backends.
//!
//! EXPECTATIONS (`%%meta` field 5): `right` = the IEEE 1800-2017 5.7.1 value ("All bits of
//! the unsized value shall be set to the value of the specified bit", l.3017-3018) over the
//! whole assignment context, a select leaving the other bits at the declaration's
//! `1010..` pattern; `loud` = an exit code 1 whose stderr holds every `%%err` line exactly
//! and every `%%frag` text; `obs` = a value no oracle decides, pinned as measured; `known
//! <tag>` = a value known to be wrong today and filed under `<tag>` (PRE == POST), pinned as
//! measured so the row fails, and is flipped, when the filing is fixed.
//!
//! ORACLES: iverilog 13.0 (`-g2012` + `vvp -n`) and verilator 5.054 (`--binary --timing`) on
//! 128 of the G2 cells and all 45 G3 cells (the grounding's OR_CMP.txt / G3_OR_CMP.txt):
//! every `right` value here that an oracle runs agrees with iverilog; verilator is 2-state
//! and reads 0 for `'x` / `'z`. Where neither runs a cell (a for-clause concatenation, a
//! class property in a concatenation: iverilog aborts) the value is hand-IEEE.
//!
//! CANNOT SEE: a left-side form nobody wrote a row for. `table_covers_every_site_and_form`
//! holds the site-by-form table below so a row cannot silently disappear; a new form has
//! no row until one is written (the rule's exhaustive `match` is what refuses it meanwhile).
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const CELLS: &str = include_str!("fill_matrix/cells.txt");

struct Row {
    id: String,
    group: String,
    site: String,
    form: String,
    expect: String,
    tag: String,
    outs: Vec<(String, String)>,
    errs: Vec<String>,
    frags: Vec<String>,
    src: String,
}

fn parse(text: &str) -> Vec<Row> {
    let mut blocks: BTreeMap<String, String> = BTreeMap::new();
    let mut rows = Vec::new();
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if let Some(name) = l.strip_prefix("%%block ") {
            let mut body = Vec::new();
            for b in lines.by_ref() {
                if b == "%%end" {
                    break;
                }
                body.push(b);
            }
            blocks.insert(name.to_string(), body.join("\n"));
        } else if let Some(id) = l.strip_prefix("%%cell ") {
            let mut r = Row {
                id: id.to_string(),
                group: String::new(),
                site: String::new(),
                form: String::new(),
                expect: String::new(),
                tag: String::new(),
                outs: Vec::new(),
                errs: Vec::new(),
                frags: Vec::new(),
                src: String::new(),
            };
            let mut src: Vec<String> = Vec::new();
            for b in lines.by_ref() {
                if b == "%%end" {
                    break;
                } else if let Some(m) = b.strip_prefix("%%meta ") {
                    let f: Vec<&str> = m.split('\t').collect();
                    assert_eq!(f.len(), 6, "meta of {id}");
                    r.group = f[0].to_string();
                    r.site = f[1].to_string();
                    r.form = f[2].to_string();
                    r.expect = f[4].to_string();
                    r.tag = f[5].to_string();
                } else if let Some(o) = b.strip_prefix("%%out ") {
                    let (k, v) = o.split_once('\t').expect("out label");
                    r.outs.push((k.to_string(), v.to_string()));
                } else if let Some(e) = b.strip_prefix("%%err ") {
                    r.errs.push(e.to_string());
                } else if let Some(e) = b.strip_prefix("%%frag ") {
                    r.frags.push(e.to_string());
                } else if let Some(name) = b.strip_prefix("%%use ") {
                    src.push(
                        blocks
                            .get(name)
                            .unwrap_or_else(|| panic!("block {name}"))
                            .clone(),
                    );
                } else {
                    src.push(b.to_string());
                }
            }
            r.src = src.join("\n");
            rows.push(r);
        }
    }
    rows
}

fn rows() -> Vec<Row> {
    parse(CELLS)
}

/// Run one design in its own directory; `(exit code, stdout, stderr)`.
fn run(src: &str, args: &[&str]) -> (i32, String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir: PathBuf = std::env::temp_dir().join(format!("vita_fm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Every `label=value` token of the `C:` lines.
fn printed(stdout: &str) -> Vec<(String, String)> {
    let mut t = Vec::new();
    for l in stdout.lines() {
        if let Some(rest) = l.strip_prefix("C:") {
            for tok in rest.split(' ') {
                let (k, v) = tok.split_once('=').unwrap_or((tok, ""));
                t.push((k.to_string(), v.to_string()));
            }
        }
    }
    t
}

/// `None` when the row holds; else why not.
fn verdict(r: &Row, args: &[&str]) -> Option<String> {
    let (rc, out, err) = run(&r.src, args);
    let all = format!("{out}{err}");
    match r.expect.as_str() {
        "right" | "obs" | "known" => {
            if rc != 0 || err.contains("error[VITA-") {
                return Some(format!("rc={rc}, want a run:\n{all}"));
            }
            let got = printed(&out);
            (got != r.outs).then(|| format!("printed {got:?}, want {:?}", r.outs))
        }
        "loud" => {
            if rc != 1 {
                return Some(format!("rc={rc}, want 1:\n{all}"));
            }
            for e in &r.errs {
                if !all.lines().any(|l| l == e) {
                    return Some(format!("missing line `{e}`:\n{all}"));
                }
            }
            for f in &r.frags {
                if !all.contains(f.as_str()) {
                    return Some(format!("missing text `{f}`:\n{all}"));
                }
            }
            None
        }
        other => Some(format!("unknown expectation `{other}`")),
    }
}

/// Check every row of `group` (and, when given, of a site in `sites`) on the native
/// backend, several at a time; one failure list at the end.
fn check(group: &str, sites: &[&str], backends: &[&[&str]]) {
    let all = rows();
    let pick: Vec<&Row> = all
        .iter()
        .filter(|r| r.group == group && (sites.is_empty() || sites.contains(&r.site.as_str())))
        .collect();
    assert!(!pick.is_empty(), "no rows for {group} {sites:?}");
    let next = std::sync::atomic::AtomicUsize::new(0);
    let fails = std::sync::Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(r) = pick.get(i) else { break };
                for args in backends {
                    if let Some(why) = verdict(r, args) {
                        let tag = if r.tag == "-" {
                            String::new()
                        } else {
                            format!(" [{}]", r.tag)
                        };
                        fails
                            .lock()
                            .unwrap()
                            .push(format!("{} {:?}{tag}: {why}", r.id, args));
                    }
                }
            });
        }
    });
    let mut fails = fails.into_inner().unwrap();
    fails.sort();
    assert!(
        fails.is_empty(),
        "{} of {} rows of {group} {sites:?} fail:\n{}",
        fails.len(),
        pick.len(),
        fails.join("\n")
    );
}

const NATIVE: &[&[&str]] = &[&[]];

#[test]
fn g2_site_s1_blocking() {
    check("g2", &["S1"], NATIVE);
}

#[test]
fn g2_site_s2_nonblocking() {
    check("g2", &["S2"], NATIVE);
}

#[test]
fn g2_sites_s3_s4_procedural_assign_and_force() {
    check("g2", &["S3", "S4"], NATIVE);
}

#[test]
fn g2_sites_s5_to_s8_continuous_initializers_for() {
    check("g2", &["S5", "S6", "S7", "S8"], NATIVE);
}

#[test]
fn g3_the_other_doors() {
    check("g3", &[], NATIVE);
}

#[test]
fn plan_cells() {
    check("plan", &[], NATIVE);
}

#[test]
fn judge_rows() {
    check("judge", &[], NATIVE);
}

/// The doors U2 routes through the assignment funnel: an intra-assignment event control
/// (FI1: `v8 = @(posedge clk) 'x;`, `<= @(..)`, `<= repeat (n) @(..)` incl. `repeat (0)`,
/// events.rs) and the x_merge block of a ternary whose arm calls a function with an output
/// formal (hoist/arms.rs: `x8 = c ? '1 : f4(o8);` with `c` x, IEEE 11.4.11 l.11914-11919).
#[test]
fn u2_event_controls_and_x_merge() {
    check("u2", &[], NATIVE);
}

/// A class property inside a concatenation target: the native kernel's multi-chunk write
/// routes the property's piece to the class-field store (U-W7), as the single-chunk path
/// and the `vm` / `interp` executors do. Every row on all three backends.
#[test]
fn class_property_in_a_concatenation_target_on_every_backend() {
    check(
        "class",
        &[],
        &[&[], &["--backend", "vm"], &["--backend", "interp"]],
    );
}

/// The G2 site-by-form table, held here so a row cannot drop out of the data file and a
/// new form cannot enter it unlisted: for each left-side form, the sites it has rows for.
const FORMS: &[(&str, &str)] = &[
    ("assoc_elem", "S1 S2"),
    ("bit8", "S1 S2 S7"),
    ("bits_hier_decl", "S1 S2 S6"),
    ("bitsel_const", "S1 S2"),
    ("bitsel_var", "S3"),
    ("block_local", "S6"),
    ("block_local_wide", "S6"),
    ("class_prop", "S1 S2 S8"),
    ("class_prop_wide", "S1 S2"),
    ("clocking_drive", "S2"),
    ("clocking_drive_wide", "S2"),
    ("concat_2s4s", "S1 S2"),
    ("concat_bits_hier_decl", "S1 S2"),
    ("concat_class", "S1 S2"),
    ("concat_class2", "S1"),
    ("concat_class_hier", "S1"),
    ("concat_class_last", "S1"),
    ("concat_class_wide", "S1"),
    ("concat_gen_local", "S1 S2"),
    ("concat_gen_local_first", "S1"),
    ("concat_genfor", "S1"),
    ("concat_hier", "S3"),
    ("concat_hier1", "S1 S2 S5 S8"),
    ("concat_hier1_first", "S1 S2"),
    ("concat_hier2", "S1 S2 S5 S8"),
    ("concat_hier2_nonscopefree", "S1"),
    ("concat_hier2_scopefree_op", "S1"),
    ("concat_hier2_step", "S8"),
    ("concat_hier2_ternary_read", "S1"),
    ("concat_hier2_unop", "S1"),
    ("concat_hier_2state", "S1"),
    ("concat_hier_alwayscomb", "S1"),
    ("concat_hier_bitsel", "S1 S2"),
    ("concat_hier_gen", "S1 S2"),
    ("concat_hier_in_fork", "S1"),
    ("concat_hier_in_function_call", "S1"),
    ("concat_hier_in_if", "S1"),
    ("concat_hier_in_task", "S1"),
    ("concat_hier_intra_delay", "S1"),
    ("concat_hier_nb_delay", "S2"),
    ("concat_hier_net", "S4"),
    ("concat_hier_only", "S1 S2 S5 S8"),
    ("concat_hier_partsel", "S1 S2"),
    ("concat_hier_signed", "S1"),
    ("concat_hier_struct_member", "S1"),
    ("concat_hier_unp_elem", "S1"),
    ("concat_hier_unp_elem2", "S1"),
    ("concat_hier_var", "S4"),
    ("concat_hier_w129", "S1"),
    ("concat_hier_wide", "S1 S2"),
    ("concat_iface", "S1 S2 S8"),
    ("concat_iface_hier", "S1"),
    ("concat_iface_loc", "S1 S2"),
    ("concat_instarr", "S1 S2"),
    ("concat_late", "S5"),
    ("concat_late_hier", "S1 S2"),
    ("concat_loc", "S1 S2 S3 S8"),
    ("concat_nested", "S1 S2 S5"),
    ("concat_nested_hier", "S1 S2"),
    ("concat_nets", "S4 S5"),
    ("concat_sel_loc", "S1 S2"),
    ("concat_struct_member", "S1 S2"),
    ("concat_unp_elem", "S1 S2"),
    ("concat_wide", "S1 S2"),
    ("dyn_elem", "S1 S2"),
    ("gen_local", "S1 S2"),
    ("gen_local_wide", "S1"),
    ("hier_bitsel", "S1 S2"),
    ("hier_net", "S4 S5"),
    ("hier_partsel", "S1 S2 S5"),
    ("hier_unp_elem", "S1"),
    ("hier_var", "S4"),
    ("hier_whole", "S1 S2 S3 S8"),
    ("hier_whole_nonscopefree", "S1"),
    ("hier_whole_nonscopefree_plus", "S1"),
    ("hier_whole_ternary_read", "S1"),
    ("hier_wide", "S1 S2 S3"),
    ("idx_down", "S1 S2"),
    ("idx_up", "S1 S2"),
    ("iface_var", "S5"),
    ("iface_whole", "S1 S2 S3"),
    ("instarr_whole", "S1 S2"),
    ("int2", "S1 S2 S3 S6 S7"),
    ("integer", "S1 S2 S6"),
    ("late_partsel_bits", "S1 S2 S8"),
    ("late_partsel_gen", "S1 S2"),
    ("list_second", "S6"),
    ("logic70", "S7"),
    ("logic8", "S7"),
    ("modport_concat", "S1 S2"),
    ("modport_whole", "S1"),
    ("net8", "S4 S5 S6"),
    ("net_arr_elem", "S5"),
    ("net_bitsel", "S4 S5"),
    ("net_idx_up", "S5"),
    ("net_partsel", "S4 S5"),
    ("param_width", "S1 S2"),
    ("partsel_const", "S1 S2 S8"),
    ("queue_elem", "S1 S2"),
    ("real", "S6"),
    ("signed8", "S1 S2 S6 S7"),
    ("signed_net", "S5"),
    ("stream_target", "S1 S2"),
    ("struct", "S6"),
    ("struct_member", "S1 S2"),
    ("struct_var", "S5"),
    ("struct_whole", "S1 S2 S3"),
    ("task_formal_concat", "S1"),
    ("task_inout_hier", "S1"),
    ("task_out_concat_hier", "S1"),
    ("task_out_hier", "S1"),
    ("task_out_loc", "S1"),
    ("task_ref_loc", "S1"),
    ("union", "S6"),
    ("union_member", "S1 S2"),
    ("unp_elem", "S1 S2"),
    ("unpacked_whole", "S6"),
    ("upward_concat", "S1 S2"),
    ("upward_whole", "S1"),
    ("var8", "S1 S2 S3 S4 S6 S8"),
    ("var8_step", "S8"),
    ("var_bit", "S1 S2"),
    ("var_logic", "S5"),
    ("var_partsel", "S4"),
    ("vif_concat", "S1"),
    ("vif_member", "S1 S2"),
    ("wide100", "S6"),
    ("wide129", "S1 S2 S8"),
    ("wide200", "S1 S2 S3 S5"),
    ("wide200_net", "S6"),
    ("wide65", "S1 S2"),
    ("wide_net", "S4"),
];

#[test]
fn table_covers_every_site_and_form() {
    let mut got: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for r in rows() {
        if (r.group == "g2" || r.group == "class") && r.site.starts_with('S') {
            got.entry(r.form.clone())
                .or_default()
                .insert(r.site.clone());
        }
    }
    let want: BTreeMap<String, std::collections::BTreeSet<String>> = FORMS
        .iter()
        .map(|(f, s)| (f.to_string(), s.split(' ').map(str::to_string).collect()))
        .collect();
    assert_eq!(got, want, "the data file's site-by-form table moved");
    // Every one of the eight marked sites has rows.
    let sites: std::collections::BTreeSet<String> = got.values().flatten().cloned().collect();
    assert_eq!(sites.len(), 8, "{sites:?}");
    // Every row of the file names a known group and expectation, and a `known` row a filing.
    for r in rows() {
        assert!(
            ["g2", "g3", "plan", "judge", "class", "u2"].contains(&r.group.as_str()),
            "{}: group {}",
            r.id,
            r.group
        );
        assert!(
            ["right", "obs", "known", "loud"].contains(&r.expect.as_str()),
            "{}: expect {}",
            r.id,
            r.expect
        );
        assert!(
            r.expect != "known" || r.tag != "-",
            "{}: a known-wrong row names its filing",
            r.id
        );
        assert!(
            r.expect != "loud" || !(r.errs.is_empty() && r.frags.is_empty()),
            "{}: a loud row names its text",
            r.id
        );
        assert!(
            r.expect == "loud" || !r.outs.is_empty(),
            "{}: a run row names its values",
            r.id
        );
    }
}
