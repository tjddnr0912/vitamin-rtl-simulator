//! Round-6 external report — UARR: unpacked-array subroutine formals (IEEE §13.3).
//!
//! `function ... (input logic [63:0] words [0:7])` was rejected at PARSE (E2002
//! "expected ')' … found LBracket") → a 6-error cascade that desynced the whole
//! design. Now the parser accepts the trailing unpacked dims and elaborate lowers
//! a SUPPORTED slice as an md-packed frame slot:
//!
//!   - single unpacked dimension, ZERO-BASED (`[0:N-1]` / `[N-1:0]` / `[N]`),
//!   - a simple UNSIGNED zero-LSB vector (or scalar) element,
//!   - a FUNCTION INPUT formal, read via `arr[i]` (element / bit-select), and
//!   - a whole-array-net ACTUAL of matching element width and length.
//!
//! The formal `arr [0:N-1]` of `[W-1:0]` becomes an md-packed `[N-1:0][W-1:0]`
//! frame net (`arr[i]` reuses the md-packed element read/write machinery); the
//! call site packs the actual's elements `{a[N-1], …, a[0]}` into the slot. Every
//! shape OUTSIDE the slice (multi-dim, non-zero-based, signed element, a non-array
//! actual, a shape mismatch, a whole-array use, or a TASK array formal) is
//! loud-rejected — correct-or-loud (iverilog itself rejects unpacked tf-ports with
//! "sorry: … not yet supported").
//!
//! Oracle = iverilog 13.0 via the equivalent PACKED-vector formulation (which
//! iverilog DOES run) — e.g. `pick(tbl, i)` on `logic [7:0] tbl [0:3]` mirrors
//! `pick(32'h40302010, i)` returning `arr[(i*8)+:8]`; both give 10/20/30/40.
//!
//! AST adds `TfPort.unpacked` (schema-hash re-pin); sim-ir / format_version 19
//! UNCHANGED (IR-0 — the formal is an ordinary md-packed frame net).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (first `o=`/`y=` stdout line, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r6_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("o=") || l.starts_with("y="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (first, out.status.success())
}

/// Does `src` elaborate + simulate successfully (exit 0)?
fn ok(src: &str) -> bool {
    run(src).1
}

// ───────────────────────── SUPPORTED slice ─────────────────────────

#[test]
fn uarr_const_index_each_element() {
    // pick(tbl, k) on tbl='{10,20,30,40} → 10/20/30/40 (== the packed-vector twin
    // `pick(32'h40302010,k) = arr[(k*8)+:8]`, which iverilog runs).
    for (k, exp) in [(0, "10"), (1, "20"), (2, "30"), (3, "40")] {
        let src = format!(
            "module m(output logic [7:0] o);\n\
             function automatic logic [7:0] pick(input logic [7:0] a[0:3], input int i); return a[i]; endfunction\n\
             logic [7:0] tbl[0:3];\n\
             initial begin tbl='{{8'h10,8'h20,8'h30,8'h40}}; o=pick(tbl,{k}); $display(\"o=%h\",o); end\n\
             endmodule"
        );
        assert_eq!(run(&src).0, format!("o={exp}"), "pick(tbl,{k})");
    }
}

#[test]
fn uarr_runtime_index_always_comb() {
    // A runtime index (the caller's driven input), the common datapath shape.
    let src = "module m(input logic [1:0] s, output logic [7:0] o);\n\
        function automatic logic [7:0] pick(input logic [7:0] a[0:3], input int i); return a[i]; endfunction\n\
        logic [7:0] tbl[0:3];\n\
        always_comb begin tbl='{8'h10,8'h20,8'h30,8'h40}; o=pick(tbl,s); end\n\
        endmodule\n\
        module tb; logic [1:0] s; logic [7:0] o; m d(.s(s),.o(o));\n\
        initial begin s=2'd0;#1 $display(\"o=%h\",o); s=2'd2;#1 $display(\"o=%h\",o); end endmodule";
    // first line = s=0 → 0x10
    assert_eq!(run(src).0, "o=10");
}

#[test]
fn uarr_loop_sum_over_formal() {
    // A body that LOOPS over the array formal (the realistic SHA-schedule shape) —
    // control flow rides the frame path. Σ(1..8) = 36.
    let src = "module m(output logic [15:0] o);\n\
        function automatic logic [15:0] chk(input logic [7:0] w[0:7]);\n\
        logic [15:0] s; s=0; for(int i=0;i<8;i++) s=s+w[i]; return s; endfunction\n\
        logic [7:0] mem[0:7];\n\
        initial begin for(int i=0;i<8;i++) mem[i]=i+1; o=chk(mem); $display(\"o=%0d\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=36");
}

