//! A3-ii-b ABSOLUTE ANCHOR — a task frame that PARKS, on tier-3, iverilog-pinned.
//!
//! Two gate rows refused this: the storage row ("a task frame that SUSPENDS
//! (delay, wait or fork inside the body)") and the executor row ("a call
//! statement whose callee suspends"). What they refused was not a shape the walk
//! could not execute — A3-ii-a already drove frame CFGs — but a LIFETIME: the
//! walk kept its open frames in a local `Vec`, so a `Delay`/`Wait` inside one
//! returned `Step::Suspended` and dropped the stack on the floor.
//!
//! The slice gives that stack somewhere to live across the suspension and hands
//! the WINDOW half to the engine's own `frame_window::{stash,restore}_windows_in`
//! (extracted, so the pop order has one spelling rather than two).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Every line is a discriminator for one piece of the park:
///
///   * `acc` parks on `@(posedge clk)` INSIDE A LOOP and keeps an automatic
///     local (`loc`) across each park — the window stash. Two processes run it
///     CONCURRENTLY with different seeds and different edge counts, which is the
///     only shape that can catch the stash popping the wrong window: get it
///     wrong and A resumes onto B's `loc`.
///   * its `res` is an OUTPUT FORMAL, so the copy-out has to survive the park
///     too (`out_binds` lives in the parked frame, not on the Rust stack).
///   * `outer` calls `inner`, and BOTH park — a nested parking frame, i.e. the
///     stack has to be saved at depth 2 and resumed innermost-first.
///   * `wq` parks on `#1` and then hits a `wait (flag)` that is ALREADY TRUE, so
///     it falls THROUGH rather than suspending — and that fall-through has to
///     move the FRAME's pc, not the process's. It was written as `bb = *resume`
///     because a frame could not reach a `Wait` at all before this slice; left
///     that way the walk keeps re-fetching the same block and the design dies on
///     the step guard. Added because the mutation battery found no other design
///     in the suite exercising it.
///   * the four prints are at DIFFERENT times (t=1, t=3, t=4, t=7), deliberately:
///     two `initial` blocks that finish at the same instant have no
///     IEEE-defined order, and iverilog and vita legitimately differ there. A
///     known divergence in an anchor stops it being an anchor.
///
/// ⚠️ ANTI-VACUITY: run.json must say the design actually ran natively. Every
/// line below already passed on the VM, which is exactly what a refused design
/// falls back to.
#[test]
fn parking_frames_on_tier_3_match_iverilog() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_park_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           reg clk = 0;\n\
           reg [7:0] a, b, c, d;\n\
           reg flag = 0;\n\
           always #1 clk = ~clk;\n\
           task automatic acc(input [7:0] seed, input integer edges, output [7:0] res);\n\
             reg [7:0] loc;\n\
             integer i;\n\
             begin\n\
               loc = seed;\n\
               for (i = 0; i < edges; i = i + 1) begin\n\
                 @(posedge clk);\n\
                 loc = loc + 8'd1;\n\
               end\n\
               res = loc;\n\
             end\n\
           endtask\n\
           task automatic inner(input [7:0] x, output [7:0] y);\n\
             begin #2; y = x + 8'd7; end\n\
           endtask\n\
           task automatic outer(input [7:0] x, output [7:0] y);\n\
             reg [7:0] mid;\n\
             begin\n\
               #1;\n\
               inner(x, mid);\n\
               #1;\n\
               y = mid + 8'd1;\n\
             end\n\
           endtask\n\
           task automatic wq(input [7:0] x, output [7:0] y);\n\
             begin\n\
               #1;\n\
               wait (flag);\n\
               y = x + 8'd2;\n\
             end\n\
           endtask\n\
           initial begin flag = 1'b1; wq(8'd20, d); $display(\"D d=%0d t=%0t\", d, $time); end\n\
           initial begin acc(8'd10,  2, a); $display(\"A a=%0d t=%0t\", a, $time); end\n\
           initial begin acc(8'd100, 4, b); $display(\"B b=%0d t=%0t\", b, $time); end\n\
           initial begin outer(8'd5, c);    $display(\"C c=%0d t=%0t\", c, $time); end\n\
           initial #20 $finish;\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{txt}"
    );
    let mut body = String::new();
    for l in txt.lines().filter(|l| !l.starts_with("simulation ended")) {
        body.push_str(l);
        body.push('\n');
    }
    assert_eq!(
        body,
        "D d=22 t=1\n\
         A a=12 t=3\n\
         C c=13 t=4\n\
         B b=104 t=7\n",
        "iverilog-pinned parking-frame behaviour on tier-3"
    );
}
