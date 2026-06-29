//! `$writememb` / `$writememh` — the write-side mirror of `$readmemb/h`
//! (Medium-bundle rank 5, SYS-READ; pure IR-0 — the SysTaskId variants exist
//! from the v9 shape bump). Every expected byte below is pinned to LIVE
//! iverilog 13.0 (`iverilog -g2012` + `vvp`):
//!   - the first line is ALWAYS the literal `// 0x00000000` header (it never
//!     reflects start/base);
//!   - hex = ceil(width/4) lowercase zero-padded digits, X/Z nibble
//!     compression (all-x->x, all-z->z, mixed-with-x->X, mixed-z-only->Z,
//!     X dominates Z); bin = exactly `width` per-bit chars, NO compression;
//!   - optional (start[,finish]) is an inclusive declared-index window,
//!     descending when finish < start; out-of-range = non-fatal (file NOT
//!     created, sim continues, exit 0).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `vita` on `src` in a fresh temp dir; return (stdout, exit-code, dir).
fn run(src: &str) -> (String, Option<i32>, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wm_{}_{n}", std::process::id()));
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
        d,
    )
}

fn read_out(d: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(d.join(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

#[test]
fn writememh_hex_digits_and_header() {
    let (_o, _c, d) = run("module t;\n\
         reg [11:0] mem [0:3];\n\
         initial begin\n\
           mem[0]=12'h001; mem[1]=12'habc; mem[2]=12'hfff; mem[3]=12'h000;\n\
           $writememh(\"o.hex\", mem);\n\
         end\n\
         endmodule\n");
    assert_eq!(read_out(&d, "o.hex"), "// 0x00000000\n001\nabc\nfff\n000\n");
}

#[test]
fn writememb_binary_width_and_xz() {
    let (_o, _c, d) = run("module t;\n\
         reg [11:0] mem [0:1];\n\
         initial begin\n\
           mem[0]=12'b1x1x_1z1z_0101; mem[1]=12'b0000_0000_000x;\n\
           $writememb(\"o.bin\", mem);\n\
         end\n\
         endmodule\n");
    assert_eq!(
        read_out(&d, "o.bin"),
        "// 0x00000000\n1x1x1z1z0101\n00000000000x\n"
    );
}

#[test]
fn writememh_xz_nibble_compression() {
    // axz: 1010->a, xxxx->x, zzzz->z. XZ5: 1x1x->X, 1z1z->Z, 0101->5.
    // 00X: 0000,0000,000x. X00: 10xz->X (X dominates Z in a nibble).
    let (_o, _c, d) = run("module t;\n\
         reg [11:0] m [0:3];\n\
         initial begin\n\
           m[0]=12'b1010_xxxx_zzzz; m[1]=12'b1x1x_1z1z_0101;\n\
           m[2]=12'b0000_0000_000x; m[3]=12'b10xz_0000_0000;\n\
           $writememh(\"o.hex\", m);\n\
         end\n\
         endmodule\n");
    assert_eq!(read_out(&d, "o.hex"), "// 0x00000000\naxz\nXZ5\n00X\nX00\n");
}

#[test]
fn writememh_range_inclusive_and_descending() {
    let (_o, _c, d) = run("module t;\n\
         reg [7:0] r [0:7]; integer i;\n\
         initial begin\n\
           for (i=0;i<8;i=i+1) r[i]=i*16+i;\n\
           $writememh(\"asc.hex\", r, 2, 5);\n\
           $writememh(\"desc.hex\", r, 5, 2);\n\
         end\n\
         endmodule\n");
    assert_eq!(read_out(&d, "asc.hex"), "// 0x00000000\n22\n33\n44\n55\n");
    assert_eq!(read_out(&d, "desc.hex"), "// 0x00000000\n55\n44\n33\n22\n");
}

#[test]
fn writememh_oob_is_nonfatal_no_file() {
    let (out, code, d) = run("module t;\n\
         reg [7:0] mem [0:3]; integer i;\n\
         initial begin\n\
           for (i=0;i<4;i=i+1) mem[i]=i;\n\
           $writememh(\"oob.hex\", mem, 0, 99);\n\
           $display(\"after-oob\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("after-oob"),
        "sim must continue past OOB:\n{out}"
    );
    assert_eq!(code, Some(0), "OOB writemem is non-fatal (exit 0)");
    assert!(
        !d.join("oob.hex").exists(),
        "OOB must NOT create the output file"
    );
}

#[test]
fn writememh_default_base_nonzero_index() {
    // declared base != 0: the data lines cover the full declared range; the
    // header is STILL the literal 0x00000000 (never the base).
    let (_o, _c, d) = run("module t;\n\
         reg [7:0] mem [2:4];\n\
         initial begin\n\
           mem[2]=8'haa; mem[3]=8'hbb; mem[4]=8'hcc;\n\
           $writememh(\"b.hex\", mem);\n\
         end\n\
         endmodule\n");
    assert_eq!(read_out(&d, "b.hex"), "// 0x00000000\naa\nbb\ncc\n");
}
