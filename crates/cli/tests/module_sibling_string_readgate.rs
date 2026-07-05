//! The module/process `hoist_block_local_nets` coalesces a block-local onto a
//! same-named earlier block-local's net (v1 has no per-block scope). A later `string`
//! block-local coalescing onto an earlier PACKED net was silently wrong when the
//! string is READ (`begin logic s; …=s[0]; end  begin string s; s="AB"; …=s[0]; end`)
//! — the string read the packed net's bits. §4.5.95 found that a blanket reject
//! over-rejects a WRITE-ONLY string (which coalesces harmlessly — its truncated
//! write is discarded), so this is a READ-GATED loud reject: fire only when the
//! block reads the string. Pinned to iverilog 13.0 for the non-rejected cases.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mssr_{}_{n}", std::process::id()));
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
    )
}

#[test]
fn module_sibling_string_read_loud() {
    // b1 packed, b2 string that READS s (`r2 = s[0]`) → was silently 1000, now loud.
    let src = "module m;\n\
         integer r1; integer r2;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end\n\
           begin : b2 string s; s = \"AB\"; r2 = s[0]; end\n\
           $display(\"r=%0d\", r1*1000+r2); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "string-read coalesce must loud, got code={code:?}\n{out}"
    );
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn module_sibling_string_write_only_ok() {
    // b2 packed (read), b1 string WRITE-ONLY (never read) — coalesces harmlessly, the
    // truncated write discarded → must NOT be rejected (§4.5.95 false-positive class).
    let src = "module m;\n\
         integer r;\n\
         initial begin\n\
           begin : b2 logic [7:0] s; s = 8'h41; r = s[0]; end\n\
           begin : b1 string s; s = \"AB\"; end\n\
           $display(\"r=%0d\", r); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "write-only string coalesce OK, got:\n{out}"
    );
}

#[test]
fn module_sibling_string_display_read_loud() {
    // A `$display("%s", s)` READS the string via a sysfunc arg → loud.
    let src = "module m;\n\
         initial begin\n\
           begin : b2 logic [7:0] s; s = 8'h41; end\n\
           begin : b1 string s; s = \"AB\"; $display(\"s=%s\", s); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "string display-read must loud, got code={code:?}\n{out}"
    );
}

#[test]
fn module_sibling_string_display_literal_ok() {
    // `$display("hi")` does NOT read s (literal arg) — the write-only string is not
    // rejected (precise: a sysfunc call in the block does not blanket-reject).
    let src = "module m;\n\
         integer r;\n\
         initial begin\n\
           begin : b2 logic [7:0] s; s = 8'h41; r = s[0]; end\n\
           begin : b1 string s; s = \"AB\"; $display(\"hi\"); end\n\
           $display(\"r=%0d\", r); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "display-literal write-only OK, got:\n{out}"
    );
}

#[test]
fn module_sibling_string_lvalue_index_read_loud() {
    // Round-2 find: an lvalue INDEX read (`mem[s[0]] = …`) reads the string as an
    // rvalue (the index) — it must be caught (loud), symmetric with the RHS read
    // `x = mem[s[0]]`. Was silently mem[66]=xx.
    let src = "module m;\n\
         logic [7:0] mem [0:255];\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; mem[66] = s; end\n\
           begin : b2 string s; s = \"AB\"; mem[s[0]] = 8'hFF; end\n\
           $display(\"mem66=%0h\", mem[66]); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "lvalue-index string read must loud, got code={code:?}\n{out}"
    );
}

#[test]
fn module_sibling_string_sformat_write_only_ok() {
    // Round-2 find: `$sformat(s, …)` WRITES its dest arg (arg 0) — a string populated
    // by $sformat and never read is write-only → must NOT be rejected.
    let src = "module m;\n\
         logic [7:0] out;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; out = s; end\n\
           begin : b2 string s; $sformat(s, \"hi %0d\", 7); end\n\
           $display(\"out=%0d\", out); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("out=65"),
        "$sformat write-only OK, got:\n{out}"
    );
}

#[test]
fn module_sibling_same_type_reuse_ok() {
    // GUARD: same-name SAME-type (packed) sibling reuse coalesces safely — unaffected.
    let src = "module m;\n\
         integer r1; integer r2;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end\n\
           begin : b2 logic [7:0] s; s = 8'h80; r2 = s[7]; end\n\
           $display(\"r=%0d\", r1*10+r2); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=11"), "same-type reuse, got:\n{out}");
}

