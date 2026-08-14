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

/// Slice #8 ABSOLUTE ANCHOR — `$writemem*` on TIER-3, iverilog-pinned.
///
/// This was the LAST entry in `systask_refusal` a corpus design reached, and its
/// reason was right and specific: "it reads the MEMORY itself, not a formatted
/// argument". That is three reads — the two window bounds and the per-element
/// value — and unthreaded they all see the engine's untouched slots, so a
/// native run writes a file full of declaration initializers. A file that
/// EXISTS and is wrong, which no exit code reports.
///
/// Everything here is a discriminator:
///   * the elements are written by a LOOP at runtime (a decl-init memory would
///     agree by accident, exactly the trap slice #3's `new[]` anchor hit);
///   * the window bounds are MODULE NETS, not literals;
///   * one window DESCENDS, so the step sign rides the same reads;
///   * one element is partly X, so the nibble compression is exercised on a
///     value only the right store has.
///
/// ⚠️ The descending row kills its mutant by NON-TERMINATION, not by a value: a
/// `step` that stays +1 while `finish < start` can never reach the loop's only
/// exit, so the mutant appends to `body` forever. Measured the hard way — that
/// mutation ran twice through the full battery and took the machine to ~33 GB
/// RSS and a userspace-watchdog kernel panic. Score this row with a bounded
/// runner, never by waiting for the suite (LOOPROMPT §4).
#[test]
fn writemem_on_tier_3_matches_iverilog() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wm_nat_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           reg [11:0] mem [0:7];\n\
           reg [7:0] lo = 8'd2; reg [7:0] hi = 8'd5;\n\
           integer i;\n\
           initial begin\n\
             for (i = 0; i < 8; i = i + 1) mem[i] = 12'h100 + i[11:0];\n\
             mem[3] = 12'hxA5;\n\
             $writememh(\"wh.txt\", mem);\n\
             $writememh(\"wwin.txt\", mem, lo, hi);\n\
             $writememb(\"wb.txt\", mem, hi, lo);\n\
             $display(\"done\"); $finish;\n\
           end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    // ANTI-VACUITY: a refused design falls back to the VM, and every other test
    // in this file already measures that.
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{txt}"
    );
    assert_eq!(
        read_out(&d, "wh.txt"),
        "// 0x00000000\n100\n101\n102\nxa5\n104\n105\n106\n107\n"
    );
    assert_eq!(
        read_out(&d, "wwin.txt"),
        "// 0x00000000\n102\nxa5\n104\n105\n"
    );
    assert_eq!(
        read_out(&d, "wb.txt"),
        "// 0x00000000\n000100000101\n000100000100\nxxxx10100101\n000100000010\n"
    );
}

/// Slice #8's REACHABILITY argument, pinned — the two shapes that would hand
/// `read_task_net` a net the arena does not own.
///
/// The seam threads the whole-net read as `nets.read_net(net, word)` on the BARE
/// arena, not through `HeapRouted` the way `eval_task_arg` threads an
/// expression. That is only sound because a `$writemem*` target can never be a
/// heap-kind or frame-local net — and `NetArena::read_net`'s ownership guard is
/// a `debug_assert!`, so if it ever could, a release build would read a slot
/// that is not the value. This is measured rather than assumed:
///
///   * a DYNAMIC array target is refused by elaborate (`E3009` — a
///     dynamic-storage handle has no whole-value surface). iverilog refuses the
///     same design ("second argument must be a memory"), so vita is not behind.
///   * a whole unpacked-array LOCAL of an `automatic` task is refused by
///     elaborate (`E3009` — a whole unpacked-array formal has no value here).
///     ⚠️ iverilog RUNS that one, so this half is an honest-loud vita gap, not
///     parity — recorded in ROADMAP §3. What matters here is that it cannot
///     reach the seam, and if that gate is ever opened this test fails first.
///
/// A frame-local BOUND on a module memory is a different shape and is supported:
/// `writemem_on_tier_3_matches_iverilog`'s window comes from module nets and the
/// task-formal spelling is covered by the differential.
#[test]
fn writemem_targets_the_seam_cannot_own_are_refused_before_the_backend() {
    for (src, want) in [
        (
            "module top;\n\
               int dq [];\n\
               initial begin dq = new[2]; dq[0] = 1; $writememh(\"x.txt\", dq); end\n\
             endmodule\n",
            "dynamic-storage handle has no whole-value surface",
        ),
        (
            "module top;\n\
               task automatic t;\n\
                 reg [7:0] loc [0:1];\n\
                 begin loc[0] = 8'h1; $writememh(\"x.txt\", loc); end\n\
               endtask\n\
               initial t;\n\
             endmodule\n",
            "whole unpacked-array formal has no value here",
        ),
    ] {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vita_wm_ref_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("t.sv");
        std::fs::write(&f, src).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg("--backend")
            .arg("native")
            .arg(f.to_str().unwrap())
            .current_dir(&d)
            .output()
            .expect("run vita");
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(
            out.status.code(),
            Some(1),
            "expected a loud refusal:\n{err}"
        );
        assert!(err.contains("VITA-E3009"), "expected E3009:\n{err}");
        assert!(err.contains(want), "expected {want:?}:\n{err}");
        assert!(!d.join("x.txt").exists(), "a refused design wrote a file");
    }
}
