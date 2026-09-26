//! A real stored into a CLASS FIELD or a CONTAINER ELEMENT takes the assignment conversion
//! (IEEE 1800 §6.12.2) at the destination's own width and sign (ROADMAP §2 "Real", the
//! class-field / container-element bullet §4.5.536 recorded).
//!
//! The mechanism: the engine's write funnel converted a real at the HANDLE net's width
//! (`write_lvalue_general`, 32 bits for a class handle) and the class-field funnel then
//! zero-extended it (`longint f; c.f = -2.5` read `00000000fffffffd`, a `[129:0]` field
//! `…0fffffffd`; both oracles sign-complete), the tier-3 funnel converted nothing (the IEEE-754
//! word at every width, in a field and in every container element), and a queue push / insert
//! evaluated its argument real and stored the word on every backend (`int q[$];
//! q.push_back(300.5)` → `4072c80000000000`, both oracles `0000012d`). Now `class_field_write_with`
//! converts a real at the field's width and sign, `coerce_dyn_elem` (the one funnel every dyn
//! array, queue, assoc and string-keyed assoc store and every push / insert goes through)
//! converts at the element's, and the engine's pre-coercion leaves a class-field store to its
//! funnel. Every cell runs on all three backends; every value is verilator's, iverilog's where
//! it runs the shape (it aborts on a real pushed into a `longint aa[int]` and on a real actual
//! of a class method).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rscc_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(&path);
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.output().expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
            && !l.contains("W-RUN-BACKEND-FALLBACK")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita_on(src, None);
    assert!(ok, "expected exit 0, got:\n{s}");
    for b in ["interp", "vm", "native"] {
        let (t, ok) = vita_on(src, Some(b));
        assert!(ok, "backend {b}: expected exit 0, got:\n{t}");
        assert_eq!(t, s, "backend {b} diverges from the default on:\n{src}");
    }
    s
}

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        assert_eq!(run(src), *want, "design:\n{src}");
    }
}

#[test]
fn a_class_field_of_every_width_takes_the_conversion() {
    check(&[
        (
            "module t; class C; byte b; int g; longint f; logic [129:0] w; bit [7:0] u; shortint s; endclass\n\
             C c; initial begin real rv = -2.5; real big = 300.5; c = new;\n\
             c.b = rv; c.g = rv; c.f = rv; c.w = rv; c.u = rv; c.s = rv;\n\
             $display(\"b=%h g=%h f=%h w=%h u=%h s=%h\", c.b, c.g, c.f, c.w, c.u, c.s);\n\
             c.b = big; c.g = big; c.f = big; c.w = big; c.u = big; c.s = big;\n\
             $display(\"b=%h g=%h f=%h w=%h u=%h s=%h\", c.b, c.g, c.f, c.w, c.u, c.s);\n\
             c.f = 1.0e40; c.w = 1.0e40; $display(\"f=%h w=%h\", c.f, c.w); #1 $finish; end endmodule\n",
            "b=fd g=fffffffd f=fffffffffffffffd w=3fffffffffffffffffffffffffffffffd u=fd s=fffd\n\
             b=2d g=0000012d f=000000000000012d w=00000000000000000000000000000012d u=2d s=012d\n\
             f=0000000000000000 w=16329f1c35ca500000000000000000000\n",
        ),
        // Inside a method (the frame lane) and through a nonblocking store (iverilog's values;
        // it then aborts internally on the real actual).
        (
            "module t; class C; longint f; int g; byte b; function void set(real x); f = x; g = x; b = x; endfunction endclass\n\
             C c; initial begin real rv = -2.5; c = new; c.set(rv); $display(\"f=%h g=%h b=%h\", c.f, c.g, c.b);\n\
             c.f <= 300.5; c.b <= -300.5; #1 $display(\"f=%h b=%h\", c.f, c.b); #1 $finish; end endmodule\n",
            "f=fffffffffffffffd g=fffffffd b=fd\nf=000000000000012d b=d3\n",
        ),
    ]);
}

