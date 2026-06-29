//! A task `output`/`inout` argument whose ACTUAL is a part-select (`a[5:2]`),
//! bit-select (`a[3]`), or indexed part-select (`a[2+:4]`) used to be loud-
//! rejected ("output/inout arg must be a simple net"). IEEE 1800 §13.5.3 allows
//! any variable lvalue. The copy-out now writes back through the lowered lvalue
//! (and an `inout` copies the actual's value in at entry). Oracle: iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_tps_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn output_part_select_actual() {
    // setit writes 4'hF to a[5:2] -> 0011_1100 = 3c (other bits untouched).
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task setit(output [3:0] x); x = 4'hF; endtask\n\
           initial begin a = 8'h00; setit(a[5:2]); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "3c\n");
}

#[test]
fn inout_part_select_actual() {
    // a=3C -> a[5:2]=F; incr does x=x+1 -> 0 (4-bit wrap) -> a=00.
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task incr(inout [3:0] x); x = x + 1; endtask\n\
           initial begin a = 8'h3C; incr(a[5:2]); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "00\n");
}

#[test]
fn output_bit_select_actual() {
    // setbit writes 1 to a[3] -> 0000_1000 = 08.
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task setbit(output b); b = 1'b1; endtask\n\
           initial begin a = 8'h00; setbit(a[3]); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "08\n");
}

#[test]
fn output_indexed_part_select_actual() {
    // setit writes 4'hF to a[2+:4] = a[5:2] -> 3c.
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task setit(output [3:0] x); x = 4'hF; endtask\n\
           initial begin a = 8'h00; setit(a[2+:4]); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "3c\n");
}

#[test]
fn automatic_task_output_part_select_actual() {
    // Frame (automatic) path: setit writes 4'hA to a[7:4] -> a0.
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task automatic setit(output [3:0] x); x = 4'hA; endtask\n\
           initial begin a = 8'h00; setit(a[7:4]); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "a0\n");
}

#[test]
fn output_array_element_actual() {
    // An unpacked-array element as the output actual writes only that element.
    let out = run("module t;\n\
           logic [7:0] m[0:1];\n\
           task setit(output [7:0] x); x = 8'hCD; endtask\n\
           initial begin m[0] = 8'hAB; m[1] = 8'h00; setit(m[1]);\n\
             $display(\"%h %h\", m[0], m[1]); end\n\
         endmodule\n");
    assert_eq!(out, "ab cd\n");
}

#[test]
fn select_of_frame_local_actual_is_loud_not_panic() {
    // A part-select of an AUTOMATIC task's frame-local variable passed as a nested
    // output actual cannot be routed by the engine's frame copy-out (whole-net
    // only). It must be a clean loud reject, NOT a debug-build panic.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_tps_fl_{}_{n}.sv", std::process::id()));
    std::fs::write(
        &path,
        "module top;\n\
           reg [7:0] x;\n\
           task automatic inner(output [3:0] nn); nn = 4'h9; endtask\n\
           task automatic outer(output [7:0] w);\n\
             reg [7:0] tmp; tmp = 8'h00; inner(tmp[3:0]); w = tmp;\n\
           endtask\n\
           initial begin x = 8'h00; outer(x); $display(\"%h\", x); end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a loud reject (nonzero exit)"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("panicked"), "must not panic, got:\n{err}");
    assert!(
        err.contains("frame-local"),
        "expected a frame-local diagnostic, got:\n{err}"
    );
}

#[test]
fn output_simple_net_still_works() {
    // CONTROL: a whole-net output actual is unchanged.
    let out = run("module t;\n\
           logic [7:0] a;\n\
           task setit(output [7:0] x); x = 8'h42; endtask\n\
           initial begin setit(a); $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "42\n");
}
