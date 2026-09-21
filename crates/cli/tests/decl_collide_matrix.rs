//! The KIND-PAIR MATRIX: every unordered pair of the fourteen binders
//! `decl_collide.rs` collects, measured once against both oracles and pinned here.
//!
//! 105 pairs (same-kind included): 100 both oracles REJECT, 2 both RUN
//! (a non-ANSI port beside its own `wire` / `logic` declaration), 3 the oracles
//! SPLIT (a port beside a parameter, a localparam or a genvar — iverilog rejects,
//! verilator runs). vita runs all 105 CONTROLS, so no pair is undecidable.
//!
//! The pairs live as a table rather than as 105 hand-written designs because the
//! defect this file exists for is a pair NOBODY measured: the list was widened one
//! measured pair at a time, and every new kind arrived with an unmeasured column.
//! A table is the only shape in which adding a kind forces the whole row.
//!
//! Designs, generator and verbatim 3-tool logs: `impl/matrix/` (see the module doc
//! of `crates/elaborate/src/decl_collide.rs` for the rendered matrix).
//!
//! ORACLES: iverilog 13.0 (`-g2012`), Verilator 5.052 (`--binary --timing`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mx_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (out.status.code().unwrap_or(-1), text)
}

/// One declaration of `name`, as `(header port names, body items, top-level items)`
/// — the Rust twin of `impl/matrix/gen.py`, so the designs the tests build are the
/// designs the oracles were run on.
fn snippet(kind: &str, name: &str, u: usize) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (p, b, t): (Vec<String>, Vec<String>, Vec<String>) = match kind {
        "function" => (
            vec![],
            vec![format!("function int {name}(); {name} = 44; endfunction")],
            vec![],
        ),
        "task" => (
            vec![],
            vec![format!("task {name}; begin end endtask")],
            vec![],
        ),
        "net" => (vec![], vec![format!("wire {name};")], vec![]),
        "variable" => (vec![], vec![format!("logic {name};")], vec![]),
        "parameter" => (vec![], vec![format!("parameter int {name} = 5;")], vec![]),
        "localparam" => (vec![], vec![format!("localparam int {name} = 5;")], vec![]),
        "port" => (
            vec![name.to_string()],
            vec![format!("input {name};")],
            vec![],
        ),
        "genvar" => (vec![], vec![format!("genvar {name};")], vec![]),
        "instance" => (
            vec![],
            vec![format!("sub {name}();")],
            vec!["module sub; endmodule".to_string()],
        ),
        "namedblock" => (
            vec![],
            vec![format!(
                "initial begin : {name} integer z{u}; z{u} = 0; end"
            )],
            vec![],
        ),
        "generateblock" => (
            vec![],
            vec![format!(
                "generate if (1) begin : {name} wire w{u}; end endgenerate"
            )],
            vec![],
        ),
        "typedef" => (vec![], vec![format!("typedef int {name};")], vec![]),
        "class" => (vec![], vec![format!("class {name}; endclass")], vec![]),
        "enumlabel" => (
            vec![],
            vec![format!("typedef enum int {{ {name} = 7 }} et{u}_t;")],
            vec![],
        ),
        other => panic!("unknown kind {other}"),
    };
    (p, b, t)
}

fn design(ka: &str, kb: &str, na: &str, nb: &str) -> String {
    let (mut ports, mut body, mut top): (Vec<String>, Vec<String>, Vec<String>) =
        (vec![], vec![], vec![]);
    for (i, (k, n)) in [(ka, na), (kb, nb)].iter().enumerate() {
        let (p, b, t) = snippet(k, n, i);
        for x in p {
            if !ports.contains(&x) {
                ports.push(x);
            }
        }
        body.extend(b);
        for x in t {
            if !top.contains(&x) {
                top.push(x);
            }
        }
    }
    let hdr = if ports.is_empty() {
        "module top;".to_string()
    } else {
        format!("module top({});", ports.join(", "))
    };
    let mut lines = top;
    lines.push(hdr);
    for l in body {
        lines.push(format!("  {l}"));
    }
    lines.push("  initial begin #1 $finish; end".to_string());
    lines.push("endmodule".to_string());
    lines.join("\n") + "\n"
}