#[test]
fn uarr_64bit_elements_index15() {
    // 64-bit elements, `[0:15]` — exactly the report's `schedule_mem [0:15]` shape.
    let src = "module m(input logic [3:0] sel, output logic [63:0] o);\n\
        function automatic logic [63:0] pick(input logic [63:0] a[0:15], input int i); return a[i]; endfunction\n\
        logic [63:0] mem[0:15];\n\
        always_comb begin for(int i=0;i<16;i++) mem[i]=64'hDEAD_0000_0000_0000+i; o=pick(mem,sel); end\n\
        endmodule\n\
        module tb; logic [3:0] sel; logic [63:0] o; m d(.sel(sel),.o(o));\n\
        initial begin sel=4'd15;#1 $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=dead00000000000f");
}

#[test]
fn uarr_descending_declared_range() {
    // `[3:0]` (descending declaration) — element index i still addresses element i.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[3:0], input int i); return a[i]; endfunction\n\
        logic [7:0] g[3:0];\n\
        initial begin g[0]=8'hA0;g[1]=8'hA1;g[2]=8'hA2;g[3]=8'hA3; o=f(g,2); $display(\"o=%h\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=a2");
}

#[test]
fn uarr_size_shorthand() {
    // `[N]` == `[0:N-1]`.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[4], input int i); return a[i]; endfunction\n\
        logic [7:0] g[4];\n\
        initial begin g[0]=8'hB0;g[1]=8'hB1;g[2]=8'hB2;g[3]=8'hB3; o=f(g,3); $display(\"o=%h\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=b3");
}

#[test]
fn uarr_write_formal_is_local_copy() {
    // Writing the input formal mutates the callee's COPY (pass-by-value): the
    // return sees the write (0x01+0xFF = 0x00) but the caller's array is unchanged.
    let src = "module m(output logic [7:0] o1, output logic [7:0] o2);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:1]); a[1]=8'hFF; return a[0]+a[1]; endfunction\n\
        logic [7:0] g[0:1];\n\
        initial begin g[0]=8'h01;g[1]=8'h02; o1=f(g); o2=g[1]; $display(\"o=%h_%h\",o1,o2); end\n\
        endmodule";
    // o1 = 0x00 (0x01+0xFF), o2 = 0x02 (caller's g[1] untouched).
    assert_eq!(run(src).0, "o=00_02");
}

#[test]
fn uarr_element_bit_select() {
    // `arr[i][j]` — element i, bit j.
    let src = "module m(output logic o);\n\
        function automatic logic f(input logic [7:0] a[0:3], input int i, input int j); return a[i][j]; endfunction\n\
        logic [7:0] g[0:3];\n\
        initial begin g[2]=8'b0000_1000; o=f(g,2,3); $display(\"o=%b\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=1");
}

#[test]
fn uarr_two_array_formals_and_scalar() {
    // Two array formals + a scalar formal, mixed.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:1], input logic [7:0] b[0:1], input int k);\n\
        return a[k]+b[k]; endfunction\n\
        logic [7:0] g[0:1]; logic [7:0] h[0:1];\n\
        initial begin g[0]=8'h10;g[1]=8'h20;h[0]=8'h01;h[1]=8'h02; o=f(g,h,1); $display(\"o=%h\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=22"); // 0x20 + 0x02
}

#[test]
fn uarr_int_unsigned_element() {
    // An integral ATOM element (`int unsigned`, implicit 32-bit zero-LSB) — the
    // width comes from the atom, not a `[msb:lsb]` range.
    let src = "module m(output int o);\n\
        function automatic int f(input int unsigned a[0:3], input int i); return a[i]; endfunction\n\
        int unsigned g[0:3];\n\
        initial begin g[0]=100;g[1]=200;g[2]=300;g[3]=400; o=f(g,3); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=400");
}

#[test]
fn uarr_byte_unsigned_element() {
    let src = "module m(output int o);\n\
        function automatic int f(input byte unsigned a[0:3], input int i); return a[i]; endfunction\n\
        byte unsigned g[0:3];\n\
        initial begin g[0]=8'hF0;g[1]=8'hF1;g[2]=8'hF2;g[3]=8'hF3; o=f(g,2); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=242"); // 0xF2
}

#[test]
fn uarr_loud_real_element() {
    // A real element is not a bit-vector → cannot be md-packed → loud.
    let src = "module m(output real o);\n\
        function automatic real f(input real a[0:3], input int i); return a[i]; endfunction\n\
        real g[0:3];\n\
        initial begin g[0]=1.5; o=f(g,0); $display(\"o=%f\",o); end endmodule";
    assert!(!ok(src), "a real-element formal must be loud");
}

