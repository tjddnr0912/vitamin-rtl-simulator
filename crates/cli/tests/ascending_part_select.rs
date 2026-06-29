//! A part-select on an ASCENDING (little-endian) vector `logic [0:N] v` — both
//! read (`v[2:5]`) and write (`v[2:5] = …`) — used to be loud-rejected with
//! E3009 even though it is legal and the offset machinery already handled it;
//! only the WIDTH (`msb - lsb + 1`) underflowed for `msb < lsb`. A part-select
//! whose bound direction does NOT match the net's declared direction is still
//! rejected ("out of order"), matching iverilog. Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_aps_{}_{n}.sv", std::process::id()));
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

fn run_expect_loud(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_aps_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a loud reject (nonzero exit)"
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn ascending_part_select_read() {
    // logic [0:7] v = 8'hA5 (index 0 is MSB). v[2:5] selects source indices 2..5
    // = bits 1,0,0,1 = 9. iverilog: 9.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'hA5; $display(\"%h\", v[2:5]); end\n\
         endmodule\n");
    assert_eq!(out, "9\n");
}

#[test]
fn ascending_part_select_write() {
    // logic [0:7] v = 0; v[2:5] = 4'hF sets source indices 2..5 -> 0011_1100 = 3c.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'h00; v[2:5] = 4'hF; $display(\"%h\", v); end\n\
         endmodule\n");
    assert_eq!(out, "3c\n");
}

#[test]
fn ascending_part_select_in_arith() {
    // The selected value participates in arithmetic, proving genuine width.
    let out = run("module t;\n\
           logic [0:15] v;\n\
           logic [7:0] s;\n\
           initial begin v = 16'h00FF; s = v[8:11] + 4'd1; $display(\"%0d\", s); end\n\
         endmodule\n");
    // v=00FF: source idx 8..11 -> bits (idx8..11). 16'h00FF MSB(idx0)..LSB(idx15).
    // bits 8,9,10,11 = 1,1,1,1 = 15; +1 = 16.
    assert_eq!(out, "16\n");
}

#[test]
fn descending_part_select_still_works() {
    // CONTROL: a normal descending part-select is byte-identical to before.
    let out = run("module t;\n\
           logic [7:0] v;\n\
           initial begin v = 8'h00; v[5:2] = 4'hF; $display(\"%h\", v); end\n\
         endmodule\n");
    assert_eq!(out, "3c\n");
}

#[test]
fn ascending_bounds_on_descending_net_is_loud() {
    // logic [7:0] v; v[2:5] — bound direction (ascending) does not match the net
    // (descending). iverilog: "part select v[2:5] is out of order". Stay loud.
    let err = run_expect_loud(
        "module t;\n\
           logic [7:0] v;\n\
           initial begin v = 8'hA5; $display(\"%h\", v[2:5]); end\n\
         endmodule\n",
    );
    assert!(
        err.contains("part-select") || err.contains("part select"),
        "expected a part-select diagnostic, got:\n{err}"
    );
}

#[test]
fn descending_bounds_on_ascending_net_is_loud() {
    // logic [0:7] v; v[5:2] — descending bounds on an ascending net. Out of order.
    let err = run_expect_loud(
        "module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'hA5; $display(\"%h\", v[5:2]); end\n\
         endmodule\n",
    );
    assert!(
        err.contains("part-select") || err.contains("part select"),
        "expected a part-select diagnostic, got:\n{err}"
    );
}

// ───────── indexed part-select (`+:` / `-:`) on ascending nets ─────────
// IEEE 1800 §11.5.1 + §7.4.3: the index range is on the DECLARED index space.
// On an ascending net, `+:` moves toward the LSB end (down in internal bits) and
// `-:` toward the MSB end — vita previously kept the descending direction, so the
// select X-poisoned or picked the wrong bits. Oracle: iverilog.

#[test]
fn ascending_indexed_part_up_read() {
    // logic [0:7] v=8'hA5; v[0+:4] = source idx {0,1,2,3} = 1,0,1,0 = 1010.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'hA5; $display(\"%b\", v[0+:4]); end\n\
         endmodule\n");
    assert_eq!(out, "1010\n");
}

#[test]
fn ascending_indexed_part_down_read() {
    // v[7-:4] = source idx {7,6,5,4} = 0101.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'hA5; $display(\"%b\", v[7-:4]); end\n\
         endmodule\n");
    assert_eq!(out, "0101\n");
}

#[test]
fn ascending_indexed_part_up_write() {
    // v[0+:4]=4'hF sets the MSB-end nibble (source idx 0..3) -> f0.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'h00; v[0+:4] = 4'hF; $display(\"%h\", v); end\n\
         endmodule\n");
    assert_eq!(out, "f0\n");
}

#[test]
fn ascending_indexed_part_down_write() {
    // v[7-:4]=4'hF sets the LSB-end nibble (source idx 4..7) -> 0f.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           initial begin v = 8'h00; v[7-:4] = 4'hF; $display(\"%h\", v); end\n\
         endmodule\n");
    assert_eq!(out, "0f\n");
}

#[test]
fn ascending_indexed_nonzero_base() {
    // logic [1:8] v=8'hA5; v[1+:4] = source idx {1,2,3,4} = 1010.
    let out = run("module t;\n\
           logic [1:8] v;\n\
           initial begin v = 8'hA5; $display(\"%b\", v[1+:4]); end\n\
         endmodule\n");
    assert_eq!(out, "1010\n");
}

#[test]
fn ascending_indexed_runtime_base() {
    // A RUNTIME base on an ascending net normalizes at run time.
    let out = run("module t;\n\
           logic [0:7] v;\n\
           int i;\n\
           initial begin v = 8'hA5; i = 2; $display(\"%b\", v[i+:3]); end\n\
         endmodule\n");
    // v[2+:3] = source idx {2,3,4} = 1,0,0 = 100.
    assert_eq!(out, "100\n");
}

// ───────── ascending part-select on an unpacked-array ELEMENT ─────────

#[test]
fn ascending_array_element_regular_read() {
    // logic [0:7] m[0:1]; m[0]=8'hA5; m[0][2:5] = source idx {2,3,4,5} = 1001 = 9.
    let out = run("module t;\n\
           logic [0:7] m[0:1];\n\
           initial begin m[0] = 8'hA5; $display(\"%h\", m[0][2:5]); end\n\
         endmodule\n");
    assert_eq!(out, "9\n");
}

#[test]
fn ascending_array_element_indexed_read() {
    // m[0][0+:4] = source idx {0,1,2,3} = 1010.
    let out = run("module t;\n\
           logic [0:7] m[0:1];\n\
           initial begin m[0] = 8'hA5; $display(\"%b\", m[0][0+:4]); end\n\
         endmodule\n");
    assert_eq!(out, "1010\n");
}

#[test]
fn ascending_array_element_write() {
    // m[1][2:5]=4'hF -> m[1] = 0011_1100 = 3c (other element untouched).
    let out = run("module t;\n\
           logic [0:7] m[0:1];\n\
           initial begin m[0] = 8'hFF; m[1] = 8'h00; m[1][2:5] = 4'hF;\n\
             $display(\"%h %h\", m[0], m[1]); end\n\
         endmodule\n");
    assert_eq!(out, "ff 3c\n");
}

// ───────── descending controls (must remain byte-identical) ─────────

#[test]
fn descending_indexed_still_works() {
    let out = run("module t;\n\
           logic [7:0] v;\n\
           initial begin v = 8'hA5; $display(\"%b\", v[2+:4]); end\n\
         endmodule\n");
    assert_eq!(out, "1001\n");
}

#[test]
fn descending_array_indexed_still_works() {
    let out = run("module t;\n\
           logic [7:0] m[0:1];\n\
           initial begin m[0] = 8'hA5; $display(\"%b\", m[0][2+:4]); end\n\
         endmodule\n");
    assert_eq!(out, "1001\n");
}