#[test]
fn module_sibling_string_declinit_read_loud() {
    // Round-3 find: a SIBLING decl's own initializer (`int x = s[0]+1;`) reads the
    // string but is a decl, not a stmt — the gate now scans block decl inits. This is
    // a genuine coalesce silent-wrong (baseline x=0 vs iverilog x=1: the decl-init
    // runs at the t0 pre-sweep, so vita reads the uninit packed net while iverilog
    // reads the empty string's byte 0), now loudly rejected.
    let src = "module m;\n\
         int y;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; int x = s[0] + 1; y = x; $display(\"x=%0d\", x); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "sibling decl-init string read must loud, got code={code:?}\n{out}"
    );
}

#[test]
fn module_sibling_string_declinit_nested_read_loud() {
    // Round-3 find: the read lives in a NESTED block's decl-init — the Block/Fork
    // arm of `stmt_reads_ident` now recurses into nested decls.
    let src = "module m;\n\
         int y;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; begin int x = s[0]; y = x; end $display(\"y=%0d\", y); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "nested decl-init string read must loud");
}

#[test]
fn module_sibling_string_assign_read_loud() {
    // Round-3 find: a procedural-continuous `assign y = s[0];` reads the string —
    // the new `Assign` arm catches it. Was silently y=1.
    let src = "module m;\n\
         integer y;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; s = \"h\"; assign y = s[0]; end\n\
           #1 $display(\"y=%0d\", y); $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "assign string read must loud");
}

#[test]
fn module_sibling_string_force_read_loud() {
    // Round-3 find: `force y = s[0];` reads the string — the new `Force` arm catches
    // it. Was silently y=1.
    let src = "module m;\n\
         integer y;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; s = \"h\"; force y = s[0]; end\n\
           #1 $display(\"y=%0d\", y); $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "force string read must loud");
}

#[test]
fn module_sibling_string_sscanf_rvalue_write_only_ok() {
    // Round-3 find: `n = $sscanf(src, fmt, s)` WRITES its trailing dest `s` — as an
    // rvalue (blocking RHS) the shared `expr_reads_ident` counted all args as reads,
    // over-rejecting this write-only string. The gated `rvalue_reads_ident` now skips
    // the dest. iverilog runs it (exit 0).
    let src = "module m;\n\
         int n;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; n = $sscanf(\"hello\", \"%s\", s); end\n\
           $display(\"done\"); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("done"), "sscanf write-only OK, got:\n{out}");
}

#[test]
fn module_sibling_string_fread_write_only_ok() {
    // Round-3 find: `$fread(s, fd)` writes its dest arg 0 — it was missing from the
    // dest-skip whitelist entirely, over-rejecting a write-only string.
    let src = "module m;\n\
         int code; integer fd;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; code = $fread(s, fd); end\n\
           $display(\"done\"); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("done"), "fread write-only OK, got:\n{out}");
}

#[test]
fn module_sibling_string_assignpattern_read_loud() {
    // Round-3 find: `'{s[0], s[1]}` assignment-pattern elements read the string — the
    // `AssignPattern` leaf kind was unhandled in `expr_reads_ident` (a blind spot
    // shared with the scope-leak walker). Was silently r=1 vs iverilog r=65066.
    let src = "module m;\n\
         int r;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; s = \"AB\"; begin int a[2]; a = '{s[0], s[1]}; r = a[0]*1000+a[1]; end $display(\"r=%0d\", r); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "assignment-pattern string read must loud");
}

#[test]
fn module_sibling_string_new_size_read_loud() {
    // Round-3 find: `new[s[0]]` dynamic-array size expr reads the string — the `New`
    // leaf kind was unhandled in `expr_reads_ident`. Was silently r=0 vs iverilog 65.
    let src = "module m;\n\
         int r;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; s = \"AB\"; begin int d[]; d = new[s[0]]; r = d.size(); end $display(\"r=%0d\", r); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "new[] size string read must loud");
}

#[test]
fn module_sibling_string_assignpattern_write_only_ok() {
    // GUARD: a write-only string whose block has an `'{…}` that does NOT read it is
    // not over-rejected (the new leaf arm matches only genuine reads of the name).
    let src = "module m;\n\
         logic [7:0] o;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; o = s; end\n\
           begin : b2 string s; begin int a[2]; a = '{1, 2}; end $sformat(s, \"x\"); end\n\
           $display(\"o=%0d\", o); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("o=65"),
        "assign-pattern write-only OK, got:\n{out}"
    );
}

