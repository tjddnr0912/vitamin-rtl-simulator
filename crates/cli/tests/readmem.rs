//! v7 $readmemb/$readmemh — iverilog 13.0 live pins (probes t11–t13,
//! 2026-06-12): fill starts at the LOWEST index ascending regardless of
//! declaration direction (1364-2005 default), `@addr` is hex in both
//! variants, unwritten entries keep their prior value, a token shortfall
//! warns ONLY when the file has no address directives, and a missing file
//! diagnoses but the run continues with exit 0 (vitamin: W4023 — iverilog
//! labels it ERROR yet still exits 0; we keep the exit parity).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_with_files(src: &str, files: &[(&str, &str)]) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for (name, content) in files {
        std::fs::write(d.join(name), content).unwrap();
    }
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

const HEX: &str = "// comment line\naa bb\n@4 cc\ndd\n";

#[test]
fn readmemh_tokens_jump_and_default_x() {
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [7:0] m [0:7];\n\
         integer i;\n\
         initial begin\n\
           $readmemh(\"mem1.hex\", m);\n\
           for (i = 0; i < 8; i = i + 1) $display(\"m[%0d]=%h\", i, m[i]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[("mem1.hex", HEX)],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let want = "m[0]=aa\nm[1]=bb\nm[2]=xx\nm[3]=xx\nm[4]=cc\nm[5]=dd\nm[6]=xx\nm[7]=xx";
    assert!(out.contains(want), "got:\n{out}");
}

#[test]
fn readmemb_xz_digits_and_shortfall_warning() {
    // 2 tokens for a 4-element range, no @ directives → warning + partial.
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [3:0] b [0:3];\n\
         integer i;\n\
         initial begin\n\
           $readmemb(\"mem1.bin\", b);\n\
           for (i = 0; i < 4; i = i + 1) $display(\"b[%0d]=%b\", i, b[i]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[("mem1.bin", "1010 // four\nxx1z\n")],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(err.contains("W4023"), "want shortfall warning, got:\n{err}");
    let want = "b[0]=1010\nb[1]=xx1z\nb[2]=xxxx\nb[3]=xxxx";
    assert!(out.contains(want), "got:\n{out}");
}

#[test]
fn readmemh_with_start_finish_range() {
    // Tokens map into [2..5]; the @4 directive still jumps inside the range.
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [7:0] m [0:7];\n\
         initial begin\n\
           $readmemh(\"mem1.hex\", m, 2, 5);\n\
           $display(\"r2=%h %h %h %h\", m[2], m[3], m[4], m[5]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[("mem1.hex", HEX)],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("r2=aa bb cc dd"), "got:\n{out}");
}

#[test]
fn readmem_descending_decl_fills_lowest_first() {
    // [7:0] declaration: 1364-2005 behavior = same lowest-ascending fill.
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [7:0] m [7:0];\n\
         integer i;\n\
         initial begin\n\
           $readmemh(\"mem1.hex\", m);\n\
           for (i = 0; i < 8; i = i + 1) $display(\"m[%0d]=%h\", i, m[i]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[("mem1.hex", HEX)],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let want = "m[0]=aa\nm[1]=bb\nm[2]=xx\nm[3]=xx\nm[4]=cc\nm[5]=dd\nm[6]=xx\nm[7]=xx";
    assert!(out.contains(want), "got:\n{out}");
}

#[test]
fn readmem_missing_file_warns_and_continues() {
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [7:0] m [0:3];\n\
         initial begin\n\
           $readmemh(\"nofile.hex\", m);\n\
           $display(\"after=%h\", m[0]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(err.contains("W4023"), "want file warning, got:\n{err}");
    assert!(out.contains("after=xx"), "got:\n{out}");
}

#[test]
fn readmem_nonzero_base_offsets_addresses() {
    // [4:7] array: address 4 is the first element (word 0).
    let (out, err, code) = run_with_files(
        "module top;\n\
         reg [7:0] m [4:7];\n\
         initial begin\n\
           $readmemh(\"m2.hex\", m);\n\
           $display(\"v=%h %h %h %h\", m[4], m[5], m[6], m[7]);\n\
           $finish;\n\
         end\n\
         endmodule\n",
        &[("m2.hex", "@5 11 22\n")],
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("v=xx 11 22 xx"), "got:\n{out}");
}
