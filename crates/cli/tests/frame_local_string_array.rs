//! T1-7: a `string` ARRAY as a task/function BODY-LOCAL — `string s[2]` / `string s[]`
//! declared inside a `task automatic` or a framed function.
//!
//! A frame-local scalar `string` already worked (§4.5.167, a slab-stored
//! `NetKind::String`) and a frame-local `int a[2]` already worked (an md-packed slot),
//! but neither representation fits a string CONTAINER: a string has no packed width, so
//! `count * elem_w` is meaningless for it.
//!
//! §4.5.171 gave frame locals a real `DynArray` heap handle with a per-activation
//! lifecycle, and guarded it to bit-vector elements after a measured regression — a
//! frame-local string shares `dyn_is_handle` with a heap array while being slab-stored.
//! That guard is what this lifts, for the ELEMENT type only: the container is the same
//! heap handle, and `string_elem_dyn_nets` is what makes the engine hold its elements as
//! byte strings. A frame-local scalar string never reaches that code (it has no unpacked
//! dimension), so the two cannot be confused.
//!
//! The FIXED form additionally needs its `new[n]` pre-size at FRAME ENTRY — the
//! module-scope twin gets one from the t0 var-init flush, which a frame local never
//! reaches.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn compile(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_flsa_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            }),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, ok) = compile(src);
    assert!(ok, "expected success for:\n{src}");
    out
}

fn loud(src: &str) -> bool {
    !compile(src).1
}

