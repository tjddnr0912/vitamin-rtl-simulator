//! The index seal (§4.5.308/309) asks a question about an expression WHILE the
//! expression is still being built, and a hierarchical reference is not built
//! yet at that point — it is a placeholder (`Signal{net: POISON_NET}` /
//! `Call{func: POISON_FID}`) patched in place once every instance exists.
//!
//! Two defects lived in that gap, both found by the §4.5.309 adversarial review,
//! both invisible to every other test in this repo because **no test anywhere
//! used a deferred/hierarchical expression as an index**:
//!
//! 1. Asking the canonical width rule about a placeholder got a FABRICATED
//!    answer (1-bit unsigned, the "net not there" fallback) rather than a wrong
//!    one — and the seal read that as "unsigned", zero-extending an index that
//!    was about to become a signed −1. `mg[u.k]` on `reg [7:0] mg[-3:2]` went
//!    from the oracle's `aa` to `x` plus an E4002 and exit 1: correct-support
//!    down to loud-wrong.
//! 2. The answer was then MEMOIZED, and the memo was never invalidated when the
//!    placeholder got patched. So the answer for one statement depended on
//!    whether an unrelated later statement had happened to extend the
//!    expression arena first — adding three lines about a different net at the
//!    END of the file silently changed the result of a write ABOVE it.
//!
//! Oracle: iverilog 13 (all six designs compile and run there).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hidx_{}_{n}", std::process::id()));
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

const SUB: &str = "module sub;\n\
     integer k;\n\
     localparam integer P = -1;\n\
     function automatic integer f(input integer v); f = v; endfunction\n\
   endmodule\n";

fn negarr(idx: &str) -> String {
    format!(
        "{SUB}module top;\n\
           sub u();\n\
           reg [7:0] mg [-3:2];\n\
           integer q;\n\
           initial begin\n\
             u.k = -1;\n\
             for (q=-3; q<=2; q=q+1) mg[q] = 8'hAA;\n\
             $display(\"R %h\", mg[{idx}]);\n\
             $finish;\n\
           end\n\
         endmodule\n"
    )
}

/// A hierarchical signal, a hierarchical `localparam` and a hierarchical
/// function call are three DIFFERENT placeholders (two `Signal` spellings and a
/// `Call`), resolved by three different passes. All three reach the seal.
#[test]
fn a_hierarchical_index_is_not_read_as_an_unsigned_one_bit_net() {
    for idx in ["u.k", "u.P", "u.f(-1)"] {
        let (out, code) = run(&negarr(idx));
        assert!(
            out.contains("R aa"),
            "index `{idx}`: expected the oracle's `R aa`\n{out}"
        );
        assert_eq!(code, Some(0), "index `{idx}`: must not go loud\n{out}");
    }
}

/// The same access as a WRITE. Kept separate because the write funnel resolves
/// through `resolve_deferred_hier_sel_write`, a different call into the seal —
/// and because a dropped write is SILENT where a dropped read at least reads x.
#[test]
fn a_hierarchical_index_write_lands_where_the_oracle_puts_it() {
    let src = format!(
        "{SUB}module top;\n\
           sub u();\n\
           reg [7:0] mg [-3:2];\n\
           integer q;\n\
           initial begin\n\
             u.k = -1;\n\
             for (q=-3; q<=2; q=q+1) mg[q] = 8'h11;\n\
             mg[u.k] = 8'hAA;\n\
             $display(\"W %h %h\", mg[-1], mg[-3]);\n\
             $finish;\n\
           end\n\
         endmodule\n"
    );
    let (out, code) = run(&src);
    assert!(out.contains("W aa 11"), "expected `W aa 11`\n{out}");
    assert_eq!(code, Some(0), "must not go loud\n{out}");
}

/// The memo test. Three designs that differ ONLY in unrelated trailing/leading
/// statements must agree — that is the whole property, and it is exactly what a
/// stale never-invalidated cache breaks. `PLAIN` and `AFTER` were measurably
/// different before the fix (`00000001` vs `00000000`, both at exit 0).
#[test]
fn an_unrelated_later_statement_cannot_change_an_earlier_hierarchical_write() {
    let body = |pre: &str, post: &str| {
        format!(
            "module sub;\n  integer k;\n  reg [33:2] bus;\nendmodule\n\
             module top;\n\
               sub u();\n\
               reg [33:2] lbus;\n\
               integer j;\n\
               initial begin\n\
                 {pre}\
                 u.k = -1;\n\
                 u.bus = 32'h0;\n\
                 u.bus[u.k +: 4] = 4'b1111;\n\
                 $display(\"A %h\", u.bus);\n\
                 {post}\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    let filler = "lbus = 0; j = 1;\n lbus[j] = 1'b1;\n $display(\"B %h\", lbus);\n";
    let plain = run(&body("", ""));
    let after = run(&body("", filler));
    let before = run(&body(filler, ""));
    for (tag, (out, code)) in [("plain", &plain), ("after", &after), ("before", &before)] {
        assert_eq!(code, &Some(0), "{tag}: unexpected exit\n{out}");
        assert!(
            out.lines().any(|l| l == "A 00000001"),
            "{tag}: expected the oracle's `A 00000001`\n{out}"
        );
    }
}

/// `elaborate`'s own driver over the shared rule resolves a user function's
/// return type through `func_metas`. Nothing else exercises that arm — a
/// mutation replacing the resolver with `|_| None` survived the whole suite,
/// while flipping this design's answer from `aa` to `xx`. Both declaration
/// orders, because the function's metadata is reserved before bodies lower and
/// "declared after the use" is the order that would break if it were not.
#[test]
fn a_function_return_type_reaches_the_seal_in_either_declaration_order() {
    let mk = |fn_first: bool| {
        let f = "function automatic signed [7:0] fs(input integer v); fs = v; endfunction\n";
        let (a, b) = if fn_first { (f, "") } else { ("", f) };
        format!(
            "module top;\n\
               {a}  reg [33:2] d2;\n\
               {b}  initial begin\n\
                 d2 = 32'h0;\n\
                 d2[fs(8'd6)] = 1'b1;\n\
                 $display(\"F %h\", d2);\n\
                 $finish;\n\
               end\n\
             endmodule\n"
        )
    };
    for fn_first in [true, false] {
        let (out, code) = run(&mk(fn_first));
        assert_eq!(code, Some(0), "fn_first={fn_first}\n{out}");
        assert!(
            out.contains("F 00000010"),
            "fn_first={fn_first}: expected `F 00000010`\n{out}"
        );
    }
}
