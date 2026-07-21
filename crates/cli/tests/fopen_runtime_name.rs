//! §4.5.187 file-I/O: `$fopen(name[, mode])` accepts a RUNTIME filename — a
//! `string` variable, a string concatenation, or a packed reg holding ASCII —
//! not only a string literal. The engine `k_fopen` resolves all three forms
//! (Const{StrUtf8} → is_str runtime value → packed-chars). Pinned LIVE against
//! iverilog 13.0: a file-driven testbench that builds its path in a variable
//! opens the same file vita opens.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_in_dir(src: &str) -> (String, String, Option<i32>, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fopenrt_{}_{n}", std::process::id()));
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
fn fopen_string_variable_name() {
    // A `string` variable holds the path; write then reopen+read confirms the
    // same file was opened for both. (iverilog: "got: hello 42".)
    let (out, err, code, _d) = run_in_dir(
        "module top;\n\
         initial begin\n\
           string fn; int fd; string line;\n\
           fn = \"rt_var.txt\";\n\
           fd = $fopen(fn, \"w\");\n\
           $fwrite(fd, \"hello %0d\\n\", 42);\n\
           $fclose(fd);\n\
           fd = $fopen(fn, \"r\");\n\
           void'($fgets(line, fd));\n\
           $write(\"got: %s\", line);\n\
           $fclose(fd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("got: hello 42"), "got:\n{out}");
}

#[test]
fn fopen_concat_name_writes_expected_file() {
    // A concatenated path `{"pre_", base, ".txt"}` opens `pre_data.txt`.
    let (out, err, code, d) = run_in_dir(
        "module top;\n\
         initial begin\n\
           string base; int fd;\n\
           base = \"data\";\n\
           fd = $fopen({\"pre_\", base, \".txt\"}, \"w\");\n\
           $fdisplay(fd, \"cat=%0d\", 7);\n\
           $fclose(fd);\n\
           $display(\"opened=%0d\", (fd != 0));\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("opened=1"), "got:\n{out}");
    let contents = std::fs::read_to_string(d.join("pre_data.txt")).expect("pre_data.txt written");
    assert_eq!(contents, "cat=7\n");
}

#[test]
fn fscanf_reads_vectors_from_variable_named_file() {
    // The CAVP-walker pattern: build the path in a variable, write vectors,
    // reopen, parse with $fscanf. iverilog-pinned decimal + hex reads.
    let (out, err, code, _d) = run_in_dir(
        "module top;\n\
         initial begin\n\
           string fn; int fd, a, b, n; logic [31:0] h;\n\
           fn = \"vec.txt\";\n\
           fd = $fopen(fn, \"w\");\n\
           $fdisplay(fd, \"10 20\");\n\
           $fdisplay(fd, \"deadbeef\");\n\
           $fclose(fd);\n\
           fd = $fopen(fn, \"r\");\n\
           n = $fscanf(fd, \"%d %d\", a, b);\n\
           $display(\"n=%0d a=%0d b=%0d\", n, a, b);\n\
           n = $fscanf(fd, \"%h\", h);\n\
           $display(\"n=%0d h=%h\", n, h);\n\
           $fclose(fd);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("n=2 a=10 b=20"), "got:\n{out}");
    assert!(out.contains("n=1 h=deadbeef"), "got:\n{out}");
}

#[test]
fn fopen_open_failure_returns_zero() {
    // A runtime path into a nonexistent directory opened for read fails → 0
    // (IEEE §21.3), same as a literal (no silent success from the relaxed gate).
    let (out, err, code, _d) = run_in_dir(
        "module top;\n\
         initial begin\n\
           string fn; int fd;\n\
           fn = \"/no_such_dir_vita_xyz/f.txt\";\n\
           fd = $fopen(fn, \"r\");\n\
           $display(\"fd0=%0d\", (fd == 0));\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("fd0=1"), "got:\n{out}");
}
