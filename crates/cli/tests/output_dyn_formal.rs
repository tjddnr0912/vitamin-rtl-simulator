//! §4.5.194 V2B: an `output`/`inout` DYNAMIC-array formal on a TASK or FUNCTION
//! (`task mk(output byte b[]); b=new[3]; b[0]=1;…`) loud→supported. The body writes the
//! formal's DynArray heap slot (new[]/element — §4.5.194 V5/write-path); at the
//! subroutine's exit the engine DEEP-COPIES the formal's heap array OUT to the caller's
//! array (`frame_dyn_copy_out`, the mirror of the input snapshot). INOUT also snapshots
//! the caller IN at entry. An output/inout dyn-formal task is forced to the FRAME path
//! (the inline path has no heap binding). A function with output/inout formals rides the
//! same run_task copy-out path.
//!
//! iverilog 13.0 IS an oracle for the TASK cases (it runs task output/inout dyn formals);
//! it REJECTS function output ports, so the function cases are hand-IEEE (§13.4.1/§13.5.1)
//! — values below match iverilog for tasks and the IEEE pass-by-value result for functions.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_odf_{}_{n}", std::process::id()));
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

#[test]
fn task_output_dyn_formal() {
    let o = run(
        "module top;\n\
         task automatic mk(output byte b[]); b=new[3]; b[0]=1;b[1]=2;b[2]=3; endtask\n\
         byte a[]; initial begin mk(a); $display(\"sz=%0d %0d %0d %0d\", a.size(), a[0],a[1],a[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("sz=3 1 2 3"), "got:\n{}", o.0);
}

#[test]
fn task_output_replaces_caller_array() {
    // The copy-out REPLACES the caller's prior (larger) array. iverilog: sz=2 7 8.
    let o = run(
        "module top;\n\
         task automatic mk(output int b[]); b=new[2]; b[0]=7;b[1]=8; endtask\n\
         int a[]; initial begin a=new[5]; a[0]=99; mk(a); $display(\"sz=%0d %0d %0d\", a.size(), a[0], a[1]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("sz=2 7 8"), "got:\n{}", o.0);
}

#[test]
fn task_inout_dyn_formal_modifies_in_place() {
    // INOUT copies IN, the body mutates, then copies OUT. iverilog: 11 12 13.
    let o = run(
        "module top;\n\
         task automatic bump(inout byte b[]); foreach(b[i]) b[i]=b[i]+10; endtask\n\
         byte a[]; initial begin a=new[3]; a[0]=1;a[1]=2;a[2]=3; bump(a); $display(\"%0d %0d %0d\", a[0],a[1],a[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("11 12 13"), "got:\n{}", o.0);
}

#[test]
fn task_inout_resize_grow() {
    // INOUT that reallocates (grow by one). iverilog: sz=3 1 2 99.
    let o = run(
        "module top;\n\
         task automatic grow(inout int b[]); int old[]; old=b; b=new[b.size()+1]; foreach(old[i]) b[i]=old[i]; b[b.size()-1]=99; endtask\n\
         int a[]; initial begin a=new[2]; a[0]=1;a[1]=2; grow(a); $display(\"sz=%0d %0d %0d %0d\", a.size(), a[0],a[1],a[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("sz=3 1 2 99"), "got:\n{}", o.0);
}

#[test]
fn task_two_calls_isolated() {
    let o = run(
        "module top;\n\
         task automatic mk(input int k, output int b[]); b=new[k]; for(int i=0;i<k;i++) b[i]=i+k; endtask\n\
         int x[]; int y[]; initial begin mk(2,x); mk(3,y); $display(\"%0d %0d | %0d %0d %0d\", x[0],x[1], y[0],y[1],y[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("2 3 | 3 4 5"), "got:\n{}", o.0);
}

#[test]
fn function_output_dyn_formal_hand_ieee() {
    // No iverilog oracle (function output ports are rejected there); hand-IEEE §13.4.1:
    // b=new[3]; b[i]=i+1; copied out → sz=3 1 2 3.
    let o = run(
        "module top;\n\
         function void mk(output byte b[]); b=new[3]; b[0]=1;b[1]=2;b[2]=3; endfunction\n\
         byte a[]; initial begin mk(a); $display(\"sz=%0d %0d %0d %0d\", a.size(), a[0],a[1],a[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("sz=3 1 2 3"), "got:\n{}", o.0);
}

#[test]
fn function_input_and_output_dyn() {
    // A function copying a doubled input dyn array to an output dyn formal (hand-IEEE): 2 4 6.
    let o = run(
        "module top;\n\
         function void f(input int src[], output int b[]); b=new[src.size()]; foreach(src[i]) b[i]=src[i]*2; endfunction\n\
         int s[]; int d[]; initial begin s=new[3]; s[0]=1;s[1]=2;s[2]=3; f(s,d); $display(\"%0d %0d %0d\", d[0],d[1],d[2]); end\n\
         endmodule\n",
    );
    assert!(o.0.contains("2 4 6"), "got:\n{}", o.0);
}

#[test]
fn non_bare_output_actual_stays_loud() {
    // Correct-or-loud: the copy-out targets a whole dyn net — a select/element actual
    // has no whole-handle target, so it stays loud rather than mis-routing.
    let o = run("module top;\n\
         task automatic mk(output byte b[]); b=new[2]; b[0]=1; endtask\n\
         byte a[][]; initial begin a=new[2]; mk(a[0]); $display(\"done\"); end\n\
         endmodule\n");
    assert!(
        !o.1 || o.0.contains("E3009"),
        "non-bare actual should be loud:\n{}",
        o.0
    );
}