/// `(kind a, kind b, oracle verdict)` — `refuse` = both oracles reject, `legal` =
/// both run it, `split` = they disagree.
const MATRIX: &[(&str, &str, &str)] = &[
    ("function", "function", "refuse"),
    ("function", "task", "refuse"),
    ("function", "net", "refuse"),
    ("function", "variable", "refuse"),
    ("function", "parameter", "refuse"),
    ("function", "localparam", "refuse"),
    ("function", "port", "refuse"),
    ("function", "genvar", "refuse"),
    ("function", "instance", "refuse"),
    ("function", "namedblock", "refuse"),
    ("function", "generateblock", "refuse"),
    ("function", "typedef", "refuse"),
    ("function", "class", "refuse"),
    ("function", "enumlabel", "refuse"),
    ("task", "task", "refuse"),
    ("task", "net", "refuse"),
    ("task", "variable", "refuse"),
    ("task", "parameter", "refuse"),
    ("task", "localparam", "refuse"),
    ("task", "port", "refuse"),
    ("task", "genvar", "refuse"),
    ("task", "instance", "refuse"),
    ("task", "namedblock", "refuse"),
    ("task", "generateblock", "refuse"),
    ("task", "typedef", "refuse"),
    ("task", "class", "refuse"),
    ("task", "enumlabel", "refuse"),
    ("net", "net", "refuse"),
    ("net", "variable", "refuse"),
    ("net", "parameter", "refuse"),
    ("net", "localparam", "refuse"),
    ("net", "port", "legal"),
    ("net", "genvar", "refuse"),
    ("net", "instance", "refuse"),
    ("net", "namedblock", "refuse"),
    ("net", "generateblock", "refuse"),
    ("net", "typedef", "refuse"),
    ("net", "class", "refuse"),
    ("net", "enumlabel", "refuse"),
    ("variable", "variable", "refuse"),
    ("variable", "parameter", "refuse"),
    ("variable", "localparam", "refuse"),
    ("variable", "port", "legal"),
    ("variable", "genvar", "refuse"),
    ("variable", "instance", "refuse"),
    ("variable", "namedblock", "refuse"),
    ("variable", "generateblock", "refuse"),
    ("variable", "typedef", "refuse"),
    ("variable", "class", "refuse"),
    ("variable", "enumlabel", "refuse"),
    ("parameter", "parameter", "refuse"),
    ("parameter", "localparam", "refuse"),
    ("parameter", "port", "split"),
    ("parameter", "genvar", "refuse"),
    ("parameter", "instance", "refuse"),
    ("parameter", "namedblock", "refuse"),
    ("parameter", "generateblock", "refuse"),
    ("parameter", "typedef", "refuse"),
    ("parameter", "class", "refuse"),
    ("parameter", "enumlabel", "refuse"),
    ("localparam", "localparam", "refuse"),
    ("localparam", "port", "split"),
    ("localparam", "genvar", "refuse"),
    ("localparam", "instance", "refuse"),
    ("localparam", "namedblock", "refuse"),
    ("localparam", "generateblock", "refuse"),
    ("localparam", "typedef", "refuse"),
    ("localparam", "class", "refuse"),
    ("localparam", "enumlabel", "refuse"),
    ("port", "port", "refuse"),
    ("port", "genvar", "split"),
    ("port", "instance", "refuse"),
    ("port", "namedblock", "refuse"),
    ("port", "generateblock", "refuse"),
    ("port", "typedef", "refuse"),
    ("port", "class", "refuse"),
    ("port", "enumlabel", "refuse"),
    ("genvar", "genvar", "refuse"),
    ("genvar", "instance", "refuse"),
    ("genvar", "namedblock", "refuse"),
    ("genvar", "generateblock", "refuse"),
    ("genvar", "typedef", "refuse"),
    ("genvar", "class", "refuse"),
    ("genvar", "enumlabel", "refuse"),
    ("instance", "instance", "refuse"),
    ("instance", "namedblock", "refuse"),
    ("instance", "generateblock", "refuse"),
    ("instance", "typedef", "refuse"),
    ("instance", "class", "refuse"),
    ("instance", "enumlabel", "refuse"),
    ("namedblock", "namedblock", "refuse"),
    ("namedblock", "generateblock", "refuse"),
    ("namedblock", "typedef", "refuse"),
    ("namedblock", "class", "refuse"),
    ("namedblock", "enumlabel", "refuse"),
    ("generateblock", "generateblock", "refuse"),
    ("generateblock", "typedef", "refuse"),
    ("generateblock", "class", "refuse"),
    ("generateblock", "enumlabel", "refuse"),
    ("typedef", "typedef", "refuse"),
    ("typedef", "class", "refuse"),
    ("typedef", "enumlabel", "refuse"),
    ("class", "class", "refuse"),
    ("class", "enumlabel", "refuse"),
    ("enumlabel", "enumlabel", "refuse"),
];

