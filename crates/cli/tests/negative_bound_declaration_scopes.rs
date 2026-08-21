//! A negative declared bound sizes a vector `|msb-lsb|+1` in EVERY declaration scope,
//! not only at module scope (§4.5.359).
//!
//! §4.5.350 fixed `logic [-3:0] v` for a module-scope net and left the other sites on
//! the old warn-and-clamp, deliberately: the width opt-in (`range_to_dims_opt`) and the
//! select-normalisation record (`net_decl_asc_lsb` / `net_decl_neg_lsb`) have to be
//! turned on TOGETHER, or the net is wide while its selects address the wrong bits, and
//! the sites that create nets are not in one place. Its adversarial review measured
//! which ones were left; this slice wires them.
//!
//! Every row below read `w=1` at exit 0 under `W-ELAB-FEATURE-LIMIT`, where both
//! oracles read `w=4`:
//!
//! | scope                                   | site                          |
//! |-----------------------------------------|-------------------------------|
//! | `module s(input logic [-3:0] p)`        | `ports.rs`                    |
//! | `function automatic` local              | `frames_reserve.rs`           |
//! | `task automatic` local                  | `frames_reserve.rs` (same)    |
//! | `function automatic` formal             | `frames_reserve.rs`           |
//! | `function automatic logic [-3:0] h()`   | `frames_reserve.rs` + return  |
//! | static `task` local                     | `inline_task.rs`              |
//!
//! ⭐ THE FIX IS ONE FUNCTION, NOT SIX COPIES. `record_declared_bounds_for` is the
//! twenty lines that used to be inline in the module-scope declaration; each site now
//! asks `declared_odd_bound` for the opt-in and calls it with the net it just made.
//! Copying the block six times would have been six spellings of one rule, and the next
//! declaration site would have been the seventh to forget it.
//!
//! ⚠️ ONE SCOPE IS DELIBERATELY LEFT: a CLASS PROPERTY. A class field is not a net — it
//! is a `ClassField` materialised into a heap slot — and the normalisation maps are
//! keyed by NetId, so there is no place to record against. Turning the width on alone is
//! precisely the descent §4.5.350's review caught (wide storage, un-normalised selects),
//! so it stays clamped and is recorded in ROADMAP §2-3b with its prerequisite: a
//! field-keyed normalisation, not a net-keyed one. iverilog cannot run that shape at all
//! (`ivl_type_packed_msb >= 0` assertion), so it is also the one row with a single
//! oracle.
//!
//! ORACLES: iverilog 13.0 for every row here; the rule itself (`|msb-lsb|+1` whatever
//! the signs, IEEE §7.4.2) is pinned by both oracles on the module-scope rows.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_nbds_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "expected success:\n{src}\n{all}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.strip_prefix("r=").map(str::to_string))
        .unwrap_or_else(|| panic!("no r= line:\n{src}\n{all}"))
}

#[test]
fn a_port_takes_the_declared_width() {
    assert_eq!(
        run("module s(input logic [-3:0] p);\n\
             initial begin #1 $display(\"r=%b w=%0d\", p, $bits(p)); end\nendmodule\n\
             module t; logic [7:0] x = 8'hA; s u(.p(x));\n\
             initial begin #2 $finish; end endmodule\n"),
        "1010 w=4"
    );
}

#[test]
fn a_subprogram_local_takes_the_declared_width() {
    // automatic (frame) and static (inline) are SEPARATE collectors — the static task
    // local was a sixth site, found only because the census ran both spellings.
    for (kw, sub) in [
        ("function automatic int f();", "f()"),
        ("function int f();", "f()"),
    ] {
        assert_eq!(
            run(&format!(
                "module t;\n  {kw} logic [-3:0] v; v = 4'hA; return $bits(v); endfunction\n\
                 initial begin #1 $display(\"r=%0d\", {sub}); $finish; end\nendmodule\n"
            )),
            "4",
            "for `{kw}`"
        );
    }
    for kw in ["task automatic g(output int w);", "task g(output int w);"] {
        assert_eq!(
            run(&format!(
                "module t;\n  {kw} logic [-3:0] v; v = 4'hA; w = $bits(v); endtask\n\
                 int w;\n  initial begin g(w); #1 $display(\"r=%0d\", w); $finish; end\nendmodule\n"
            )),
            "4",
            "for `{kw}`"
        );
    }
}

