//! r18 (families A + G, and the E2 array-element case):
//!
//! A — a queue / fixed array of an unpacked record whose members are NOT all
//! literal-width (a PARAMETER-width member `[ADDR_W-1:0]`, or a MIXED 2-/4-state
//! record `{int; logic}`, or a string/real member) now lowers to a struct-of-arrays
//! (SoA): one native per-member array/queue `$unp$var$field`. Was E2002 "record array
//! unsupported" — the packable (whole-packed-vector) path can't represent a per-instance
//! parameter width, but each `$unp$var$field` resolves its width at elaborate. Queue
//! methods (`push_back`/`push_front`/`pop_*`/`insert`/`delete`) fan out per field
//! (all-or-loud — never a partial push that desyncs the fields).
//!
//! G — `foreach` over a SoA record dyn-array/queue: the `.first`/`.next` iterator rides
//! field 0's net (all fields share length).
//!
//! E2-array — a method on a SoA record STRING member element (`kats[i].name.substr(..)`)
//! now dispatches (a string dyn/queue element is recognized as a string operand).
//!
//! ORACLE: iverilog 13.0 rejects unpacked structs entirely — hand-IEEE, cross-checked
//! against the literal-width record path (which vita already supports) and plain string
//! dyn arrays.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rasoa_{}_{n}", std::process::id()));
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

fn is_loud(o: &str) -> bool {
    o.contains("E3009") || o.contains("E2002") || o.contains("E3010")
}

// ── A: queue-of-record with a PARAMETER-width member ──
#[test]
fn queue_of_record_param_width_member() {
    let o = run(
        "module t #(parameter int ADDR_W = 32, parameter int ID_W = 4);\n\
        typedef struct { logic [ADDR_W-1:0] addr; logic [7:0] len; logic [ID_W-1:0] id; } pkt_t;\n\
        pkt_t q[$];\n\
        initial begin\n\
          pkt_t p; p.addr = 32'h1000; p.len = 8'd4; p.id = 0;\n\
          q.push_back(p);\n\
          if (q[0].len == 8'd4 && q[0].addr == 32'h1000) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n",
    );
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "queue-of-record param width:\n{o}"
    );
}

// ── A: queue-of-record with a MIXED 2-/4-state member (`int` + `logic`) ──
#[test]
fn queue_of_record_mixed_state_member() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] len; } pkt_t;\n\
        pkt_t q[$];\n\
        initial begin\n\
          pkt_t p; p.a = 7; p.len = 8'd4;\n\
          q.push_back(p);\n\
          if (q[0].len == 8'd4 && q[0].a == 7) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "queue-of-record mixed state:\n{o}"
    );
}

// ── A: full queue lifecycle — push_back ×2, size, element read, pop_front into a record ──
#[test]
fn queue_of_record_push_size_pop() {
    let o = run("module t #(parameter int ADDR_W = 16);\n\
        typedef struct { logic [ADDR_W-1:0] addr; logic [7:0] len; int tag; } pkt_t;\n\
        pkt_t q[$]; pkt_t a, b, r;\n\
        initial begin\n\
          a.addr=16'h1000; a.len=8'd4; a.tag=100;\n\
          b.addr=16'h2000; b.len=8'd8; b.tag=200;\n\
          q.push_back(a); q.push_back(b);\n\
          $display(\"size=%0d q1tag=%0d\", q.size(), q[1].tag);\n\
          r = q.pop_front();\n\
          $display(\"pop addr=%h len=%0d tag=%0d size=%0d\", r.addr, r.len, r.tag, q.size());\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o)
            && o.contains("size=2 q1tag=200")
            && o.contains("pop addr=1000 len=4 tag=100 size=1"),
        "queue lifecycle:\n{o}"
    );
}

// ── A: insert / delete on a record queue ──
#[test]
fn queue_of_record_insert_delete() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec_t;\n\
        rec_t q[$]; rec_t x;\n\
        initial begin\n\
          x.a=1; x.b=8'd10; q.push_back(x);\n\
          x.a=3; x.b=8'd30; q.push_back(x);\n\
          x.a=2; x.b=8'd20; q.insert(1, x);\n\
          $display(\"mid=%0d,%0d\", q[1].a, q[1].b);\n\
          q.delete(0);\n\
          $display(\"after=%0d size=%0d\", q[0].a, q.size());\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("mid=2,20") && o.contains("after=2 size=2"),
        "insert/delete:\n{o}"
    );
}

// ── A: push_back with a `'{…}` pattern actual ──
#[test]
fn queue_of_record_push_pattern() {
    let o = run("module t #(parameter int W = 12);\n\
        typedef struct { logic [W-1:0] v; int k; } rec_t;\n\
        rec_t q[$];\n\
        initial begin\n\
          q.push_back('{12'hABC, 5});\n\
          if (q[0].v == 12'hABC && q[0].k == 5) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS"), "push pattern:\n{o}");
}

