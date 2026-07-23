//! §4.5.191 UARR: a FIXED 1-D unpacked array of a PACKABLE unpacked struct
//! (`entry_t mem[N]`) loud→supported — the reviewer's round-16 V6 (a memory model
//! `mem[addr].field`). It lowers to a fixed array of the packed-struct-width vector
//! `logic/bit [W-1:0] mem[N]` and registers as a struct 1-D array, so `mem[i].field`
//! reuses the §4.5.190 packed-struct-array desugar (`struct_array_field_geom` falls
//! back to `packable_record_layout`). A non-packable record (string/real/nested
//! member), a multi-dim array, or a decl-init stays loud (correct-or-loud).
//!
//! iverilog 13.0 rejects unpacked structs outright, so verified by vita self-
//! consistency: a field write then read round-trips, the whole-element packing is
//! declaration-order MSB-first, and undriven fields take the element default.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fra_{}_{n}", std::process::id()));
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
fn memory_model_field_access() {
    // The reviewer's axi_mem_model shape: a fixed array of {addr,data,valid}.
    let o = run("module top;\n\
         typedef struct { logic [7:0] addr; logic [31:0] data; logic valid; } entry_t;\n\
         entry_t mem[4];\n\
         initial begin\n\
           mem[0].addr=8'h10; mem[0].data=32'hDEADBEEF; mem[0].valid=1;\n\
           mem[2].addr=8'h20; mem[2].data=32'hCAFEBABE;\n\
           $display(\"a=%h d=%h v=%b\", mem[0].addr, mem[0].data, mem[0].valid);\n\
           $display(\"a=%h d=%h\", mem[2].addr, mem[2].data);\n\
         end\nendmodule\n");
    assert!(o.contains("a=10 d=deadbeef v=1"), "got:\n{o}");
    assert!(o.contains("a=20 d=cafebabe"), "got:\n{o}");
}

#[test]
fn runtime_index_read_write() {
    let o = run("module top;\n\
         typedef struct { logic [7:0] tag; logic [15:0] val; } e_t;\n\
         e_t mem[8]; int i;\n\
         initial begin\n\
           for (i=0;i<3;i++) begin mem[i].tag=i; mem[i].val=i*100; end\n\
           for (i=0;i<3;i++) $display(\"t=%0d v=%0d\", mem[i].tag, mem[i].val);\n\
         end\nendmodule\n");
    assert!(o.contains("t=0 v=0"), "got:\n{o}");
    assert!(o.contains("t=1 v=100"), "got:\n{o}");
    assert!(o.contains("t=2 v=200"), "got:\n{o}");
}

#[test]
fn whole_element_and_field_offsets() {
    // Declaration-order MSB-first: a is the high byte, b the low byte.
    let o = run("module top;\n\
         typedef struct { logic [7:0] a; logic [7:0] b; } p_t;\n\
         p_t mem[2];\n\
         initial begin mem[0].a=8'h11; mem[0].b=8'h22; $display(\"whole=%h\", mem[0]); end\n\
         endmodule\n");
    assert!(o.contains("whole=1122"), "got:\n{o}");
}

#[test]
fn assign_pattern_on_element() {
    // `mem[i] = '{…}` positional pattern (asymmetric fields exercise the field order).
    let o = run("module top;\n\
         typedef struct { logic [3:0] x; logic [11:0] y; } asym_t;\n\
         asym_t mem[2];\n\
         initial begin mem[0] = '{4'hA, 12'hBCD};\n\
           $display(\"x=%h y=%h whole=%h\", mem[0].x, mem[0].y, mem[0]);\n\
         end\nendmodule\n");
    assert!(o.contains("x=a y=bcd whole=abcd"), "got:\n{o}");
}

#[test]
fn two_state_undriven_field_is_zero() {
    let o = run("module top;\n\
         typedef struct { byte a; byte b; } p_t;\n\
         p_t mem[2];\n\
         initial begin mem[0].a=8'h11; $display(\"a=%h b=%h\", mem[0].a, mem[0].b); end\n\
         endmodule\n");
    assert!(o.contains("a=11 b=00"), "got:\n{o}");
}

#[test]
fn four_state_undriven_field_is_x() {
    let o = run("module top;\n\
         typedef struct { logic [7:0] a; logic [7:0] b; } p_t;\n\
         p_t mem[2];\n\
         initial begin mem[0].a=8'h11; $display(\"a=%h b=%h\", mem[0].a, mem[0].b); end\n\
         endmodule\n");
    assert!(o.contains("a=11 b=xx"), "got:\n{o}");
}

#[test]
fn nonpackable_record_array_now_soa() {
    // r18 (Fix A): a non-packable (string/mixed/param-width member) record array — fixed
    // OR queue — now lowers to struct-of-arrays (per-member native `$unp$arr$field`), the
    // same fallback the dynamic-array path already had. `arr[0].id` → `$unp$arr$id[0]`.
    let o = run("module top;\n\
         typedef struct { string name; int id; } rec_t;\n\
         rec_t arr[2];\n\
         initial begin arr[0].id = 5; $display(\"%0d\", arr[0].id); end\n\
         endmodule\n");
    assert!(
        !o.contains("E2002") && !o.contains("E3010") && o.contains('5'),
        "fixed string-member record array now works via SoA:\n{o}"
    );
}

#[test]
fn multi_dim_record_array_stays_loud() {
    let o = run("module top;\n\
         typedef struct { logic [7:0] a; logic [7:0] b; } p_t;\n\
         p_t mem[2][2];\n\
         initial begin mem[0][0].a=8'h11; $display(\"%h\", mem[0][0].a); end\n\
         endmodule\n");
    assert!(
        o.contains("E2002") || o.contains("E3010"),
        "should be loud:\n{o}"
    );
}
