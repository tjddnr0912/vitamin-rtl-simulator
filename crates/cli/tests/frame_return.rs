//! Remaining Tier-ⓐ: `return` keyword in module/free functions and tasks (was
//! class-method-only). A function body containing `return` is already routed to
//! the frame path (`body_needs_frame`); the fix sets `cur_return` there, mirroring
//! class methods. IR-0 (exit BB + Goto/Return are existing shapes — NO format bump;
//! the doc's format-bump claim was refuted by investigation). Oracle = iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fret_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn return_value_in_module_function() {
    let (out, err, code) = run("module top;\n\
         integer r;\n\
         function integer addone(input integer a);\n\
           return a + 1;\n\
         endfunction\n\
         initial begin r = addone(5); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=6"), "got:\n{out}");
}

#[test]
fn return_early_exit_branch() {
    let (out, err, code) = run("module top;\n\
         integer r1, r2;\n\
         function integer sgn(input integer a);\n\
           if (a < 0) return -1;\n\
           return 1;\n\
         endfunction\n\
         initial begin r1 = sgn(-7); r2 = sgn(3); $display(\"r1=%0d r2=%0d\", r1, r2); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r1=-1 r2=1"), "got:\n{out}");
}

#[test]
fn return_after_loop_accumulator() {
    let (out, err, code) = run("module top;\n\
         integer r;\n\
         function integer sumto(input integer n);\n\
           integer i, s;\n\
           s = 0;\n\
           for (i = 1; i <= n; i = i + 1) s = s + i;\n\
           return s;\n\
         endfunction\n\
         initial begin r = sumto(5); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=15"), "got:\n{out}");
}

#[test]
fn return_unassigned_two_state_local_defaults_to_zero() {
    // IEEE §6.4: a 2-state local (int) defaults to 0, not X. A frame function reading
    // an unassigned `int` local must return 0 (previously X — frame setup ignored the
    // net's 2-state init and X-filled every slot).
    let (out, err, code) = run("module top;\n\
         integer r;\n\
         function automatic int f();\n\
           int z;\n\
           return z;\n\
         endfunction\n\
         initial begin r = f(); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=0"), "got:\n{out}");
}

#[test]
fn two_state_local_default_drives_control_flow() {
    // The dangerous form: an X default would silently take the wrong case arm. With
    // the 2-state default of 0, `case (z) 0:` matches -> returns 1000.
    let (out, err, code) = run("module top;\n\
         integer r;\n\
         function automatic int chooser();\n\
           int z;\n\
           case (z)\n\
             0: return 1000;\n\
             default: return 2000;\n\
           endcase\n\
         endfunction\n\
         initial begin r = chooser(); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=1000"), "got:\n{out}");
}

#[test]
fn four_state_local_still_defaults_x() {
    // Control: a 4-state `integer` local still defaults to X (unchanged) — %0d of X
    // prints `x`. Guards against over-zeroing.
    let (out, err, code) = run("module top;\n\
         function automatic integer f();\n\
           integer z;\n\
           return z;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", f()); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r=x"), "got:\n{out}");
}

#[test]
fn return_bare_early_exit_in_task() {
    let (out, err, code) = run("module top;\n\
         integer x, y;\n\
         task setpos(input integer a, output integer o);\n\
           if (a < 0) begin o = 0; return; end\n\
           o = a;\n\
         endtask\n\
         initial begin setpos(-5, x); setpos(7, y); $display(\"x=%0d y=%0d\", x, y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("x=0 y=7"), "got:\n{out}");
}