#[test]
fn uarr_bare_int_signed_element_reads_signed() {
    // G3 (round-10): a bare `int` element is SIGNED and is now SUPPORTED — the
    // whole-element read re-stamps `$signed`, so a negative element reads negative
    // (not zero-extended via the unsigned part-select). Was loud before.
    let src = "module m(output int o);\n\
        function automatic int f(input int a[0:3], input int i); return a[i]; endfunction\n\
        int g[0:3];\n\
        initial begin g[0]=-5; o=f(g,0); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=-5");
}

#[test]
fn uarr_two_state_element_coerces_x() {
    // R2: a 2-state element (`int unsigned`/`bit`) can never hold X/Z (§6.11.3) — an
    // X-containing 4-state actual coerces to 0 at the formal (the md-packed slot is
    // registered 2-state). Was `xxxxxxxx` (silent-wrong) before the coercion fix.
    let src = "module m(output logic [31:0] o);\n\
        function automatic logic [31:0] f(input int unsigned a[0:1], input int i); return a[i]; endfunction\n\
        logic [31:0] b[0:1];\n\
        initial begin b[0]=32'hxxxx_xxxx; b[1]=32'd5; o=f(b,0); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=00000000");
}

#[test]
fn uarr_four_state_element_keeps_x() {
    // The 4-state twin: a `logic` element retains X/Z (NOT registered 2-state).
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:1], input int i); return a[i]; endfunction\n\
        logic [7:0] b[0:1];\n\
        initial begin b[0]=8'hxx; b[1]=8'h5; o=f(b,0); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=xx");
}

#[test]
fn uarr_out_of_range_index_reads_x() {
    // An out-of-range element index reads X (matches the packed-vector twin's
    // out-of-range part-select), NOT a neighbour / silent 0.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3], input int i); return a[i]; endfunction\n\
        logic [7:0] g[0:3];\n\
        initial begin g[0]=8'h11; o=f(g,5); $display(\"o=%h\",o); end\n\
        endmodule";
    assert_eq!(run(src).0, "o=xx");
}

// ───────────────────────── correct-or-LOUD boundaries ─────────────────────────

#[test]
fn uarr_loud_multidim_formal() {
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:1][0:1]); return a[0][0]; endfunction\n\
        logic [7:0] g[0:1][0:1];\n\
        initial begin g[0][0]=8'h55; o=f(g); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "multi-dim unpacked formal must be loud");
}

#[test]
fn uarr_loud_non_zero_based() {
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[1:4], input int i); return a[i]; endfunction\n\
        logic [7:0] g[1:4];\n\
        initial begin g[1]=8'h55; o=f(g,1); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "non-zero-based range must be loud");
}

#[test]
fn uarr_signed_element_reads_signed() {
    // G3 (round-10): a `logic signed [7:0]` element is now SUPPORTED — the whole-element
    // read re-stamps `$signed` so it reads negative (not unsigned via the part-select).
    let src = "module m(output logic signed [7:0] o);\n\
        function automatic logic signed [7:0] f(input logic signed [7:0] a[0:3], input int i); return a[i]; endfunction\n\
        logic signed [7:0] g[0:3];\n\
        initial begin g[0]=-8'sd5; o=f(g,0); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=-5");
}

#[test]
fn uarr_loud_non_array_actual() {
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3]); return a[0]; endfunction\n\
        initial begin o=f(8'h55); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "a non-array actual must be loud");
}

#[test]
fn uarr_loud_shape_mismatch() {
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3]); return a[0]; endfunction\n\
        logic [7:0] g[0:7];\n\
        initial begin g[0]=8'h55; o=f(g); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "an actual/formal length mismatch must be loud");
}

#[test]
fn uarr_loud_whole_array_use() {
    // Reading the whole formal (`x=a`) — not an element — is loud.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] a[0:3]); logic [31:0] x; x=a; return x[7:0]; endfunction\n\
        logic [7:0] g[0:3];\n\
        initial begin g[0]=8'h55; o=f(g); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "a whole-array formal use must be loud");
}

#[test]
fn uarr_loud_whole_formal_part_select() {
    // R1: `arr[hi:lo]` on the WHOLE formal is an unpacked slice, not a value — the
    // md-packed rep would silently return `{arr[hi],…,arr[lo]}` (found: `bbaa`).
    let src = "module m(output logic [15:0] o);\n\
        function automatic logic [15:0] f(input logic [7:0] arr[0:3]); return arr[1:0]; endfunction\n\
        logic [7:0] tbl[0:3];\n\
        initial begin tbl[0]=8'hAA;tbl[1]=8'hBB; o=f(tbl); $display(\"o=%h\",o); end endmodule";
    assert!(
        !ok(src),
        "a part-select of the whole array formal must be loud"
    );
}

