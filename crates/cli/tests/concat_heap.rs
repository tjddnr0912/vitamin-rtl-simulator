//! A8-concat ABSOLUTE ANCHOR — a heap-kind chunk inside a CONCAT lvalue, on
//! tier-3.
//!
//! V1 slice 2 found this shape by flipping the default backend over the whole
//! suite: `NativeKernel::write_routed` routed on `if let [c] = lhs.chunks…`,
//! i.e. only when the WHOLE lvalue is one chunk, while the engine routes PER
//! CHUNK inside `write_chunk`. So `{d[0], x} = …` sent both halves to the arena
//! and the heap one reached `assert_owns`. That slice REFUSED the shape and
//! wrote down the follow-on rather than the fix: "the split rule already lives
//! in `NetArena::write_lvalue`; a second spelling of it in the router is the
//! §4.5.279 class of defect … give the funnel a per-chunk escape."
//!
//! This is that escape. The slicing stays in one place and runs once; the router
//! only says which pieces are not its store's.
//!
//! ⚠️ NO IVERILOG ORACLE. `iverilog -g2012` does not compile this shape — it
//! aborts inside `ivl` on an internal assert (`ivl_stmt_lvals(net) == 1`,
//! `show_stmt_assign_sig_darray`). The values below are hand-IEEE: §11.4.12
//! assigns a concat lvalue MSB-first, so the source's top bits go to the leftmost
//! chunk. Each line states the split it pins.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_native(src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cchp_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    (String::from_utf8_lossy(&out.stdout).into_owned(), rj)
}

/// Three splits, each pinning a different half of the escape:
///
///   * `{d[0], x} = 36'h12345678A` — the heap chunk is the LEFT one, so it takes
///     the source's top 32 bits and `x` takes the low 4. Get the escape's slice
///     order wrong and the two swap.
///   * `{x, d[1]} = 36'hABCDEF012` — the heap chunk is the RIGHT one, which is
///     the case where the arena writes first and the escaped piece is applied
///     after. That deferral is what makes the collect sound, and this is the row
///     that would catch it being applied to the wrong chunk index.
///   * `{d[0], y} = 40'h99AABBCCDD` — an 8-bit flat chunk, so the boundary is
///     not the same as either row above.
///
/// ⚠️ ANTI-VACUITY: run.json must say the run was native. Before this slice the
/// design was refused by the STORAGE gate and fell back to the VM, where all
/// three lines already passed.
#[test]
fn concat_with_a_dyn_chunk_splits_msb_first_on_tier_3() {
    let (out, rj) = run_native(
        "module top;\n\
           int d [];\n\
           reg [3:0] x;\n\
           reg [7:0] y;\n\
           initial begin\n\
             d = new[2];\n\
             d[0] = 0; d[1] = 0;\n\
             {d[0], x} = 36'h12345678A;\n\
             $display(\"a d0=%0h x=%0h\", d[0], x);\n\
             {x, d[1]} = 36'hABCDEF012;\n\
             $display(\"b x=%0h d1=%0h\", x, d[1]);\n\
             {d[0], y} = 40'h99AABBCCDD;\n\
             $display(\"c d0=%0h y=%0h\", d[0], y);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{out}"
    );
    let mut body = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        body.push_str(l);
        body.push('\n');
    }
    assert_eq!(
        body,
        "a d0=12345678 x=a\n\
         b x=a d1=bcdef012\n\
         c d0=99aabbcc y=dd\n",
        "hand-IEEE §11.4.12: a concat lvalue takes the source MSB-first"
    );
}

/// The NEIGHBOUR that must stay loud. An assoc chunk is admitted by the gate now
/// too, but its key cannot ride the `(offset, word)` pairs the split produces —
/// so the write is IGNORED with `W4020`, on both backends, and that is the
/// engine's own behaviour rather than something this slice introduced.
///
/// Pinned because "admitted by the gate" and "written" are different claims, and
/// a slice that opened a row is exactly when they get conflated.
#[test]
fn concat_with_an_assoc_chunk_stays_loud() {
    let (out, rj) = run_native(
        "module top;\n\
           int aa [int];\n\
           reg [3:0] x;\n\
           initial begin\n\
             {aa[5], x} = 8'hAB;\n\
             $display(\"a aa5=%0d x=%h\", aa[5], x);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{out}"
    );
    assert!(
        out.contains("a aa5=x x=b"),
        "the flat chunk lands and the assoc one does not:\n{out}"
    );
}
