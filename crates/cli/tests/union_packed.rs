//! N7-REST ⓑ-breadth: SystemVerilog packed `union` (IEEE 1800 §7.3).
//!
//! Oracle: iverilog -g2012 LIVE. A packed union OVERLAYS its members on the same
//! bits (width = max member); vita reuses the packed-struct flat-layout machinery
//! with every member at offset 0 — a pure parser addition (IR-0).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_union_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn union_overlay_same_width() {
    // write a, read b → same bits (a5).
    let out = run(
        "module t; typedef union packed { logic [7:0] a; logic [7:0] b; } u_t;\n\
         u_t u; initial begin u.a = 8'hA5; $display(\"b=%0h\", u.b); end endmodule\n",
    );
    assert_eq!(out, "b=a5\n");
}

#[test]
fn union_overlay_overwrite() {
    // second member write overwrites the shared bits.
    let out = run(
        "module t; typedef union packed { logic [7:0] full; logic [7:0] same; } u_t;\n\
         u_t u; initial begin u.full = 8'hFF; u.same = 8'h0F; $display(\"full=%0h\", u.full); end endmodule\n",
    );
    assert_eq!(out, "full=f\n");
}

#[test]
fn union_wide_member() {
    let out = run(
        "module t; typedef union packed { logic [15:0] w; logic [15:0] alias2; } u_t;\n\
         u_t u; initial begin u.w = 16'h1234; $display(\"a2=%0h\", u.alias2); end endmodule\n",
    );
    assert_eq!(out, "a2=1234\n");
}

#[test]
fn union_bit_select_through_member() {
    // a union member sliced — verify the overlay bits read correctly.
    let out = run(
        "module t; typedef union packed { logic [7:0] a; logic [7:0] b; } u_t;\n\
         u_t u; initial begin u.a = 8'hC3; $display(\"hi=%0h lo=%0h\", u.b[7:4], u.b[3:0]); end endmodule\n",
    );
    assert_eq!(out, "hi=c lo=3\n");
}
