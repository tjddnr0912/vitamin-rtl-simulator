//! Per-element array VCD (Phase-1.x ⑤a): unpacked arrays declare one `$var`
//! per element (`mem[4]`, `g[1][2]`) and element writes land on the right id.
//! v1 declared a single var showing word 0 only.
//!
//! ⚠️ iverilog never dumps memories at all (`$dumpvars` on one errors with
//! "cannot dump a vpiMemory"), so the naming/content here is hand-pinned to
//! the de-facto VCD convention (bracketed declared indices).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_dir(src: &str) -> (std::path::PathBuf, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_vcda_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        d,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn nonzero_lo_array_declares_true_indices() {
    let (d, err, code) = run_dir(
        "module top;\n\
         reg [7:0] m [4:7];\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, top);\n\
           m[4] = 8'h11; m[7] = 8'h77;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    for k in 4..=7 {
        assert!(
            vcd.contains(&format!("m[{k}] $end")),
            "declared index {k}:\n{vcd}"
        );
    }
    assert!(!vcd.contains("m[0]"), "no storage-index leak:\n{vcd}");
    assert!(vcd.contains("b00010001 "), "m[4]=11h change:\n{vcd}");
    assert!(vcd.contains("b01110111 "), "m[7]=77h change:\n{vcd}");
}

#[test]
fn two_d_array_declares_nested_brackets() {
    let (d, err, code) = run_dir(
        "module top;\n\
         reg [3:0] g [0:1][2:3];\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, top);\n\
           g[1][2] = 4'ha;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    for name in ["g[0][2]", "g[0][3]", "g[1][2]", "g[1][3]"] {
        assert!(vcd.contains(&format!("{name} $end")), "{name}:\n{vcd}");
    }
    assert!(vcd.contains("b1010 "), "g[1][2]=a change:\n{vcd}");
}

#[test]
fn staged_pipeline_carries_dims_sidecar() {
    // vcmp → velab → vrun must produce the same per-element names as the
    // one-shot (the dims ride the 10th .velab trailer).
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_vcds_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let src = "module top;\n\
         reg [7:0] m [4:6];\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, top);\n\
           m[5] = 8'h55;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n";
    std::fs::write(d.join("t.sv"), src).unwrap();
    let vita = env!("CARGO_BIN_EXE_vita");
    let run = |args: &[&str]| {
        let out = Command::new(vita)
            .args(args)
            .current_dir(&d)
            .output()
            .expect("run");
        assert!(
            out.status.success(),
            "step {args:?}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&["vcmp", "t.sv", "-o", "t.vu"]);
    run(&["velab", "t.vu", "-o", "t.velab"]);
    run(&["vrun", "t.velab"]);
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    for k in 4..=6 {
        assert!(
            vcd.contains(&format!("m[{k}] $end")),
            "staged element {k}:\n{vcd}"
        );
    }
    assert!(vcd.contains("b01010101 "), "m[5]=55h change:\n{vcd}");
}

// ── ⑤b: $dumpvars depth/scope/net selection ────────────────────────────

#[test]
fn dumpvars_level_one_excludes_child_scope() {
    // iverilog-pinned: $dumpvars(1, top) = top's OWN vars only.
    let (d, err, code) = run_dir(
        "module sub(input wire si);\n\
         reg sr;\n\
         always @(si) sr = si;\n\
         endmodule\n\
         module top;\n\
         reg a; wire w;\n\
         assign w = a;\n\
         sub u(.si(a));\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(1, top);\n\
           a = 0; #2 a = 1; #2 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    assert!(vcd.contains(" a $end"), "own var a:\n{vcd}");
    assert!(vcd.contains(" w $end"), "own var w:\n{vcd}");
    assert!(!vcd.contains(" sr $end"), "child var excluded:\n{vcd}");
    assert!(!vcd.contains(" si $end"), "child port excluded:\n{vcd}");
}

#[test]
fn dumpvars_level_zero_includes_subtree() {
    let (d, err, code) = run_dir(
        "module sub(input wire si);\n\
         reg sr;\n\
         always @(si) sr = si;\n\
         endmodule\n\
         module top;\n\
         reg a;\n\
         sub u(.si(a));\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, top);\n\
           a = 0; #2 a = 1; #2 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    assert!(vcd.contains(" a $end"), "got:\n{vcd}");
    assert!(vcd.contains(" sr $end"), "subtree var included:\n{vcd}");
}

#[test]
fn dumpvars_child_scope_arg_selects_subtree_only() {
    // `$dumpvars(0, u)` from inside top resolves via the elaborate fq
    // candidate to top.u — only the child's vars appear.
    let (d, err, code) = run_dir(
        "module sub(input wire si);\n\
         reg sr;\n\
         always @(si) sr = si;\n\
         endmodule\n\
         module top;\n\
         reg a;\n\
         sub u(.si(a));\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, u);\n\
           a = 0; #2 a = 1; #2 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    assert!(vcd.contains(" sr $end"), "child var:\n{vcd}");
    assert!(vcd.contains(" si $end"), "child port:\n{vcd}");
    assert!(!vcd.contains(" a $end"), "parent var excluded:\n{vcd}");
}

#[test]
fn dumpvars_net_arg_selects_single_net_and_array_arg_expands() {
    // A net arg dumps just that net; an ARRAY arg (iverilog errors on these)
    // is a hand-extension dumping the array per-element.
    let (d, err, code) = run_dir(
        "module top;\n\
         reg a, b;\n\
         reg [3:0] mem [0:1];\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(1, b, mem);\n\
           a = 0; b = 1; mem[1] = 4'h9;\n\
           #1 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    assert!(vcd.contains(" b $end"), "selected net:\n{vcd}");
    assert!(vcd.contains("mem[0] $end"), "array arg element 0:\n{vcd}");
    assert!(vcd.contains("mem[1] $end"), "array arg element 1:\n{vcd}");
    assert!(!vcd.contains(" a $end"), "unselected net excluded:\n{vcd}");
}

#[test]
fn second_dumpvars_call_warns_once_and_is_ignored() {
    let (d, err, code) = run_dir(
        "module top;\n\
         reg a;\n\
         initial begin\n\
           $dumpfile(\"w.vcd\"); $dumpvars(0, top);\n\
           $dumpvars(0, top);\n\
           $dumpvars(0, top);\n\
           a = 1; #1 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert_eq!(
        err.matches("VITA-W4021").count(),
        1,
        "exactly one W4021:\n{err}"
    );
    assert!(d.join("w.vcd").exists());
}