#[test]
fn fixed_string_array_in_a_task_body() {
    // iverilog: aa
    assert_eq!(
        run("module m;\n\
             task automatic tk; string s[2]; s[0]=\"aa\"; $display(\"%s\", s[0]); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "aa\n"
    );
}

#[test]
fn dynamic_string_array_in_a_task_body() {
    // iverilog: 3 aa bb cc / aa.bb.cc.
    assert_eq!(
        run("module m;\n\
             task automatic tk; string s[]; s=new[3]; s[0]=\"aa\"; s[1]=\"bb\"; s[2]=\"cc\";\n\
               $display(\"%0d %s %s %s\", s.size(), s[0], s[1], s[2]);\n\
               foreach(s[j]) $write(\"%s.\", s[j]); $display(\"\"); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "3 aa bb cc\naa.bb.cc.\n"
    );
}

#[test]
fn each_activation_gets_its_own_array() {
    // The pre-size runs at FRAME ENTRY, so a second call must start from a fresh
    // container rather than inheriting the first call's elements. (hand-IEEE: iverilog
    // rejects a `string` formal here, but the per-call isolation is the point.)
    assert_eq!(
        run("module m;\n\
             task automatic tk(input string tag); string s[2];\n\
               s[0]={tag,\"0\"}; s[1]={tag,\"1\"};\n\
               $display(\"%s|%s|%0d\", s[0], s[1], s.size()); endtask\n\
             initial begin tk(\"a\"); tk(\"b\"); $finish; end\n\
             endmodule\n"),
        "a0|a1|2\nb0|b1|2\n"
    );
}

#[test]
fn foreach_and_a_runtime_index_over_a_frame_local() {
    // iverilog: "x x LAST " — the same container capabilities the module-scope form got.
    assert_eq!(
        run("module m;\n\
             task automatic tk; string s[3]; int k;\n\
               for(k=0;k<3;k=k+1) s[k]=\"x\"; s[2]=\"LAST\";\n\
               foreach(s[j]) $write(\"%s \", s[j]); $display(\"\"); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "x x LAST \n"
    );
}

#[test]
fn multi_dim_frame_local() {
    // iverilog: aa dd. Registering the frame-local net in the same `fixed_string_dyn`
    // set as the module-scope form is what gives it the row-major chain walk for free —
    // and, deliberately, the same `new[]` reject and partial-index reject.
    assert_eq!(
        run("module m;\n\
             task automatic tk; string s[2][2]; s[0][0]=\"aa\"; s[1][1]=\"dd\";\n\
               $display(\"%s %s\", s[0][0], s[1][1]); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "aa dd\n"
    );
}

#[test]
fn survives_a_suspend() {
    // The array must still be there after the task blocks — the frame window stash and
    // restore has to carry the heap handle across the suspension. iverilog: pre post.
    assert_eq!(
        run("module m; reg clk=0; always #1 clk=~clk;\n\
             task automatic tk; string s[2]; s[0]=\"pre\"; @(posedge clk); s[1]=\"post\";\n\
               $display(\"%s %s\", s[0], s[1]); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "pre post\n"
    );
}

#[test]
fn in_a_framed_function() {
    // A function body-local, not just a task's. `.len()` of "abc" is 3 — iverilog says 2
    // here, which is its own long-standing defect on a string-array element, so this is
    // pinned to the IEEE answer rather than the oracle.
    assert_eq!(
        run("module m;\n\
             function automatic int f(); string s[2]; s[0]=\"abc\"; return s[0].len(); endfunction\n\
             initial begin $display(\"%0d\", f()); $finish; end\n\
             endmodule\n"),
        "3\n"
    );
}

// ── correct-or-loud ──────────────────────────────────────────────────────────

#[test]
fn a_frame_local_fixed_array_is_not_resizable() {
    // Same rule as the module-scope form, and it comes from the same set membership.
    assert!(loud(
        "module m;\n\
        task automatic tk; string s[2]; s = new[5]; $display(\"x\"); endtask\n\
        initial begin tk(); $finish; end\n\
        endmodule\n"
    ));
}

#[test]
fn recursion_with_a_frame_local_string_array_works() {
    // T1-9. §4.5.171's guard made this a fatal because the heap slot is per-NET; the
    // entry now TAKES the outer activation's contents into a stash carried by the
    // activation itself and the exit restores them, so each level gets its own array.
    // A routed frame-local string array is a `DynArray` net, so it rides that machinery
    // unchanged. iverilog: lvl0 / lvl1 / lvl2.
    assert_eq!(
        run("module m;\n\
             task automatic tk(input int n); string s[2]; s[0]=\"lvl\"; s[1]=\"x\";\n\
               if(n>0) tk(n-1);\n\
               $display(\"%s%0d %s\", s[0], n, s[1]); endtask\n\
             initial begin tk(2); $finish; end\n\
             endmodule\n"),
        "lvl0 x\nlvl1 x\nlvl2 x\n"
    );
}

#[test]
fn concurrent_activations_sharing_a_frame_local_string_array_stay_loud() {
    // T1-9 BOUNDARY. Two fork arms suspend inside the same task, so their activation
    // lifetimes OVERLAP rather than nest and the stash cannot separate them — still a
    // fatal, and now one that names the actual reason. (The recursion above nests, which
    // is the whole difference.)
    let (_, ok) = compile(
        "module m; reg clk=0; always #1 clk=~clk;\n\
         task automatic tk(input int id); string s[2]; s[0]=\"A\";\n\
           @(posedge clk); $display(\"%0d %s\", id, s[0]); endtask\n\
         initial fork tk(1); tk(2); join\n\
         initial #10 $finish;\n\
         endmodule\n",
    );
    assert!(!ok, "expected the concurrent-activation fatal");
}

#[test]
fn a_frame_local_scalar_string_is_unaffected() {
    // The lifted guard is on the ELEMENT type of a container. A scalar `string` local has
    // no unpacked dimension, keeps its slab-stored `NetKind::String`, and must not have
    // been dragged onto the heap path — that confusion is exactly what §4.5.171 measured.
    assert_eq!(
        run("module m;\n\
             task automatic tk; string x; x=\"aa\"; $display(\"%s %0d\", x, x.len()); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "aa 2\n"
    );
}

#[test]
fn a_frame_local_int_array_is_unaffected() {
    // The md-packed slot path for a bit-vector element is untouched. iverilog: 7 9.
    assert_eq!(
        run("module m;\n\
             task automatic tk; int a[2]; a[0]=7; a[1]=9; $display(\"%0d %0d\", a[0], a[1]); endtask\n\
             initial begin tk(); $finish; end\n\
             endmodule\n"),
        "7 9\n"
    );
}
