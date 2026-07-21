//! Family A (round-17): a queue / dynamic array of an all-packable UNPACKED record,
//! end-to-end. §4.5.191 opened FIXED arrays of packable records; this opens the
//! QUEUE (`[$]`) form (parser routes `Dim::Queue(None)` through the same packed-vector
//! lowering as a packed-struct queue) AND unifies packable struct VARIABLES + struct
//! tf-ports on a whole-vector representation so `q.push_back(p)` / `r = q[i]` /
//! `f(structvar)` all work. iverilog does not support unpacked structs (nor queue-of-
//! struct), so these are hand-IEEE / self-consistent with the packed-struct path
//! (which iverilog does confirm for the value semantics).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_qrec_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// The report's exact Family A repro: declare a queue of a record, push a struct var,
// read a field of an element.
#[test]
fn queue_of_record_push_and_field_read() {
    let o = run("module t;\n\
         typedef struct { logic [31:0] addr; logic [7:0] len; } pkt_t;\n\
         pkt_t q[$];\n\
         initial begin pkt_t p; p.addr=32'h1000; p.len=8'd4; q.push_back(p);\n\
           if (q[0].len==8'd4) $display(\"PASS\"); $finish; end endmodule\n");
    assert!(o.contains("PASS"), "queue-of-record push+field read:\n{o}");
}

// Multiple pushes, size, per-element field read, and a WHOLE-element read into a
// struct var (`r = q[1]`).
#[test]
fn queue_of_record_multi_push_size_whole_read() {
    let o = run("module t;\n\
         typedef struct { logic [31:0] addr; logic [7:0] len; } pkt_t;\n\
         pkt_t q[$];\n\
         initial begin pkt_t p; pkt_t r;\n\
           p.addr=32'h10; p.len=8'd4; q.push_back(p);\n\
           p.addr=32'h20; p.len=8'd7; q.push_back(p);\n\
           r = q[1];\n\
           $display(\"size=%0d q0len=%0d q1addr=%h rlen=%0d\", q.size(), q[0].len, q[1].addr, r.len);\n\
           $finish; end endmodule\n");
    assert!(
        o.contains("size=2 q0len=4 q1addr=00000020 rlen=7"),
        "queue-of-record multi/size/whole-read:\n{o}"
    );
}

// A packable struct VARIABLE passes WHOLE to a struct tf-port (module scope).
#[test]
fn packable_struct_var_passes_whole_to_tf_port() {
    let o = run("package p; typedef struct { int a; int b; } rec_t; endpackage\n\
         module t; import p::*; rec_t g;\n\
         function automatic int addr(input rec_t r); return r.a + r.b; endfunction\n\
         initial begin g.a=10; g.b=20; if(addr(g)==30) $display(\"PASS=%0d\", addr(g)); $finish; end endmodule\n");
    assert!(
        o.contains("PASS=30"),
        "module struct var → tf-port whole:\n{o}"
    );
}

// Block-local packable struct var → tf-port (the scope the reviewer hit).
#[test]
fn block_local_struct_var_passes_whole() {
    let o = run("package p; typedef struct { int a; int b; } rec_t; endpackage\n\
         module t; import p::*;\n\
         function automatic int addr(input rec_t r); return r.a + r.b; endfunction\n\
         initial begin rec_t r; r.a=7; r.b=8; if(addr(r)==15) $display(\"PASS=%0d\", addr(r)); $finish; end endmodule\n");
    assert!(
        o.contains("PASS=15"),
        "block-local struct var → tf-port whole:\n{o}"
    );
}

// Whole-value copy `q = p` has VALUE semantics — mutating the copy leaves the source.
#[test]
fn whole_struct_copy_is_by_value() {
    let o = run(
        "module t; typedef struct { logic [7:0] x; logic [7:0] y; } pr;\n\
         initial begin pr p; pr q; p.x=8'hAA; p.y=8'hBB; q=p; q.x=8'h11;\n\
           $display(\"q=%h_%h p=%h_%h\", q.x,q.y, p.x,p.y); $finish; end endmodule\n",
    );
    assert!(
        o.contains("q=11_bb p=aa_bb"),
        "whole struct copy must be by-value:\n{o}"
    );
}

// `'{…}` assignment pattern on a packable struct var (bonus of the whole-vector form).
#[test]
fn assign_pattern_on_packable_struct_var() {
    let o = run("module t; typedef struct { int a; int b; } rec_t;\n\
         initial begin rec_t r; r = '{5, 6}; $display(\"a=%0d b=%0d\", r.a, r.b); $finish; end endmodule\n");
    assert!(o.contains("a=5 b=6"), "'{{…}} on packable struct var:\n{o}");
}

// output struct tf-port — whole-vector copy-OUT (R5-B path with a single formal).
#[test]
fn output_struct_tf_port_copies_out() {
    let o = run("module t; typedef struct { int a; int b; } rec_t;\n\
         task automatic fill(output rec_t r); r.a=7; r.b=9; endtask\n\
         initial begin rec_t r; fill(r); $display(\"a=%0d b=%0d\", r.a, r.b); $finish; end endmodule\n");
    assert!(
        o.contains("a=7 b=9"),
        "output struct tf-port copy-out:\n{o}"
    );
}

// correct-or-loud: a NON-packable (string member) record queue stays loud. (Errors
// print to stderr, which `run` does not capture — so a loud run prints NO sentinel.)
#[test]
fn non_packable_record_queue_stays_loud() {
    let o = run("module t; typedef struct { int k; string s; } np_t; np_t q[$];\n\
         initial begin np_t p; p.k=1; p.s=\"x\"; q.push_back(p); $display(\"SZ=%0d\", q.size()); $finish; end endmodule\n");
    assert!(
        !o.contains("SZ="),
        "non-packable record queue must be loud:\n{o}"
    );
}

// correct-or-loud: a BOUNDED queue of record (`[$:N]`) stays loud.
#[test]
fn bounded_record_queue_stays_loud() {
    let o = run("module t; typedef struct { logic [7:0] x; } p_t; p_t q[$:3];\n\
         initial begin p_t p; p.x=8'd1; q.push_back(p); $display(\"SZ=%0d\", q.size()); $finish; end endmodule\n");
    assert!(
        !o.contains("SZ="),
        "bounded record queue must be loud:\n{o}"
    );
}

// Regression: a dynamic array of record already worked (§4.5.191 kin) — keep it.
#[test]
fn dynamic_array_of_record_still_works() {
    let o = run(
        "module t; typedef struct { logic [15:0] a; logic [7:0] b; } p_t; p_t d[];\n\
         initial begin d=new[2]; d[0].a=16'h11; d[0].b=8'd5;\n\
           $display(\"a=%h b=%0d sz=%0d\", d[0].a, d[0].b, d.size()); $finish; end endmodule\n",
    );
    assert!(
        o.contains("a=0011 b=5 sz=2"),
        "dynamic array of record:\n{o}"
    );
}