#[test]
fn uarr_loud_whole_formal_indexed_part() {
    let src = "module m(output logic [15:0] o);\n\
        function automatic logic [15:0] f(input logic [7:0] arr[0:3]); return arr[0 +: 2]; endfunction\n\
        logic [7:0] tbl[0:3];\n\
        initial begin tbl[0]=8'hAA; o=f(tbl); $display(\"o=%h\",o); end endmodule";
    assert!(
        !ok(src),
        "an indexed part-select of the whole array formal must be loud"
    );
}

#[test]
fn uarr_loud_direction_mismatch() {
    // R1: a descending formal `[3:0]` fed an ascending actual `[0:3]` — §7.6 copies
    // by POSITION, so the index-based md-packed read would silently reverse elements
    // (found `arr[0]=11`, IEEE `arr[0]=44`).
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] arr[3:0], input int i); return arr[i]; endfunction\n\
        logic [7:0] a[0:3];\n\
        initial begin a='{8'h11,8'h22,8'h33,8'h44}; o=f(a,0); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "opposite formal/actual directions must be loud");
}

#[test]
fn uarr_loud_2d_actual_for_1d_formal() {
    // R1 secondary: a 2-D actual with the same total element count as a 1-D formal.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] arr[0:3]); return arr[0]; endfunction\n\
        logic [7:0] mm[0:1][0:1];\n\
        initial begin mm[0][0]=8'hA1; o=f(mm); $display(\"o=%h\",o); end endmodule";
    assert!(!ok(src), "a 2-D actual for a 1-D formal must be loud");
}

#[test]
fn uarr_matching_descending_direction_ok() {
    // The correct twin of the direction-mismatch loud: both `[3:0]` — supported.
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] f(input logic [7:0] arr[3:0], input int i); return arr[i]; endfunction\n\
        logic [7:0] a[3:0];\n\
        initial begin a[0]=8'h11;a[1]=8'h22;a[2]=8'h33;a[3]=8'h44; o=f(a,2); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=33");
}

#[test]
fn uarr_element_part_select_still_supported() {
    // The element part-select `arr[i][hi:lo]` (BitSelect base) stays supported —
    // NOT caught by the whole-formal part-select guard.
    let src = "module m(output logic [3:0] o);\n\
        function automatic logic [3:0] f(input logic [7:0] a[0:3], input int i); return a[i][7:4]; endfunction\n\
        logic [7:0] g[0:3];\n\
        initial begin g[1]=8'hAB; o=f(g,1); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=a");
}

#[test]
fn uarr_task_input_array_formal_now_supported() {
    // §4.5.188: an `input` unpacked-fixed array formal on a TASK is now an
    // md-packed frame slot (mirrors the function path); the `output` scalar `r`
    // copies out normally. Formerly loud (round-6 pinned the reject). 1+2+3+4=10.
    let src = "module m(output logic [7:0] o);\n\
        task automatic sum(input logic [7:0] a[0:3], output logic [7:0] r); r=a[0]+a[1]+a[2]+a[3]; endtask\n\
        logic [7:0] mem[0:3];\n\
        initial begin mem='{8'd1,8'd2,8'd3,8'd4}; sum(mem,o); $display(\"o=%0d\",o); end endmodule";
    assert_eq!(run(src).0, "o=10");
}

// ───────────────────────── regression guards ─────────────────────────

#[test]
fn uarr_ok_packed_vector_workaround_still_runs() {
    // The report's packed-vector workaround (UARR_OK) — unaffected.
    let src = "module m(input logic [1:0] s, output logic [7:0] o);\n\
        function automatic logic [7:0] pick(input logic [31:0] a, input int i); return a[(i*8)+:8]; endfunction\n\
        always_comb o=pick(32'h40_30_20_10, s);\n\
        endmodule\n\
        module tb; logic [1:0] s; logic [7:0] o; m d(.s(s),.o(o)); initial begin s=0;#1 $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=10");
}

#[test]
fn scalar_formal_function_unaffected() {
    // A function with NO array formal is byte-identically unaffected (still inline).
    let src = "module m(output logic [7:0] o);\n\
        function automatic logic [7:0] dbl(input logic [7:0] x); return x + x; endfunction\n\
        initial begin o=dbl(8'h21); $display(\"o=%h\",o); end endmodule";
    assert_eq!(run(src).0, "o=42");
}

#[test]
fn pkg_scoped_call_now_supported() {
    // The round-6 awareness item PKGCALL — `pkg::f(...)` — is SUPPORTED as of round-7
    // (§4.5.111) for a self-contained, straight-line package function (`f` reads only
    // its formal `x`). `cp::f(2'b10)` → 2'b10.
    let src = "package cp; function automatic logic [1:0] f(input logic [1:0] x); return x; endfunction endpackage\n\
        module m(output logic [1:0] o); assign o = cp::f(2'b10); initial begin #1; $display(\"o=%0d\",o); end endmodule";
    assert!(ok(src), "self-contained package-scoped call now elaborates");
    assert_eq!(run(src).0, "o=2");
}