#[test]
fn a_function_formal_and_return_take_the_declared_width() {
    for kw in ["function automatic", "function"] {
        assert_eq!(
            run(&format!(
                "module t;\n  {kw} int k(input logic [-3:0] a); return $bits(a); endfunction\n\
                 initial begin #1 $display(\"r=%0d\", k(4'hA)); $finish; end\nendmodule\n"
            )),
            "4",
            "formal, {kw}"
        );
        assert_eq!(
            run(&format!(
                "module t;\n  {kw} logic [-3:0] h(); return 4'hA; endfunction\n\
                 initial begin #1 $display(\"r=%b\", h()); $finish; end\nendmodule\n"
            )),
            "1010",
            "return, {kw}"
        );
    }
}

#[test]
fn the_scopes_that_already_worked_still_work() {
    // §4.5.350 fixed these; they are the reference the new sites are supposed to match,
    // and the control that says the shared helper did not change them.
    assert_eq!(
        run("module t;\n  logic [-3:0] v;\n\
             initial begin v = 4'hA; #1 $display(\"r=%b\", v); $finish; end\nendmodule\n"),
        "1010"
    );
    assert_eq!(
        run("interface I; logic [-3:0] v; endinterface\n\
             module t; I i();\n\
             initial begin i.v = 4'hA; #1 $display(\"r=%b\", i.v); $finish; end endmodule\n"),
        "1010"
    );
    assert_eq!(
        run("module t;\n  logic [-3:0] m [0:1];\n\
             initial begin m[0] = 4'hA; #1 $display(\"r=%b\", m[0]); $finish; end\nendmodule\n"),
        "1010"
    );
    assert_eq!(
        run(
            "module t;\n  initial begin : b\n    logic [-3:0] v; v = 4'hA;\n\
             #1 $display(\"r=%b\", v); $finish; end\nendmodule\n"
        ),
        "1010"
    );
}

#[test]
fn an_ordinary_range_is_untouched_in_every_scope() {
    // `declared_odd_bound` answers None here, so `record_declared_bounds_for` is a no-op
    // and every one of these sites keeps the byte-identical clamp-free path it had.
    assert_eq!(
        run("module s(input logic [3:0] p);\n\
             initial begin #1 $display(\"r=%b w=%0d\", p, $bits(p)); end\nendmodule\n\
             module t; logic [7:0] x = 8'hA; s u(.p(x));\n\
             initial begin #2 $finish; end endmodule\n"),
        "1010 w=4"
    );
    assert_eq!(
        run(
            "module t;\n  function automatic logic [3:0] h(); return 4'hA; endfunction\n\
             initial begin #1 $display(\"r=%b\", h()); $finish; end\nendmodule\n"
        ),
        "1010"
    );
    // Ascending non-negative (`[0:3]`) is a different shape again and must not be
    // dragged in: `declared_asc_lsb` requires a NEGATIVE msb.
    assert_eq!(
        run(
            "module t;\n  function automatic int f(); logic [0:3] v; v = 4'hA;\n\
             return $bits(v); endfunction\n\
             initial begin #1 $display(\"r=%0d\", f()); $finish; end\nendmodule\n"
        ),
        "4"
    );
}

#[test]
fn a_descending_negative_low_bound_works_in_the_new_scopes_too() {
    // `[3:-2]` is the OTHER direction — `declared_neg_lsb`, the mirror map. The shared
    // helper routes both, so wiring a site turns on both spellings at once.
    assert_eq!(
        run(
            "module t;\n  function automatic int f(); logic [3:-2] v; v = 6'h2A;\n\
             return $bits(v); endfunction\n\
             initial begin #1 $display(\"r=%0d\", f()); $finish; end\nendmodule\n"
        ),
        "6"
    );
    assert_eq!(
        run("module s(input logic [3:-2] p);\n\
             initial begin #1 $display(\"r=%0d\", $bits(p)); end\nendmodule\n\
             module t; logic [7:0] x = 8'h2A; s u(.p(x));\n\
             initial begin #2 $finish; end endmodule\n"),
        "6"
    );
}
