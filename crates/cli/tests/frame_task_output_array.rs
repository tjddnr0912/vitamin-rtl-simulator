//! §4.5.193 UARR: an `output` unpacked-FIXED array formal on a TASK (`output byte
//! digest[N]`) loud→supported — the reviewer's `shaN_compute`/`hex2bytes`-style
//! output. The body writes the same md-packed `[count][elem_w]` slot as an INPUT
//! formal (§4.5.188); at the task's exit the whole slot is copied to a fresh caller-
//! side packed temp (a normal scalar out-bind), then UNPACKED into the caller array
//! elements (`caller[i] = packed[i*ew +: ew]`) after the call returns. No heap, no
//! format change — works on the synchronous and suspendable frame paths.
//!
//! Correct-or-loud: an INOUT array formal, a non-bare array actual (a slice), and a
//! classifier-rejected shape stay loud. IEEE §13.5.2 pass-by-value: the formal starts
//! at its default (0 for a 2-state element, X for 4-state), so an element the body
//! never writes copies out that default (the caller's prior value is overwritten).
//!
//! iverilog rejects unpacked subroutine ports, so the oracle is an element-wise
//! reference computing the same values.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ftoa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn output_byte_array_suspendable() {
    let o = run(
        "module top;\n\
         task automatic fill(output byte b[4]);\n\
           for (int i=0;i<4;i++) b[i] = i*10 + 1;\n\
           $display(\"done\");\n\
         endtask\n\
         byte data[4];\n\
         initial begin fill(data); $display(\"%0d %0d %0d %0d\", data[0],data[1],data[2],data[3]); end\n\
         endmodule\n",
    );
    assert!(o.contains("1 11 21 31"), "got:\n{o}");
}

#[test]
fn output_array_nonsuspendable_with_input() {
    // Combined input + output arrays, no timing (synchronous frame path).
    let o = run(
        "module top;\n\
         task automatic mk(input byte seed, output byte b[3]);\n\
           for (int i=0;i<3;i++) b[i] = seed + i;\n\
         endtask\n\
         byte data[3];\n\
         initial begin mk(8'd100, data); $display(\"%0d %0d %0d\", data[0],data[1],data[2]); end\n\
         endmodule\n",
    );
    assert!(o.contains("100 101 102"), "got:\n{o}");
}

#[test]
fn output_wide_element() {
    let o = run(
        "module top;\n\
         task automatic gen(output logic [15:0] x[3]);\n\
           x[0]=16'hAAAA; x[1]=16'hBBBB; x[2]=16'hCCCC;\n\
         endtask\n\
         logic [15:0] d[3];\n\
         initial begin gen(d); $display(\"%h %h %h\", d[0],d[1],d[2]); end\n\
         endmodule\n",
    );
    assert!(o.contains("aaaa bbbb cccc"), "got:\n{o}");
}

#[test]
fn partial_write_uses_formal_default() {
    // IEEE §13.5.2: the output formal starts at its default (0 for a 2-state byte);
    // an element the body never writes copies out 0, overwriting the caller's prior FF.
    let o = run(
        "module top;\n\
         task automatic t(output byte b[3]); b[1]=8'h22; endtask\n\
         byte d[3];\n\
         initial begin d[0]=8'hFF; d[1]=8'hFF; d[2]=8'hFF; t(d); $display(\"%h %h %h\", d[0],d[1],d[2]); end\n\
         endmodule\n",
    );
    assert!(o.contains("00 22 00"), "got:\n{o}");
}

#[test]
fn two_calls_isolated() {
    let o = run(
        "module top;\n\
         task automatic t(input byte s, output byte b[2]); b[0]=s; b[1]=s+1; endtask\n\
         byte x[2]; byte y[2];\n\
         initial begin t(8'd10,x); t(8'd20,y); $display(\"%0d %0d %0d %0d\", x[0],x[1],y[0],y[1]); end\n\
         endmodule\n",
    );
    assert!(o.contains("10 11 20 21"), "got:\n{o}");
}

#[test]
fn signed_output_elements() {
    let o = run(
        "module top;\n\
         task automatic t(output byte b[2]); b[0]=-5; b[1]=-100; endtask\n\
         byte d[2];\n\
         initial begin t(d); $display(\"%0d %0d\", d[0], d[1]); end\n\
         endmodule\n",
    );
    assert!(o.contains("-5 -100"), "got:\n{o}");
}

#[test]
fn inout_array_formal_stays_loud() {
    let o = run(
        "module top;\n\
         task automatic t(inout byte b[3]); b[0]=b[0]+1; endtask\n\
         byte d[3];\n\
         initial begin d[0]=5; t(d); $display(\"%0d\", d[0]); end\n\
         endmodule\n",
    );
    assert!(o.contains("E3009"), "should be loud:\n{o}");
}

#[test]
fn non_bare_output_actual_stays_loud() {
    let o = run(
        "module top;\n\
         task automatic t(output byte b[2]); b[0]=1; endtask\n\
         byte d[4];\n\
         initial begin t(d[0:1]); $display(\"%0d\", d[0]); end\n\
         endmodule\n",
    );
    assert!(o.contains("E3009"), "should be loud:\n{o}");
}
