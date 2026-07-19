//! A packed range/dimension is legal only on a vector-typed declaration — a net
//! or `logic`/`reg`/`bit` (IEEE §6.11 `integer_vector_type`). A fixed-width integer
//! atom (`byte`/`shortint`/`int`/`longint`/`integer`/`time`) and the dimensionless
//! types (`real`/`string`/`event`) take NO packed dims. vita used to SILENTLY accept
//! them: a single range was dropped (`byte [7:0] x` sized 8, self-consistent but
//! non-conformant), and a SECOND packed dim genuinely diverged (`byte [7:0][1:0] x`
//! — `packed_extents` folds 8×2=16 while `range_to_dims`/`$bits` report 8). iverilog
//! rejects all of them. §4.5.156 makes the parser (module/block/func-body/class-member
//! decl path) reject them loudly; sibling decl paths stay lenient (ROADMAP §3).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (stderr, exit_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_adr_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn mk(decl: &str) -> String {
    format!("module t; {decl} initial $finish; endmodule\n")
}

/// A fixed-width integer atom carrying a packed range is rejected loudly (was
/// silently accepted with an inconsistent net width).
#[test]
fn atom_with_packed_range_is_rejected() {
    for decl in [
        "byte [7:0] x;",
        "shortint [1:0] x;",
        "int [3:0] x;",
        "longint [0:0] x;",
        "integer [7:0] x;",
        "byte [7:0][1:0] x;", // multi-packed
        "real [1:0] x;",      // dimensionless real
        "event [3:0] ev;",    // event (moved from legality_semantics — now parse-time)
    ] {
        let (err, ok) = run(&mk(decl));
        assert!(
            !ok && err.contains("VITA-E2002") && err.contains("vector-typed"),
            "[{decl}] should be rejected; got ok={ok} err:\n{err}"
        );
    }
}

/// §4.5.156 (§3 全 site): an atom+packed-range is rejected at EVERY declaration
/// position via the shared `reject_packed_dims_on_nonvector` guard — not just a
/// plain var decl, but typedef alias, module port (ANSI + non-ANSI), tf-port (ANSI +
/// non-ANSI), struct member, typed param, and for-typed-init. iverilog rejects all.
#[test]
fn atom_with_packed_range_rejected_at_all_decl_sites() {
    let srcs = [
        ("typedef", "module t; typedef int [3:0] tt; tt x; initial $finish; endmodule"),
        ("port-ANSI", "module m(input byte [7:0] p); endmodule"),
        ("port-nonANSI", "module m(p); input byte [7:0] p; endmodule"),
        (
            "tf-port-ANSI",
            "module t; task tk(input int [3:0] a); endtask initial $finish; endmodule",
        ),
        (
            "tf-port-nonANSI",
            "module t; task tk; input byte [7:0] a; endtask initial $finish; endmodule",
        ),
        (
            "struct-member",
            "module t; typedef struct packed { int [3:0] f; } s_t; s_t s; initial $finish; endmodule",
        ),
        (
            "typed-param",
            "module t; localparam real [1:0] Q = 0; initial $finish; endmodule",
        ),
        (
            "for-init",
            "module t; initial for (byte [3:0] i=0; i<2; i++) ; endmodule",
        ),
        (
            "enum-base",
            "module t; typedef enum byte [3:0] { A, B } e_t; e_t e; initial $finish; endmodule",
        ),
        (
            "class-value-param",
            "module t; class C #(int [3:0] X = 0); endclass initial $finish; endmodule",
        ),
        (
            "func-return-type",
            "module t; function int [7:0] f; f=0; endfunction initial $finish; endmodule",
        ),
    ];
    for (site, src) in srcs {
        let (err, ok) = run(src);
        assert!(
            !ok && err.contains("vector-typed"),
            "[{site}] atom+range should be rejected; got ok={ok} err:\n{err}"
        );
    }
}

/// BYTE-IDENTITY: every legal declaration still elaborates cleanly — a scalar atom,
/// a vector type WITH a packed range (`logic`/`reg`/`bit`), a net with a range, an
/// unpacked array of an atom, a multi-packed vector, and `real`/`string`.
#[test]
fn legal_decls_still_accepted() {
    for decl in [
        "byte x;",
        "int x;",
        "longint x;",
        "bit [7:0] x;",
        "logic [3:0] x;",
        "reg [15:0] x;",
        "wire [7:0] x;",
        "byte x[4];",          // unpacked array of atom
        "logic [3:0][7:0] x;", // multi-packed vector
        "real x;",
        "string x;",
    ] {
        let (err, ok) = run(&mk(decl));
        assert!(
            ok && !err.contains("error["),
            "[{decl}] should be accepted; got ok={ok} err:\n{err}"
        );
    }
}

/// The reject is scoped to PACKED dims: an unpacked dimension on an atom (a legal
/// array) is unaffected, and `$bits` of that array element is the kind width.
#[test]
fn unpacked_atom_array_unaffected() {
    let (err, ok) = run("module t; byte b[3]; int i[2];\n\
         initial begin $display(\"%0d %0d\", $bits(b[0]), $bits(i[0])); $finish; end endmodule\n");
    assert!(ok && !err.contains("error["), "got ok={ok} err:\n{err}");
}