// ── A: FIXED array of a param-width record ──
#[test]
fn fixed_array_of_param_width_record() {
    let o = run("module t #(parameter int W = 10);\n\
        typedef struct { logic [W-1:0] v; int k; } rec_t;\n\
        rec_t mem[3];\n\
        initial begin\n\
          mem[0].v = 10'd100; mem[0].k = 1;\n\
          mem[2].v = 10'd300; mem[2].k = 3;\n\
          $display(\"m0=%0d,%0d m2=%0d,%0d\", mem[0].v, mem[0].k, mem[2].v, mem[2].k);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("m0=100,1 m2=300,3"),
        "fixed array param-width:\n{o}"
    );
}

// ── G: foreach over a dynamic array of record ──
#[test]
fn foreach_over_dyn_record_array() {
    let o = run("module t;\n\
        typedef struct { int v; string s; } rec_t;\n\
        rec_t arr [];\n\
        initial begin\n\
          arr = new[3]; arr[0].v = 1; arr[1].v = 2; arr[2].v = 3;\n\
          foreach (arr[i]) $display(\"v=%0d\", arr[i].v);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("v=1") && o.contains("v=2") && o.contains("v=3"),
        "foreach record array:\n{o}"
    );
}

// ── G: foreach over a record QUEUE ──
#[test]
fn foreach_over_record_queue() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec_t;\n\
        rec_t q[$]; rec_t x;\n\
        initial begin\n\
          x.a=10; x.b=8'd1; q.push_back(x);\n\
          x.a=20; x.b=8'd2; q.push_back(x);\n\
          foreach (q[i]) $display(\"q%0d=%0d\", i, q[i].a);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("q0=10") && o.contains("q1=20"),
        "foreach record queue:\n{o}"
    );
}

// ── E2-array: a string method on a SoA record STRING member element ──
#[test]
fn method_on_record_string_member_element() {
    let o = run("module t;\n\
        typedef struct { string name; } rec_t;\n\
        rec_t kats [];\n\
        initial begin\n\
          kats = new[2]; kats[0].name = \"prefix--1MB-of-a\";\n\
          if (kats[0].name.substr(8,15) == \"1MB-of-a\") $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "record string-member method:\n{o}"
    );
}

// ── E2-array: the tb_sha2 KAT-table idiom (foreach + member.substr.atoi) ──
#[test]
fn kat_table_foreach_substr_atoi() {
    let o = run("module t;\n\
        typedef struct { string name; int expect_len; } kat_t;\n\
        kat_t kats [];\n\
        initial begin\n\
          kats = new[2];\n\
          kats[0].name = \"vec-len-0016-a\"; kats[0].expect_len = 16;\n\
          kats[1].name = \"vec-len-0128-b\"; kats[1].expect_len = 128;\n\
          foreach (kats[i]) begin\n\
            int n; n = kats[i].name.substr(8,11).atoi();\n\
            $display(\"kat%0d parsed=%0d expect=%0d\", i, n, kats[i].expect_len);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o)
            && o.contains("kat0 parsed=16 expect=16")
            && o.contains("kat1 parsed=128 expect=128"),
        "KAT-table idiom:\n{o}"
    );
}

// ── general: a string method on a plain string dyn-array element ──
#[test]
fn method_on_plain_string_dyn_element() {
    let o = run("module t;\n\
        string sa [];\n\
        initial begin\n\
          sa = new[2]; sa[0] = \"hello-world\";\n\
          $display(\"len=%0d\", sa[0].len());\n\
          if (sa[0].substr(0,4) == \"hello\") $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("len=11") && o.contains("PASS"),
        "plain string dyn element method:\n{o}"
    );
}

// ── regression: the literal-width record queue still works ──
#[test]
fn literal_width_record_queue_unchanged() {
    let o = run("module t;\n\
        typedef struct { logic [31:0] addr; logic [7:0] len; } pkt_t;\n\
        pkt_t q[$];\n\
        initial begin\n\
          pkt_t p; p.addr = 32'h1000; p.len = 8'd4;\n\
          q.push_back(p);\n\
          if (q[0].len == 8'd4) $display(\"PASS\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "literal-width record queue:\n{o}"
    );
}

// ── correct-or-loud: a record with a NON-array-able member (event) stays loud ──
#[test]
fn record_array_with_event_member_stays_loud() {
    let o = run("module t;\n\
        typedef struct { int a; event e; } rec_t;\n\
        rec_t q[$];\n\
        initial begin $display(\"ran\"); $finish; end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "event-member record array must stay loud:\n{o}"
    );
}

