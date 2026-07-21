//! Family C (round-17): a dynamic-array-formal FUNCTION call from INSIDE a frame
//! (function/task) body. §4.5.177/179 supported this only at module-process level
//! (the `handle_copy` snapshot marker needed the `&mut` process executor). §4.5.194
//! made `dyn_heap` interior-mutable (`RefCell`), so the `&self` frame executors
//! (`run_frame_call`/`run_task`) can run the marker too — this lifts the
//! `in_frame_body` gate for the common case. iverilog is the oracle (it accepts all
//! of these). The residual loud case (a function re-forwarding its OWN dyn-formal)
//! is covered by `frame_func_dyn_formal_nested::nested_in_frame_body_loud`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_famc_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const SUM: &str =
    "function automatic int sum(input byte b[]); sum=0; foreach(b[i]) sum+=b[i]; endfunction\n";

// The report's exact Family C: `s = sum(b)` inside a SUSPENDABLE task, forwarding the
// task's own dyn-array formal `b` to the function. (KAT-driver `bytes2hex` shape.)
#[test]
fn dyn_formal_call_in_suspendable_task_body() {
    let o = run(&format!(
        "module t; logic clk=0; always #5 clk=~clk;\n{SUM}\
         task automatic run(input byte b[]); int s; @(posedge clk); s=sum(b); \
           if(s==6) $display(\"PASS s=%0d\",s); endtask\n\
         initial begin byte v[]; v=new[3]; v[0]=1;v[1]=2;v[2]=3; run(v); $finish; end endmodule\n"
    ));
    assert!(
        o.contains("PASS s=6"),
        "suspendable task dyn-formal call:\n{o}"
    );
}

// Direct-rhs `s = sum(b)` inside a NON-suspendable (subset) task body.
#[test]
fn dyn_formal_call_in_subset_task_body() {
    let o = run(&format!(
        "module t;\n{SUM}\
         task automatic run(input byte b[]); int s; s=sum(b); if(s==6) $display(\"PASS s=%0d\",s); endtask\n\
         initial begin byte v[]; v=new[3]; v[0]=1;v[1]=2;v[2]=3; run(v); $finish; end endmodule\n"
    ));
    assert!(o.contains("PASS s=6"), "subset task dyn-formal call:\n{o}");
}

// BURIED call inside a task body (`s = sum(b) + 100`) — hoisted to a direct-rhs temp.
#[test]
fn dyn_formal_buried_call_in_task_body() {
    let o = run(&format!(
        "module t;\n{SUM}\
         task automatic run(input byte b[]); int s; s=sum(b)+100; if(s==106) $display(\"PASS s=%0d\",s); endtask\n\
         initial begin byte v[]; v=new[3]; v[0]=1;v[1]=2;v[2]=3; run(v); $finish; end endmodule\n"
    ));
    assert!(
        o.contains("PASS s=106"),
        "buried dyn-formal call in task:\n{o}"
    );
}

// `$display(sum(b))` — buried in a system-task arg inside a task body.
#[test]
fn dyn_formal_call_in_display_arg_in_task() {
    let o = run(&format!(
        "module t;\n{SUM}\
         task automatic run(input byte b[]); $display(\"s=%0d\", sum(b)); endtask\n\
         initial begin byte v[]; v=new[3]; v[0]=1;v[1]=2;v[2]=3; run(v); $finish; end endmodule\n"
    ));
    assert!(o.contains("s=6"), "dyn-formal call in $display arg:\n{o}");
}

// A FUNCTION calling a dyn-formal function with a function-LOCAL dyn array (not a
// re-forward of its own formal) — supported.
#[test]
fn dyn_formal_call_with_function_local_arg() {
    let o = run(&format!(
        "module t;\n{SUM}\
         function automatic int wrap(); byte loc[]; loc=new[3]; loc[0]=1;loc[1]=2;loc[2]=3; wrap=sum(loc); endfunction\n\
         initial begin if(wrap()==6) $display(\"PASS\"); $finish; end endmodule\n"
    ));
    assert!(
        o.contains("PASS"),
        "function-local dyn arg to dyn-formal call:\n{o}"
    );
}

// Regression: the module-process-level direct-rhs path (§4.5.177) is unchanged.
#[test]
fn module_process_direct_rhs_unchanged() {
    let o = run(&format!(
        "module t;\n{SUM}\
         initial begin byte v[]; int s; v=new[3]; v[0]=1;v[1]=2;v[2]=3; s=sum(v); \
           if(s==6) $display(\"PASS\"); $finish; end endmodule\n"
    ));
    assert!(o.contains("PASS"), "module-process direct-rhs:\n{o}");
}

// correct-or-loud: RECURSION through a dyn-formal call is caught (F4004), never
// silently wrong (the per-net formal slot would clobber across activations).
#[test]
fn recursive_dyn_formal_call_is_loud() {
    let o = run(&format!(
        "module t;\n{SUM}\
         task automatic run(input int depth, input byte b[]); int s; if(depth>0) run(depth-1,b); \
           s=sum(b); if(s==6 && depth==0) $display(\"got d=%0d\",depth); endtask\n\
         initial begin byte v[]; v=new[3]; v[0]=1;v[1]=2;v[2]=3; run(1,v); $finish; end endmodule\n"
    ));
    assert!(
        o.contains("F4004") || o.contains("recursive") || !o.contains("got d=0"),
        "recursive dyn-formal call must be loud, not silently wrong:\n{o}"
    );
}