/// The two pairs another guard already reports. Refusing them HERE as well would be
/// a second report of one defect, so the matrix's `R` is answered by `add_net`
/// (`net/variable … redeclared`) and by `param_dup.rs` (the §6.20.1 sentence).
fn owned_elsewhere(a: &str, b: &str) -> bool {
    let storage = |k: &str| k == "net" || k == "variable";
    let param = |k: &str| k == "parameter" || k == "localparam";
    (storage(a) && storage(b)) || (param(a) && param(b))
}

#[test]
fn every_measured_pair_lands_where_the_oracles_put_it() {
    assert_eq!(MATRIX.len(), 105, "every unordered pair of the 14 kinds");
    let mut refused = 0;
    let mut legal = 0;
    let mut split = 0;
    for (a, b, verdict) in MATRIX {
        let (rc, out) = run(&design(a, b, "nn", "nn"));
        match *verdict {
            "refuse" if owned_elsewhere(a, b) => {
                refused += 1;
                assert_eq!(rc, 1, "{a}+{b}: still refused, by its own guard:\n{out}");
                assert!(
                    out.contains("redeclared (duplicate declaration)")
                        || out.contains("duplicate declaration of parameter"),
                    "{a}+{b}: the guard that owns the pair reports it:\n{out}"
                );
                assert!(
                    !out.contains("is declared twice in this"),
                    "{a}+{b}: and §3.13 must not report it a second time:\n{out}"
                );
            }
            "refuse" => {
                refused += 1;
                assert_eq!(rc, 1, "{a}+{b}: both oracles reject this:\n{out}");
                assert!(
                    out.contains("VITA-E3009") && out.contains("is declared twice in this module"),
                    "{a}+{b}: the §3.13 refusal:\n{out}"
                );
            }
            "legal" => {
                legal += 1;
                assert_eq!(rc, 0, "{a}+{b}: both oracles RUN this:\n{out}");
            }
            "split" => {
                split += 1;
                assert_eq!(
                    rc, 0,
                    "{a}+{b}: the oracles disagree — leave it running:\n{out}"
                );
            }
            other => panic!("unknown verdict {other}"),
        }
    }
    assert_eq!((refused, legal, split), (100, 2, 3), "the measured matrix");
}

#[test]
fn every_pair_runs_when_the_two_names_differ() {
    // THE control set, and the whole reason the matrix can be trusted: for each of the
    // 105 pairs, the same two declarations under DIFFERENT names must run. A pair
    // whose control vita cannot run could not be judged at all.
    for (a, b, _) in MATRIX {
        let (rc, out) = run(&design(a, b, "aa", "bb"));
        assert_eq!(rc, 0, "{a}+{b} control (two names) must run:\n{out}");
    }
}
