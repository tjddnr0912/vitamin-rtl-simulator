//! A SYS-READ system function ($sscanf/$fscanf/$fgets/$fread) called as a BARE
//! statement (return value discarded) must still WRITE its destination
//! arguments. vita only emitted the write from the assignment-rhs path
//! (`n = $sscanf(...)`), so a bare `$sscanf(str, fmt, a, b);` silently left
//! a/b unchanged (0/x) while iverilog wrote them — a broad silent-wrong (most
//! code calls scanf as a statement without capturing the count). The bare
//! statement now routes through the same write helper with the count discarded.
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bsr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn bare_sscanf_writes_dests() {
    // A bare `$sscanf` (return discarded) writes every matched destination.
    let out = run("module m; int a, b; initial begin \
         $sscanf(\"12 34\", \"%d %d\", a, b); $display(\"R=%0d %0d\", a, b); #1 $finish; end \
       endmodule\n");
    assert!(out.contains("R=12 34"), "bare $sscanf %d %d; got:\n{out}");
}

#[test]
fn bare_sscanf_hex_and_string() {
    let out = run("module m; logic [7:0] a, b; string s; initial begin \
         $sscanf(\"ab cd\", \"%h %h\", a, b); \
         $sscanf(\"hi\", \"%s\", s); \
         $display(\"R=%h %h %s\", a, b, s); #1 $finish; end endmodule\n");
    assert!(
        out.contains("R=ab cd hi"),
        "bare $sscanf %h/%s; got:\n{out}"
    );
}

#[test]
fn bare_sscanf_single() {
    // Single %d, return discarded.
    let out = run("module m; int a; initial begin \
         $sscanf(\"42\", \"%d\", a); $display(\"R=%0d\", a); #1 $finish; end endmodule\n");
    assert!(out.contains("R=42"), "bare $sscanf single %d; got:\n{out}");
}

#[test]
fn bare_fscanf_writes_dest() {
    let out = run("module m; int a, fd; initial begin \
         fd = $fopen(\"/tmp/vita_bsr_fscanf.txt\", \"w\"); $fwrite(fd, \"42\"); $fclose(fd); \
         fd = $fopen(\"/tmp/vita_bsr_fscanf.txt\", \"r\"); \
         $fscanf(fd, \"%d\", a); $fclose(fd); \
         $display(\"R=%0d\", a); #1 $finish; end endmodule\n");
    assert!(out.contains("R=42"), "bare $fscanf; got:\n{out}");
}

#[test]
fn bare_fgets_writes_dest() {
    let out = run("module m; string s; int fd; initial begin \
         fd = $fopen(\"/tmp/vita_bsr_fgets.txt\", \"w\"); $fwrite(fd, \"hi\"); $fclose(fd); \
         fd = $fopen(\"/tmp/vita_bsr_fgets.txt\", \"r\"); \
         $fgets(s, fd); $fclose(fd); \
         $display(\"R=%s\", s); #1 $finish; end endmodule\n");
    assert!(out.contains("R=hi"), "bare $fgets; got:\n{out}");
}

#[test]
fn bare_fread_writes_dest() {
    let out = run("module m; reg [7:0] mem[0:1]; int fd; initial begin \
         fd = $fopen(\"/tmp/vita_bsr_fread.bin\", \"w\"); $fwrite(fd, \"%c%c\", 8, 9); $fclose(fd); \
         fd = $fopen(\"/tmp/vita_bsr_fread.bin\", \"r\"); \
         $fread(mem, fd); $fclose(fd); \
         $display(\"R=%0d\", mem[0]); #1 $finish; end endmodule\n");
    assert!(out.contains("R=8"), "bare $fread; got:\n{out}");
}

#[test]
fn assign_form_still_writes_dests_and_count() {
    // Regression guard: the assignment form (`n = $sscanf(...)`) still writes the
    // dests AND returns the count — byte-identical to before the refactor.
    let out = run("module m; int a, b, n; initial begin \
         n = $sscanf(\"5 6\", \"%d %d\", a, b); $display(\"R=%0d %0d %0d\", n, a, b); #1 $finish; end \
       endmodule\n");
    assert!(
        out.contains("R=2 5 6"),
        "assign-form $sscanf count+dests; got:\n{out}"
    );
}

#[test]
fn bare_sysread_in_frame_function_does_not_hard_error() {
    // A bare sys-read inside a FRAME function/task/method body stays on the
    // pre-existing warn+skip path (the scanf can't execute under run_frame_call);
    // the routing is gated to PROCESS bodies, so it must NOT hard-error with a
    // misleading "assignment to a net outside the function". Here `a` stays 0
    // (skipped), so parse("7") = 0 + 100 = 100 — the elaborated (not rejected)
    // result, byte-identical to before this change.
    let out = run("module m;\n\
         function automatic int parse(input string s); int a;\n\
           begin $sscanf(s, \"%d\", a); parse = a + 100; end endfunction\n\
         initial begin $display(\"R=%0d\", parse(\"7\")); #1 $finish; end endmodule\n");
    assert!(
        out.contains("R=100"),
        "frame-body bare sys-read must elaborate (warn+skip), not hard-error:\n{out}"
    );
}
