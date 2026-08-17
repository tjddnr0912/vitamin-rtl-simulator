//! D2 ABSOLUTE ANCHOR — the 2-state fast lane inside tier-3's `wprog`.
//!
//! A census over the eight benchmark shapes and picorv32 measured how often an
//! expression evaluation touches x/z at all: **all eight shapes are 100%
//! definite, picorv32 is 90.1% of runs / 91.1% of ops.** So the canonical
//! evaluator's second plane is dead weight nine evaluations in ten, and `wprog`
//! now tries a one-plane lane first, bailing to the canonical loop the moment a
//! leaf brings an unknown (ROADMAP §5.1-ay).
//!
//! ⚠️⚠️ The failure this slice can have is NOT a wrong answer — the fallback IS
//! the canonical implementation, so a lane that bails too eagerly is merely
//! slow, and a lane that never fires passes every differential in the
//! repository. The failures it CAN have are two, and this file is built around
//! both:
//!
//!   1. **the lane runs when it must not** — an unknown leaf treated as a
//!      definite zero. The `p*` rows are the discriminator, because they are
//!      the only shapes whose result is PARTIALLY definite: `a | b` with
//!      `a = 8'hA5, b = 8'hxC` is `Xd`, and a lane that read x as 0 would print
//!      the fully definite `ad`. An all-x result (the `x*` rows) cannot tell
//!      the two apart, which is why those rows are not the anchor.
//!
//!   1b. **the lane runs on an unknown ARRAY ELEMENT.** `Load` and `LoadIdx`
//!      are different leaves and the second one reads through a runtime index,
//!      so its element check is a SEPARATE line — and a mutation that deletes
//!      it survived the whole suite, because no design in this repository held
//!      x in an array element and then read it through a runtime index on the
//!      native backend. The `g*` rows are that shape: `mem[1] = 8'hx5` read as
//!      `mem[i]` gives `x5`, and a lane that trusted the value plane would
//!      print the definite `05`.
//!
//!   2. **the bail double-reports** — `LoadIdx` out of range is the one arm in
//!      either loop with a side effect (it counts a deferred range diagnostic),
//!      so the fast lane must bail BEFORE reporting and let the canonical loop
//!      file it once. `errors=2` for two out-of-range reads is that assertion;
//!      a bail placed one line later prints 4.
//!
//! Values are pinned to iverilog 13.0 and compared across all three backends —
//! the VM and the interpreter have no such lane, so this differential is live
//! rather than comparing native to itself.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The design: the SAME expressions evaluated with x operands, then with
/// definite ones, then with partially-x ones, then definite again — so both
/// lanes run over one body and the lane has to switch back and forth.
const SRC: &str = "module top;\n\
  reg [7:0] a, b, r;\n\
  reg [7:0] mem [0:3];\n\
  integer i;\n\
  initial begin\n\
    r = (a ^ b) + (a & ~b);       $display(\"x1=%h\", r);\n\
    r = (a < b) ? a : b;          $display(\"x2=%h\", r);\n\
    a = 8'hA5; b = 8'h3C;\n\
    r = (a ^ b) + (a & ~b);       $display(\"d1=%h\", r);\n\
    r = (a < b) ? a : b;          $display(\"d2=%h\", r);\n\
    b = 8'hxC;\n\
    r = a | b;                    $display(\"p1=%h\", r);\n\
    r = a & b;                    $display(\"p2=%h\", r);\n\
    r = {a[3:0], b[7:4]};         $display(\"p3=%h\", r);\n\
    b = 8'h3C;\n\
    r = a | b;                    $display(\"d3=%h\", r);\n\
    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0] + 8'h10;\n\
    r = mem[2];                   $display(\"m1=%h\", r);\n\
    mem[1] = 8'hx5;\n\
    i = 1;\n\
    r = mem[i] | 8'h00;           $display(\"g1=%h\", r);\n\
    r = mem[i] & 8'hF0;           $display(\"g2=%h\", r);\n\
    i = 7;\n\
    r = mem[i];                   $display(\"m2=%h\", r);\n\
    r = mem[i];                   $display(\"m3=%h\", r);\n\
    $finish;\n\
  end\n\
endmodule\n";

/// iverilog 13.0, verbatim.
///
/// `p1`/`p2` are the rows that matter: an `X` hex digit beside a definite one.
/// `p3` is the same claim through the structural ops (`Slice` + `Splice`), where
/// the unknown lives in the HIGH half and the definite bits in the low.
const WANT: &str = "x1=xx\n\
                    x2=xx\n\
                    d1=1a\n\
                    d2=3c\n\
                    p1=Xd\n\
                    p2=X4\n\
                    p3=5x\n\
                    d3=bd\n\
                    m1=12\n\
                    g1=x5\n\
                    g2=x0\n\
                    m2=xx\n\
                    m3=xx\n";

fn run_on(backend: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_2s_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, SRC).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let body: String = txt
        .lines()
        .filter(|l| l.contains('='))
        .filter(|l| !l.starts_with("errors="))
        .fold(String::new(), |mut a, l| {
            a.push_str(l);
            a.push('\n');
            a
        });
    let diag = String::from_utf8_lossy(&out.stderr).into_owned();
    (body, format!("{rj}\n---STDERR---\n{diag}"))
}

#[test]
fn the_two_state_lane_agrees_with_iverilog_on_every_backend() {
    let (native, rj) = run_on("native");
    assert!(rj.contains("\"backend\": \"native\""), "not native:\n{rj}");
    assert_eq!(native, WANT, "the 2-state lane disagrees with iverilog");
    for b in ["vm", "interp"] {
        let (other, _) = run_on(b);
        assert_eq!(other, WANT, "backend {b} disagrees");
    }
}

#[test]
fn an_out_of_range_read_is_reported_exactly_once_per_evaluation() {
    // ⚠️ THE bail-order assertion. The fast lane meets the out-of-range index
    // first; if it reported and then handed the op to the canonical loop, this
    // would be 4. Asserted on all three backends so the number is pinned as a
    // property of the language, not of the lane.
    for b in ["native", "vm", "interp"] {
        let (_, side) = run_on(b);
        let n = side.matches("VITA-E4002").count();
        assert_eq!(
            n, 2,
            "backend {b} filed {n} range reports for two out-of-range reads\n{side}"
        );
    }
}