// ── Q (round-19): the reviewer's exact repro — a param-width record's module net
// AND a block-local `automatic` copy in an `always_ff`, plus a queue pop-into-record.
// Both `cur_ar`/`next_cur_ar` are scalar `var_unpacked_struct` records (the
// param-width member `[ADDR_W-1:0]` makes `packable_record_layout` return None,
// routing them onto the per-member SoA path) — the whole-value copy previously had
// no surface (E3010 "undeclared").
#[test]
fn param_width_record_whole_copy() {
    let o = run("module t #(parameter int ADDR_W = 32);\n\
        logic clk=0; always #5 clk=~clk;\n\
        logic rst_n=0;\n\
        typedef struct { logic [ADDR_W-1:0] addr; logic [7:0] len; } pkt_t;\n\
        pkt_t ar_q [$];\n\
        pkt_t cur_ar;\n\
        always_ff @(posedge clk or negedge rst_n) begin\n\
            automatic pkt_t next_cur_ar;\n\
            next_cur_ar = cur_ar;\n\
            if (ar_q.size() > 0) next_cur_ar = ar_q.pop_front();\n\
            cur_ar <= next_cur_ar;\n\
        end\n\
        initial begin\n\
          pkt_t p;\n\
          p.addr = 32'hDEAD_BEEF; p.len = 8'd4;\n\
          ar_q.push_back(p);\n\
          rst_n = 1;\n\
          #12;\n\
          $display(\"addr=%h len=%0d qsize=%0d\", cur_ar.addr, cur_ar.len, ar_q.size());\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("addr=deadbeef len=4 qsize=0"),
        "param-width record whole copy (Q repro):\n{o}"
    );
}

// ── Q: two instances with DIFFERENT ADDR_W overrides — the whole-copy must use
// EACH instance's own per-instance-resolved member width, never a parse-time-frozen
// one (a frozen 32-bit assumption would truncate/misrender the 48-bit instance).
#[test]
fn param_width_record_copy_override() {
    let o = run(
        "module dut #(parameter int ADDR_W = 32) (output logic [ADDR_W-1:0] out_addr, output logic [7:0] out_len);\n\
        typedef struct { logic [ADDR_W-1:0] addr; logic [7:0] len; } pkt_t;\n\
        pkt_t a, b;\n\
        initial begin\n\
          a.addr = 48'hDEADBEEF1234;\n\
          a.len = 8'd9;\n\
          b = a;\n\
          out_addr = b.addr;\n\
          out_len = b.len;\n\
        end\n\
        endmodule\n\
        module t;\n\
          logic [7:0]  addr8;\n\
          logic [47:0] addr48;\n\
          logic [7:0]  len8, len48;\n\
          dut #(.ADDR_W(8))  d8  (.out_addr(addr8),  .out_len(len8));\n\
          dut #(.ADDR_W(48)) d48 (.out_addr(addr48), .out_len(len48));\n\
          initial begin\n\
            #1;\n\
            $display(\"addr8=%h len8=%0d addr48=%h len48=%0d\", addr8, len8, addr48, len48);\n\
            $finish;\n\
          end\n\
        endmodule\n",
    );
    assert!(
        !is_loud(&o) && o.contains("addr8=34 len8=9 addr48=deadbeef1234 len48=9"),
        "param-width record whole copy, per-instance override:\n{o}"
    );
}

// ── Q: literal-width MIXED 2-/4-state record (`int` + `logic`) — non-packable
// via the mixed-state gate (not param-width), so it also rides `var_unpacked_struct`.
#[test]
fn mixed_state_record_whole_copy() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec_t;\n\
        rec_t x, y;\n\
        initial begin\n\
          x.a = 42; x.b = 8'hAB;\n\
          y = x;\n\
          $display(\"a=%0d b=%h\", y.a, y.b);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("a=42 b=ab"),
        "mixed-state record whole copy:\n{o}"
    );
}

// ── Q: by-value semantics — mutating the source AFTER the copy must not affect
// the destination (independent per-member nets, no aliasing).
#[test]
fn record_whole_copy_by_value() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec_t;\n\
        rec_t p, q;\n\
        initial begin\n\
          p.a = 5; p.b = 8'd10;\n\
          q = p;\n\
          p.a = 99;\n\
          $display(\"qa=%0d pa=%0d\", q.a, p.a);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("qa=5 pa=99"),
        "record whole copy must be by-value:\n{o}"
    );
}

// ── Q: NBA whole-copy fidelity — `q <= p;` must land EVERY field together.
#[test]
fn record_nba_whole_copy() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec_t;\n\
        rec_t p, q;\n\
        initial begin\n\
          p.a = 7; p.b = 8'd20;\n\
          q <= p;\n\
          #1;\n\
          $display(\"a=%0d b=%0d\", q.a, q.b);\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("a=7 b=20"),
        "record NBA whole copy:\n{o}"
    );
}

// ── correct-or-loud: a whole-copy between TWO DIFFERENT record types stays loud —
// no guaranteed per-field correspondence, so a silent field-by-position copy would
// be silent-wrong (truncation / reordering). Never a partial fan-out.
#[test]
fn cross_type_copy_stays_loud() {
    let o = run("module t;\n\
        typedef struct { int a; logic [7:0] b; } rec1_t;\n\
        typedef struct { int x; logic [7:0] y; } rec2_t;\n\
        rec1_t p;\n\
        rec2_t q;\n\
        initial begin\n\
          p = q;\n\
          $display(\"ran\");\n\
          $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "cross-type record whole copy must stay loud:\n{o}"
    );
}
