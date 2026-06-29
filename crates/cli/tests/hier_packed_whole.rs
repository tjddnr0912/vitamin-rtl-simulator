//! HIER-REST-MP: a WHOLE multi-dim PACKED hierarchical net (`dut.pm` where
//! `reg [1:0][7:0] pm`) reads/writes its flat vector value. A multi-dim packed net
//! is stored as a single flat vector, so the whole-net read/write needs no special
//! handling — the old guard over-rejected it as "no plain readable value". An
//! element select `dut.pm[i]` still takes the deferred-sel lane (unchanged); only
//! the WHOLE-net `dut.pm` form is unblocked here. Pure IR-0 (elaborate only).
//!
//! Values pinned to LIVE iverilog 13.0. A whole UNPACKED array (`dut.mem`) stays
//! loud — iverilog rejects it too ("$display does not support vpiMemory").
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hpk_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn whole_packed_read() {
    let (out, _c) = run(
        "module sub; reg [1:0][7:0] pm; initial pm=16'hABCD; endmodule\n\
         module top; sub dut();\n\
           initial #1 $display(\"R %h\", dut.pm);\n\
         endmodule\n",
    );
    assert!(
        out.contains("R abcd"),
        "whole multi-dim packed read:\n{out}"
    );
}

#[test]
fn whole_packed_write() {
    let (out, _c) = run("module sub; reg [1:0][7:0] pm; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.pm=16'h1234; #1 $display(\"R %h\", dut.pm); end\n\
         endmodule\n");
    assert!(
        out.contains("R 1234"),
        "whole multi-dim packed write:\n{out}"
    );
}

#[test]
fn whole_packed_write_then_element_read() {
    // write the whole 16-bit value, then read the low 8-bit element: 0x1234 -> [0]=34.
    let (out, _c) = run("module sub; reg [1:0][7:0] pm; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.pm=16'h1234; #1 $display(\"R %h\", dut.pm[0]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 34"),
        "whole write then element read:\n{out}"
    );
}

#[test]
fn three_dim_packed_whole_read() {
    let (out, _c) = run(
        "module sub; reg [1:0][1:0][7:0] g; initial g=32'hDEADBEEF; endmodule\n\
         module top; sub dut();\n\
           initial #1 $display(\"R %h\", dut.g);\n\
         endmodule\n",
    );
    assert!(
        out.contains("R deadbeef"),
        "3-dim packed whole read:\n{out}"
    );
}

#[test]
fn three_level_whole_packed_read() {
    let (out, _c) = run(
        "module leaf; reg [1:0][7:0] pm; initial pm=16'hcafe; endmodule\n\
         module mid; leaf l(); endmodule\n\
         module top; mid m();\n\
           initial #1 $display(\"R %h\", m.l.pm);\n\
         endmodule\n",
    );
    assert!(out.contains("R cafe"), "3-level whole packed read:\n{out}");
}

#[test]
fn whole_unpacked_array_read_still_loud() {
    // a whole UNPACKED array has no plain readable value — iverilog rejects it too,
    // so this must stay loud (NOT a silent value), unaffected by the MP unblock.
    let (out, code) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial #1 $display(\"R %h\", dut.mem);\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "whole unpacked array read must stay loud: {out} {code:?}"
    );
}
