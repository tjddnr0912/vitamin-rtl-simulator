//! v9 `$fread(target, fd[, start[, count]])` (Medium-bundle rank 5, SYS-READ
//! part 3b; pure IR-0). A direct-rhs-of-blocking-assign special form in the
//! $value$plusargs family: it binary-reads bytes into the target reg/memory and
//! returns the byte count to the lhs.
//!
//! Every expected value is pinned to LIVE iverilog 13.0:
//!   - a single reg reads ceil(W/8) bytes big-endian (first byte = most
//!     significant); a partial fill leaves the unread LOW bytes at their prior
//!     value; a non-byte-multiple width keeps the LOW W bits (reg[12:0] from
//!     0xFFFF => 0x1fff, NOT 0x0fff);
//!   - a memory fills elements ascending from the base (regardless of declared
//!     direction); (start, count) selects an inclusive window, an out-of-range
//!     start is a non-fatal warning + rc 0, an over-large count clamps + warns;
//!   - an element-select target (mem[i]) is a clean loud reject.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, bin: &[u8]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fread_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("data.bin"), bin).unwrap();
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

fn b16() -> Vec<u8> {
    (1u8..=16).collect()
}

#[test]
fn fread_single_reg_big_endian() {
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] w; reg [63:0] w8; reg [7:0] w1; integer fd, rc;\n\
         initial begin\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w,fd); $display(\"A %0d %h\",rc,w); $fclose(fd);\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w8,fd); $display(\"B %0d %h\",rc,w8); $fclose(fd);\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w1,fd); $display(\"C %0d %h\",rc,w1); $fclose(fd);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(out.contains("A 4 01020304"), "{out}");
    assert!(out.contains("B 8 0102030405060708"), "{out}");
    assert!(out.contains("C 1 01"), "{out}");
}

#[test]
fn fread_non_byte_multiple_keeps_low_bits() {
    // reg[12:0] reads ceil(13/8)=2 bytes (0xFFFF), the LOW 13 bits survive =>
    // 0x1fff (iverilog-pinned; the design's "0x0fff top-nibble-drop" was wrong).
    let (out, _c) = run(
        "module t;\n\
         reg [12:0] w13; integer fd, rc;\n\
         initial begin\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w13,fd); $display(\"D %0d %h\",rc,w13);\n\
         end\n\
         endmodule\n",
        &[0xFF, 0xFF],
    );
    assert!(out.contains("D 2 1fff"), "{out}");
}

#[test]
fn fread_partial_fill_keeps_prior_low_bytes() {
    // reading fewer bytes than the reg width fills the MSB side; the unread LOW
    // bytes keep their prior value. DEADBEEF + 1 byte AB => ABADBEEF (rc=1);
    // + 2 bytes ABCD => ABCDBEEF (rc=2).
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] w; integer fd, rc;\n\
         initial begin\n\
           w=32'hDEADBEEF; fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w,fd); $display(\"E1 %0d %h\",rc,w); $fclose(fd);\n\
         end\n\
         endmodule\n",
        &[0xAB],
    );
    assert!(out.contains("E1 1 abadbeef"), "{out}");
    let (out2, _c) = run(
        "module t;\n\
         reg [31:0] w; integer fd, rc;\n\
         initial begin\n\
           w=32'hDEADBEEF; fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(w,fd); $display(\"E2 %0d %h\",rc,w);\n\
         end\n\
         endmodule\n",
        &[0xAB, 0xCD],
    );
    assert!(out2.contains("E2 2 abcdbeef"), "{out2}");
}

#[test]
fn fread_memory_fills_ascending_regardless_of_declared_direction() {
    // mem[3:0] declared descending still fills index 0 first (the base).
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] memd [3:0]; integer fd, rc;\n\
         initial begin\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(memd,fd);\n\
           $display(\"G %0d %h %h\", rc, memd[0], memd[3]);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(out.contains("G 16 01020304 0d0e0f10"), "{out}");
}

#[test]
fn fread_start_count_window() {
    // $fread(mem, fd, 1, 2) fills mem[1], mem[2] only; rc = 8 bytes.
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] mem [0:7]; integer fd, rc, i;\n\
         initial begin\n\
           for (i=0;i<8;i=i+1) mem[i]=32'heeeeeeee;\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem,fd,1,2);\n\
           $display(\"H %0d %h %h %h %h\", rc, mem[0], mem[1], mem[2], mem[3]);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(
        out.contains("H 8 eeeeeeee 01020304 05060708 eeeeeeee"),
        "{out}"
    );
}

#[test]
fn fread_count_too_large_clamps_and_start_oor_is_noop() {
    // over-large count clamps to the available range (start=6,count=10 on [0:7]
    // => fills mem[6],mem[7]); an out-of-range start fills nothing (rc 0). Both
    // are non-fatal (warning + continue).
    let (out, code) = run(
        "module t;\n\
         reg [31:0] mem [0:7]; integer fd, rc, i;\n\
         initial begin\n\
           for (i=0;i<8;i=i+1) mem[i]=32'h0;\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem,fd,6,10); $display(\"HC %0d %h %h\", rc, mem[6], mem[7]); $fclose(fd);\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem,fd,99,1); $display(\"HO %0d\", rc);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(out.contains("HC 8 01020304 05060708"), "{out}");
    assert!(out.contains("HO 0"), "{out}");
    assert_eq!(code, Some(0), "range issues are non-fatal");
}

#[test]
fn fread_partial_final_element() {
    // a 6-byte file into reg[31:0] mem[0:3] (preset DEADBEEF): mem[0] full
    // (a1a2a3a4), mem[1] partial (a5a6 MSB-side, low bytes keep beef), mem[2]
    // untouched. rc = 6.
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] mem [0:3]; integer fd, rc, i;\n\
         initial begin\n\
           for (i=0;i<4;i=i+1) mem[i]=32'hDEADBEEF;\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem,fd);\n\
           $display(\"P %0d %h %h %h\", rc, mem[0], mem[1], mem[2]);\n\
         end\n\
         endmodule\n",
        &[0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6],
    );
    assert!(out.contains("P 6 a1a2a3a4 a5a6beef deadbeef"), "{out}");
}

#[test]
fn fread_xz_count_coerces_to_zero_not_absent() {
    // a count with x/z bits is NOT treated as absent: iverilog coerces each x/z
    // bit to 0. count 4'b001x => 2 (fills mem[0],mem[1]; mem[2] untouched),
    // rc=8 bytes. (vita previously read None => filled all 6 — silent-wrong.)
    let (out, _c) = run(
        "module t;\n\
         reg [31:0] mem [0:5]; integer fd, rc, i;\n\
         initial begin\n\
           for (i=0;i<6;i=i+1) mem[i]=32'heeeeeeee;\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem,fd,0,4'b001x);\n\
           $display(\"FX %0d %h %h %h\", rc, mem[0], mem[1], mem[2]);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(
        out.contains("FX 8 01020304 05060708 eeeeeeee"),
        "x/z count coerces to 2 (not all):\n{out}"
    );
}

#[test]
fn fread_element_select_target_is_loud() {
    let (out, code) = run(
        "module t;\n\
         reg [31:0] mem [0:3]; integer fd, rc;\n\
         initial begin\n\
           fd=$fopen(\"data.bin\",\"rb\"); rc=$fread(mem[0],fd);\n\
         end\n\
         endmodule\n",
        &b16(),
    );
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "element-select $fread target must be loud: {out} code={code:?}"
    );
}
