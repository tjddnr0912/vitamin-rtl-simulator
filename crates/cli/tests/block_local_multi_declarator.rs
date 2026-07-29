//! R16 §3.5: a comma declaration is N INDEPENDENT declarators.
//!
//! The block-local collision check read `d.names.first()` and applied that one
//! verdict to every name in the declaration. So in
//!
//! ```systemverilog
//! int n;                                  // module net
//! begin automatic int n = 0, n_skip = 0;  // only `n` collides
//! ```
//!
//! `n_skip` was rejected too, with a message asserting it "collides with an existing
//! net of the same name" — a net that exists nowhere in the design — and then the
//! whole declaration was dropped, so every later use reported E3010 "undeclared
//! net/variable" one line below the declaration that declared it. One module net
//! produced eight diagnostics, seven of them about a variable with nothing wrong.
//!
//! Measured at 6b6b8ef: 8 diagnostics for the two-declarator form, 11 for three, and
//! an order dependence (a collision in the SECOND declarator did not leak). All of
//! them are now the single genuine diagnostic, and the order dependence is gone.
//!
//! The fix splits a declaration only when its declarators DISAGREE about colliding.
//! A declaration where all collide, or none do, takes the original path unchanged.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bmld_{}_{n}", std::process::id()));
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
    (text, out.status.success())
}

fn errors(o: &str) -> Vec<String> {
    o.lines()
        .filter(|l| l.contains("error["))
        .map(|s| s.to_string())
        .collect()
}

/// Exactly one diagnostic, and it names `who` — no leak onto the clean declarators,
/// no E3010 cascade.
fn one_error_naming(src: &str, who: &str) {
    let (o, ok) = run(src);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    let e = errors(&o);
    assert_eq!(e.len(), 1, "expected exactly one diagnostic, got:\n{o}");
    assert!(e[0].contains(&format!("`{who}`")), "wrong subject:\n{o}");
    assert!(
        !o.contains("E3010"),
        "a dropped declarator produced an undeclared-name cascade:\n{o}"
    );
}

const MODULE_NET: &str = "module t;\n  int n;\n";

/// The report's reproducer. 8 diagnostics at 6b6b8ef.
#[test]
fn collision_in_the_first_declarator_does_not_leak() {
    one_error_naming(
        &format!(
            "{MODULE_NET}\
             initial begin begin automatic int n = 0, n_skip = 0;\n\
               n_skip++; n_skip++; if (n_skip == 2) $display(\"R PASS\");\n\
             end end\n\
           endmodule"
        ),
        "n",
    );
}

/// The report's order dependence: a collision in the SECOND declarator did not leak,
/// which is why the same code moved one word over behaved differently.
#[test]
fn collision_in_a_later_declarator_behaves_the_same() {
    one_error_naming(
        &format!(
            "{MODULE_NET}\
             initial begin begin automatic int n_skip = 0, n = 0;\n\
               n_skip++; n_skip++; if (n_skip == 2) $display(\"R PASS\");\n\
             end end\n\
           endmodule"
        ),
        "n",
    );
}

/// Three declarators, one colliding — 11 diagnostics at 6b6b8ef.
#[test]
fn two_clean_declarators_survive_one_collision() {
    one_error_naming(
        &format!(
            "{MODULE_NET}\
             initial begin begin automatic int n = 0, a2 = 0, a3 = 0;\n\
               a2++; a3++; if (a2 == 1 && a3 == 1) $display(\"R PASS\");\n\
             end end\n\
           endmodule"
        ),
        "n",
    );
}

/// The report's PASS boundary: renaming only the first declarator made it work, and
/// still does.
#[test]
fn no_collision_still_runs() {
    let (o, ok) = run(&format!(
        "{MODULE_NET}\
         initial begin begin automatic int m = 0, n_skip = 0;\n\
           n_skip++; n_skip++; if (n_skip == 2) $display(\"R PASS\");\n\
         end end\n\
       endmodule"
    ));
    assert!(ok, "expected acceptance, got:\n{o}");
    assert!(o.contains("R PASS"), "expected R PASS, got:\n{o}");
}

/// SOUNDNESS PIN. When EVERY declarator collides, every one is still reported — the
/// split must not silence a real collision, only stop it spreading.
#[test]
fn all_declarators_colliding_are_all_reported() {
    let (o, ok) = run("module t;\n  int n; int m;\n\
         initial begin begin automatic int n = 0, m = 0;\n\
           n++; m++; $display(\"R %0d %0d\", n, m);\n\
         end end\n\
       endmodule");
    assert!(!ok, "expected diagnostics, got acceptance:\n{o}");
    let e = errors(&o);
    assert_eq!(e.len(), 2, "expected one diagnostic per name, got:\n{o}");
    assert!(e.iter().any(|l| l.contains("`n`")), "missing `n`:\n{o}");
    assert!(e.iter().any(|l| l.contains("`m`")), "missing `m`:\n{o}");
}

/// A multi-declarator block-local with NO collision at all is untouched by the split
/// and behaves exactly as a sequence of single declarations would.
#[test]
fn plain_multi_declarator_block_local_is_unaffected() {
    let (o, ok) = run("module t;\n\
         initial begin begin automatic int a = 1, b = 2, c = 3;\n\
           $display(\"R %0d %0d %0d\", a, b, c);\n\
         end end\n\
       endmodule");
    assert!(ok, "expected acceptance, got:\n{o}");
    assert!(o.contains("R 1 2 3"), "expected R 1 2 3, got:\n{o}");
}
