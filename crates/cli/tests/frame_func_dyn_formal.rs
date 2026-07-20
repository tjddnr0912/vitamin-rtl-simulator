//! §4.5.177: an `input` DYNAMIC-array formal in a FRAMED (automatic, control-flow-body)
//! FUNCTION, called as the DIRECT rhs of a blocking assignment at module-process level
//! (`r = f(arr);`).
//!
//! The formal is reserved as a per-activation `DynArray` heap net (`reserve_frame_func`),
//! and `lower_stmt` emits a `handle_copy` snapshot marker before the assignment that
//! deep-copies the caller's array into the formal's heap slot at run time (in the `&mut`
//! process executor). The function body reads the formal from the heap (`read_net` dyn
//! branch) and — via §4.5.176 — a `foreach(c[i])` over it works. A framed function's
//! `&self` executor can never mutate a dyn-array (`new[]` / element write both need `&mut`
//! heap), so the snapshot is a sound pass-by-value/alias.
//!
//! Correct-or-loud by construction: `emit_frame_call` louds ANY dyn-array-formal call
//! that was not blessed by the marker-emitting direct-rhs path — so a call nested in a
//! bigger expression, a call from inside another subroutine body (no `&mut` executor to
//! run the marker), recursion, a non-bare or sign-mismatched actual, all stay loud. A
//! straight-line dyn-formal function still takes the R2 inline alias path.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffdf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009") || o.contains("F4004") || o.contains("ended (Error)")
}

// ── direct-rhs `r = fsum(a)` with a foreach-summing function (iverilog: 15) ──
#[test]
fn direct_rhs_int_foreach() {
    let o = run("module top;\n\
         function automatic int fsum(input int c[]);\n\
           int s=0; foreach(c[i]) s+=c[i]; return s;\n\
         endfunction\n\
         int a[]; int r;\n\
         initial begin a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=fsum(a); $display(\"sum=%0d\", r); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("sum=15"),
        "direct-rhs dyn-formal function = 15:\n{o}"
    );
}

// ── signed byte element (-5+10-2 = 3) ──
#[test]
fn direct_rhs_signed_byte() {
    let o = run("module top;\n\
         function automatic int f(input byte c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         byte a[]; int r; initial begin a=new[3]; a[0]=-5;a[1]=10;a[2]=-2; r=f(a); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("r=3"),
        "signed byte dyn-formal function = 3:\n{o}"
    );
}

// ── two calls with different arrays are isolated (each snapshots its own actual) ──
#[test]
fn two_calls_isolated() {
    let o = run("module top;\n\
         function automatic int f(input int c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         int a[]; int b[]; int r1; int r2;\n\
         initial begin\n\
           a=new[2]; a[0]=1;a[1]=2; b=new[3]; b[0]=10;b[1]=20;b[2]=30;\n\
           r1=f(a); r2=f(b); $display(\"%0d %0d\", r1, r2);\n\
         end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("3 60"),
        "two isolated calls: 3 60:\n{o}"
    );
}

// ── the function reads a snapshot: a later caller mutation doesn't change the result ──
#[test]
fn pass_by_value_snapshot() {
    let o = run("module top;\n\
         function automatic int f(input int c[]); return c[0]; endfunction\n\
         int a[]; int r; initial begin a=new[2]; a[0]=7; r=f(a); a[0]=999; $display(\"r=%0d a0=%0d\", r, a[0]); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("r=7 a0=999"),
        "snapshot: r=7 regardless of later mutation:\n{o}"
    );
}

// ── a for-loop over .size() (not foreach) also works ──
#[test]
fn direct_rhs_for_loop() {
    let o = run("module top;\n\
         function automatic int f(input int c[]);\n\
           int s=0; for (int i=0;i<c.size();i++) s+=c[i]; return s;\n\
         endfunction\n\
         int a[]; int r; initial begin a=new[3]; a[0]=4;a[1]=5;a[2]=6; r=f(a); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("r=15"),
        "for-loop dyn-formal function = 15:\n{o}"
    );
}

// ── §4.5.179: a call as a `$display` arg (a bigger expression, not the direct rhs) is
// now SUPPORTED — `hoist_stmt_top` lifts it to a `__t = f(a)` temp (which re-triggers the
// direct-rhs marker). Full coverage of the hoist (arith/if/short-circuit-loud/…) lives in
// `frame_func_dyn_formal_nested.rs`. Kept here as the §4.5.177→§4.5.179 transition marker.
#[test]
fn nested_in_display_now_supported() {
    let o = run("module top;\n\
         function automatic int f(input int c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         int a[]; initial begin a=new[3]; a[0]=4;a[1]=5;a[2]=6; $display(\"sum=%0d\", f(a)); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("sum=15"),
        "a dyn-formal call as a $display arg is hoisted (§4.5.179) = 15:\n{o}"
    );
}

// ── LOUD: a call from INSIDE another function body (no &mut executor for the marker) ──
#[test]
fn call_from_subroutine_body_stays_loud() {
    let o = run("module top;\n\
         function automatic int inner(input int c[]); int s=0; foreach(c[i]) s+=c[i]; return s; endfunction\n\
         function automatic int outer(input int c[]); int r; r=inner(c); return r; endfunction\n\
         int a[]; int r; initial begin a=new[3]; a[0]=1;a[1]=2;a[2]=3; r=outer(a); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "a dyn-formal call from inside a function body must stay loud:\n{o}"
    );
}

// ── LOUD: recursion (the recursive call is inside the body → no marker) ──
#[test]
fn recursion_stays_loud() {
    let o = run("module top;\n\
         function automatic int f(input int c[], input int n);\n\
           if (n<=0) return 0;\n\
           return c[n-1] + f(c, n-1);\n\
         endfunction\n\
         int a[]; int r; initial begin a=new[3]; a[0]=1;a[1]=2;a[2]=3; r=f(a,3); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "recursive dyn-formal function must stay loud:\n{o}"
    );
}

// ── LOUD: element-type sign mismatch (byte c[] ← byte unsigned a[]) ──
#[test]
fn sign_mismatch_stays_loud() {
    let o = run("module top;\n\
         function automatic int f(input byte c[]); return c[0]; endfunction\n\
         byte unsigned a[]; int r; initial begin a=new[1]; a[0]=100; r=f(a); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o) && o.contains("matching dynamic-array actual"),
        "sign-mismatched actual must stay loud:\n{o}"
    );
}
