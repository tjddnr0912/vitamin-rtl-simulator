//! v7 file I/O: $fopen/$fclose/$fdisplay/$fwrite. fd values, MCD bit
//! channels, and bad-fd behavior pinned LIVE against iverilog 13.0
//! (2026-06-12, probe t9): mode-form fds are 0x8000_0003, 0x8000_0004, …;
//! MCD-form opens take channel bits from bit 1 (bit 0 = stdout); a write to
//! a closed fd warns and is dropped (run still exits 0).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_in_dir(src: &str) -> (String, String, Option<i32>, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fio_{}_{n}", std::process::id()));
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
        d,
    )
}

#[test]
fn fopen_fd_values_and_file_contents() {
    // iverilog-pinned: fd=80000003 fd2=80000004 mcd=00000002; out1 holds the
    // fdisplay line + the two fwrites (no newline between them).
    let (out, err, code, d) = run_in_dir(
        "module top;\n\
         integer fd, fd2, mcd;\n\
         initial begin\n\
           fd = $fopen(\"out1.txt\", \"w\");\n\
           fd2 = $fopen(\"out2.txt\", \"w\");\n\
           mcd = $fopen(\"out3.txt\");\n\
           $display(\"fd=%h fd2=%h mcd=%h\", fd, fd2, mcd);\n\
           $fdisplay(fd, \"line %0d ok\", 42);\n\
           $fwrite(fd, \"no-newline %h\", 8'hab);\n\
           $fwrite(fd, \"+tail\\n\");\n\
           $fdisplay(mcd, \"mcd line\");\n\
           $fclose(fd);\n\
           $fclose(mcd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("fd=80000003 fd2=80000004 mcd=00000002"),
        "got:\n{out}"
    );
    let out1 = std::fs::read_to_string(d.join("out1.txt")).expect("out1.txt written");
    assert_eq!(out1, "line 42 ok\nno-newline ab+tail\n");
    let out3 = std::fs::read_to_string(d.join("out3.txt")).expect("out3.txt written");
    assert_eq!(out3, "mcd line\n");
    assert!(
        d.join("out2.txt").exists(),
        "open alone must create the file"
    );
}

#[test]
fn write_to_closed_fd_warns_and_drops() {
    // iverilog: runtime warning, nothing written, exit 0.
    let (_out, err, code, d) = run_in_dir(
        "module top;\n\
         integer fd;\n\
         initial begin\n\
           fd = $fopen(\"c.txt\", \"w\");\n\
           $fdisplay(fd, \"kept\");\n\
           $fclose(fd);\n\
           $fdisplay(fd, \"dropped\");\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(err.contains("W4022"), "want bad-fd warning, got:\n{err}");
    let c = std::fs::read_to_string(d.join("c.txt")).unwrap();
    assert_eq!(c, "kept\n");
}

#[test]
fn mcd_bit_zero_broadcasts_to_stdout() {
    // MCD bit 0 = stdout: writing to (mcd | 1) hits both the file and stdout.
    let (out, err, code, d) = run_in_dir(
        "module top;\n\
         integer mcd;\n\
         initial begin\n\
           mcd = $fopen(\"m.txt\");\n\
           $fdisplay(mcd | 32'd1, \"both %0d\", 7);\n\
           $fclose(mcd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("both 7"), "stdout leg, got:\n{out}");
    let m = std::fs::read_to_string(d.join("m.txt")).unwrap();
    assert_eq!(m, "both 7\n");
}

#[test]
fn fdisplay_bare_args_default_decimal() {
    // No-format fdisplay joins args like $display (default decimal).
    let (_out, err, code, d) = run_in_dir(
        "module top;\n\
         integer fd;\n\
         reg [7:0] a;\n\
         initial begin\n\
           fd = $fopen(\"b.txt\", \"w\");\n\
           a = 8'd200;\n\
           $fdisplay(fd, a);\n\
           $fclose(fd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let b = std::fs::read_to_string(d.join("b.txt")).unwrap();
    assert_eq!(b, "200\n");
}

#[test]
fn fopen_outside_direct_rhs_is_loud() {
    let (_out, err, code, _d) = run_in_dir(
        "module top;\n\
         integer fd;\n\
         initial begin\n\
           fd = $fopen(\"x.txt\", \"w\") + 0;\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_ne!(code, Some(0));
    assert!(err.contains("E3009"), "stderr:\n{err}");
}

#[test]
fn append_mode_accumulates() {
    let (_out, err, code, d) = run_in_dir(
        "module top;\n\
         integer fd;\n\
         initial begin\n\
           fd = $fopen(\"a.txt\", \"w\");\n\
           $fdisplay(fd, \"one\");\n\
           $fclose(fd);\n\
           fd = $fopen(\"a.txt\", \"a\");\n\
           $fdisplay(fd, \"two\");\n\
           $fclose(fd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let a = std::fs::read_to_string(d.join("a.txt")).unwrap();
    assert_eq!(a, "one\ntwo\n");
}

// MCD-RECLAIM (ROADMAP §5.3, 2026-06-23): MCD channel bits were handed out by
// a monotonic counter (`next_mcd_bit`) and never reclaimed on $fclose, so the
// 30-channel space exhausted after 30 lifetime opens even if all were closed.
// Allocate the lowest currently-unused bit instead (iverilog reclaims), so a
// closed bit is reused. Byte-identical when nothing is freed (lowest-unused ==
// next sequential), so fopen_fd_values_and_file_contents still sees mcd=2.
#[test]
fn mcd_channel_bit_is_reclaimed_on_close() {
    let (out, err, code, _d) = run_in_dir(
        "module top;\n\
         integer m1, m2, m3;\n\
         initial begin\n\
           m1 = $fopen(\"a.txt\");\n\
           m2 = $fopen(\"b.txt\");\n\
           $fclose(m1);\n\
           m3 = $fopen(\"c.txt\");\n\
           $display(\"m1=%0d m2=%0d m3=%0d\", m1, m2, m3);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // bit1=2, bit2=4; closing m1 frees bit1, so m3 reuses bit1 (=2), not bit3 (=8).
    assert!(out.contains("m1=2 m2=4 m3=2"), "got:\n{out}");
}

// FD-RECLAIM (ROADMAP §5.3, 2026-06-23): a readable fd's auxiliary maps
// (read_state/readable_fds/bad_fd_warned) were not cleaned on $fclose and
// next_fd was an unguarded `+= 1`. The fix (saturating add + aux-map cleanup)
// has no observable output change (fd numbers stay monotonic), so this is a
// regression characterization: repeatedly open/close readable fds and confirm
// the run stays correct.
#[test]
fn readable_fd_open_close_cycle_stays_correct() {
    let (out, err, code, _d) = run_in_dir(
        "module top;\n\
         integer fd, k;\n\
         initial begin\n\
           for (k = 0; k < 8; k = k + 1) begin\n\
             fd = $fopen(\"r.txt\", \"w+\");\n\
             $fdisplay(fd, \"row %0d\", k);\n\
             $fclose(fd);\n\
           end\n\
           $display(\"ok\");\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("ok"), "got:\n{out}");
}