#[test]
fn module_sibling_string_plusargs_write_only_ok() {
    // Round-3 find: `$value$plusargs("foo=%s", s)` WRITES its dest arg 1 (the format
    // arg 0 is the only read). It was missing from the direct-rhs writer whitelist, so
    // a write-only string dest was over-rejected. iverilog runs it (exit 0, o=65).
    let src = "module m;\n\
         logic [7:0] o; int ok;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; o = s; end\n\
           begin : b2 string s; ok = $value$plusargs(\"foo=%s\", s); end\n\
           $display(\"o=%0d\", o); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("o=65"), "plusargs write-only OK, got:\n{out}");
}

#[test]
fn module_sibling_string_plusargs_fmt_read_loud() {
    // GUARD: the format arg (arg 0) of `$value$plusargs` IS a read — a string used as
    // the format shadowing a packed sibling must still loud-reject.
    let src = "module m;\n\
         int ok;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; end\n\
           begin : b2 string s; string d; s = \"x\"; ok = $value$plusargs(s, d); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "plusargs format-arg string read must loud");
}

#[test]
fn module_sibling_string_cast_write_only_ok() {
    // Round-3 find: `$cast(dst, src)` WRITES dst=arg0 (reads src=arg1). It was missing
    // from the arg0-dest writer set, so a write-only string dst was over-rejected.
    let src = "module m;\n\
         logic [7:0] o; int ok;\n\
         initial begin\n\
           begin : b1 logic [7:0] s; s = 8'h41; o = s; end\n\
           begin : b2 string s; ok = $cast(s, \"hi\"); end\n\
           $display(\"o=%0d\", o); #1 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("o=65"), "cast write-only OK, got:\n{out}");
}

#[test]
fn module_sibling_string_eventctrl_read_loud() {
    // Round-4 find: an `@(s) stmt` event control reads s — `stmt_reads_ident`'s
    // EventCtrl arm dropped the `ctrl` prefix via `..`. vita even has a dedicated
    // `@(string)` reject that the coalesce-to-packed-net silently bypassed.
    let src = "module m;\n\
         initial begin\n\
           begin : b1 logic [15:0] s; end\n\
           begin : b2 string s; s = \"AB\"; @(s) $display(\"fired\"); end\n\
           #5 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "event-control string read must loud");
}

#[test]
fn module_sibling_string_intra_event_read_loud() {
    // Round-4 find: an intra-assignment event control `r = @(s) rhs` reads s — the
    // Blocking arm dropped the `event` field via `..`.
    let src = "module m;\n\
         logic [7:0] r;\n\
         initial begin\n\
           begin : b1 logic [15:0] s; end\n\
           begin : b2 string s; s = \"AB\"; r = @(s) 8'hEE; end\n\
           #5 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "intra-assignment event string read must loud"
    );
}

#[test]
fn module_sibling_string_delayctrl_read_loud() {
    // Round-4 find: a `#(s[0]) stmt` delay control reads s — the DelayCtrl arm dropped
    // the `delay` prefix via `..`. Was silently `fired at 0` vs iverilog `at 65`.
    let src = "module m;\n\
         initial begin\n\
           begin : b1 logic [15:0] s; end\n\
           begin : b2 string s; s = \"AB\"; #(s[0]) $display(\"fired\"); end\n\
           #99 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "delay-control string read must loud");
}

#[test]
fn module_sibling_string_classnew_read_loud() {
    // Round-4 find: a class ctor arg `new(s[0])` reads s — `ClassNew` was a `_ => false`
    // leaf; now gated in `rvalue_reads_ident` (read-gate-only). Was silently v=0.
    let src = "class C; int v; function new(int x); v = x; endfunction endclass\n\
         module m;\n\
         C h;\n\
         initial begin\n\
           begin : b1 logic [15:0] s; end\n\
           begin : b2 string s; s = \"AB\"; h = new(s[0]); $display(\"v=%0d\", h.v); end\n\
           #1 $finish;\n\
         end endmodule\n";
    let (_out, code) = run(src);
    assert_eq!(code, Some(1), "class-ctor-arg string read must loud");
}

#[test]
fn module_sibling_string_delay_write_only_ok() {
    // GUARD: a `#(const)` delay that does NOT read s (s write-only) is not over-
    // rejected — the delay-prefix walk matches only genuine reads of the name.
    let src = "module m;\n\
         logic [7:0] o;\n\
         initial begin\n\
           begin : b1 logic [15:0] s; s = 16'h0041; o = s[7:0]; end\n\
           begin : b2 string s; #(3) s = \"AB\"; end\n\
           $display(\"o=%0d\", o); #5 $finish;\n\
         end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("o=65"), "write-only #delay OK, got:\n{out}");
}