#[test]
fn every_container_element_store_takes_the_conversion() {
    check(&[
        // Queue index store, push_back, dynamic-array element, assoc element, at 32 / 64 / 8 bits.
        (
            "module t; int q[$]; int dy[]; int aa[int]; longint lq[$]; byte bq[$];\n\
             initial begin real rv = 300.5; real ng = -2.5;\n\
             q.push_back(0); q[0] = rv; q.push_back(rv); q.push_back(ng);\n\
             dy = new[2]; dy[0] = rv; dy[1] = ng;\n\
             aa[3] = rv; aa[4] = ng;\n\
             lq.push_back(ng); lq.push_back(1.0e40); bq.push_back(rv); bq.push_back(ng);\n\
             $display(\"q=%h %h %h dy=%h %h aa=%h %h\", q[0], q[1], q[2], dy[0], dy[1], aa[3], aa[4]);\n\
             $display(\"lq=%h %h bq=%h %h\", lq[0], lq[1], bq[0], bq[1]); #1 $finish; end endmodule\n",
            "q=0000012d 0000012d fffffffd dy=0000012d fffffffd aa=0000012d fffffffd\n\
             lq=fffffffffffffffd 0000000000000000 bq=2d fd\n",
        ),
        // Insert, a `longint` assoc, an unsigned element with a negative real, a `shortint`
        // wrapping a large real.
        (
            "module t; longint lq[$]; longint la[int]; byte bq[$]; bit [7:0] uq[$]; shortint sd[];\n\
             initial begin real ng = -2.5; real big = 300.5;\n\
             lq.push_back(0); lq[0] = ng; lq.insert(1, big); la[1] = ng; la[2] = 1.0e40;\n\
             bq.push_back(0); bq[0] = big; uq.push_back(ng); uq.push_back(0); uq[1] = big;\n\
             sd = new[2]; sd[0] = 70000.4; sd[1] = ng;\n\
             $display(\"lq=%h %h la=%h %h bq=%h uq=%h %h sd=%h %h\", lq[0], lq[1], la[1], la[2], bq[0], uq[0], uq[1], sd[0], sd[1]); #1 $finish; end endmodule\n",
            "lq=fffffffffffffffd 000000000000012d la=fffffffffffffffd 0000000000000000 bq=2d uq=fd 2d sd=1170 fffd\n",
        ),
        // A string-keyed assoc, a queue assignment pattern, and non-finite reals (0, as every
        // other integral store since §4.5.536; verilator).
        (
            "module t; int sa[string]; int q[$]; longint lq[$]; int dy[]; byte bq[$];\n\
             initial begin real rv = 300.5; real ng = -2.5; real inf = 1.0/0.0; real nan = 0.0/0.0;\n\
             sa[\"a\"] = rv; sa[\"b\"] = ng; q = '{rv, ng, 7.5}; lq.push_back(inf); lq.push_back(nan); bq.push_back(inf);\n\
             dy = new[2]; dy[0] = rv; dy[1] = ng;\n\
             $display(\"sa=%h %h q=%h %h %h lq=%h %h bq=%h dy=%h %h\", sa[\"a\"], sa[\"b\"], q[0], q[1], q[2], lq[0], lq[1], bq[0], dy[0], dy[1]); #1 $finish; end endmodule\n",
            "sa=0000012d fffffffd q=0000012d fffffffd 00000008 lq=0000000000000000 0000000000000000 bq=00 dy=0000012d fffffffd\n",
        ),
        // A 130-bit element with a huge real, a 4-bit 2-state element, a `foreach` store.
        (
            "module t; logic [129:0] wq[$]; bit [3:0] nq[$]; int arr[3];\n\
             initial begin real big = 1.0e40; real ng = -2.5; real half = 2.5;\n\
             wq.push_back(big); wq.push_back(ng); nq.push_back(ng); nq.push_back(half); nq.push_back(-2.5);\n\
             foreach (arr[i]) arr[i] = half * i;\n\
             $display(\"wq=%h %h nq=%h %h %h arr=%h %h %h\", wq[0], wq[1], nq[0], nq[1], nq[2], arr[0], arr[1], arr[2]); #1 $finish; end endmodule\n",
            "wq=16329f1c35ca500000000000000000000 3fffffffffffffffffffffffffffffffd nq=d 3 d arr=00000000 00000003 00000005\n",
        ),
        // A part-select of a dynamic-array element (the deposit lane): converted at the
        // element's width, then sliced (verilator; iverilog aborts on the shape; the native
        // backend sliced the IEEE-754 word — review round 1).
        (
            "module t; int dy[]; longint ld[]; byte bd[]; int m[0:1]; int v; real r;\n\
             initial begin r = -70000.4;\n\
             dy = new[2]; dy[0][15:0] = r; dy[1][7:0] = 2.5; $display(\"dy0[15:0] %h dy1[7:0] %h\", dy[0], dy[1]);\n\
             ld = new[1]; ld[0][63:32] = -2.5; $display(\"ld0[63:32] %h\", ld[0]);\n\
             bd = new[1]; bd[0][3:0] = -2.5; $display(\"bd0[3:0] %h\", bd[0]);\n\
             m[0] = 0; m[0][15:0] = r; v = 0; v[15:0] = r; $display(\"ctl %h %h\", m[0], v);\n\
             #1 $finish; end endmodule\n",
            "dy0[15:0] 0000ee90 dy1[7:0] 00000003\nld0[63:32] fffffffd00000000\nbd0[3:0] 0d\nctl 0000ee90 0000ee90\n",
        ),
        // A `string` element: the real is an integer first (§6.12.2), then §6.16 (verilator
        // `"\x03"`, `"a"`, `"b"`; iverilog aborts on the shape). It stored the IEEE-754 bytes
        // (native) or a one-bit value (interpreter) before.
        (
            "module t; string sq[$]; string sd[]; real rv;\n\
             initial begin rv = 2.5; sd = new[1];\n\
             sq.push_back(rv); sq.push_back(\"zz\"); sq[1] = 97.2; sd[0] = 98.4;\n\
             $display(\"%0d %0d %0d\", sq[0].len(), sq[1].len(), sd[0].len());\n\
             $display(\"%0d %0d %0d\", sq[0].getc(0), sq[1].getc(0), sd[0].getc(0));\n\
             $display(\"%0d %0d\", sq[1] == \"a\", sd[0] == \"b\");\n\
             #1 $finish; end endmodule\n",
            "1 1 1\n3 97 98\n1 1\n",
        ),
        // The controls: a plain variable, a module array element, a wide net (unchanged).
        (
            "module t; longint f; int arr[4]; logic [129:0] w;\n\
             initial begin real rv = -2.5; f = rv; arr[1] = rv; w = rv; $display(\"f=%h arr=%h w=%h\", f, arr[1], w);\n\
             f = 1.0e40; $display(\"f=%h\", f); #1 $finish; end endmodule\n",
            "f=fffffffffffffffd arr=fffffffd w=3fffffffffffffffffffffffffffffffd\nf=0000000000000000\n",
        ),
    ]);
}
