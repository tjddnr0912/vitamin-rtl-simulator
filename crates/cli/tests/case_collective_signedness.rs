//! `case`/`casez`/`casex` selector COLLECTIVE signedness (§12.5 / §11.8.1). The
//! case comparison is signed ONLY when the scrutinee AND every case-item label
//! are signed; if ANY participant is unsigned the whole comparison is unsigned
//! (all operands zero-extend). vita's engine sizes each `CaseEq(scrut,label)`
//! pair independently (`signed(l)&&signed(r)`), so a SIGNED scrutinee used to
//! sign-extend against a signed label even when an unsigned sibling label made
//! the set collectively unsigned — taking the wrong branch, silently:
//! `case(s) -1: ; 4'hF: ;` with signed `s=4'hF` matched `-1` (vita) instead of
//! `4'hF` (iverilog). Fixed in `lower_case` by forcing the scrutinee `$unsigned`
//! once when the label set is collectively unsigned (then every pair-signedness
//! is false → scrutinee AND each label zero-extend). Pre-existing (independent of
//! any struct type — reproduces with a plain `reg signed`); surfaced while adding
//! signed packed-struct value semantics ([[signed_packed_struct]]). Pinned to
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ccs_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

/// Signed scrutinee + an unsigned sibling label ⇒ collectively unsigned: the
/// scrutinee zero-extends, so `4'hF` (0x0000000F) does NOT match `-1`. Plain
/// `reg signed` — proves the bug is struct-independent.
#[test]
fn plain_signed_scrut_unsigned_label_is_collective_unsigned() {
    let (o, ok) = run(
        "module t; reg signed [3:0] s;\n\
         initial begin s = 4'hF;\n\
           case (s) -1: $display(\"neg1\"); 4'hF: $display(\"hF\"); default: $display(\"none\"); end\
           case\n #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("hF") && !o.contains("neg1"), "got:\n{o}");
}

/// The signed packed-struct selector (the case that surfaced the bug): same
/// collective-unsigned resolution.
#[test]
fn signed_struct_scrut_unsigned_label_is_collective_unsigned() {
    let (o, ok) = run(
        "module t;\n\
         typedef struct packed signed { logic [3:0] a; } s_t; s_t s;\n\
         initial begin s.a = 4'hF;\n\
           case (s) -1: $display(\"neg1\"); 4'hF: $display(\"hF\"); default: $display(\"none\"); end\
           case\n #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("hF") && !o.contains("neg1"), "got:\n{o}");
}

/// All participants signed ⇒ collectively SIGNED: the scrutinee sign-extends and
/// matches `-1`. Must NOT be disturbed by the fix.
#[test]
fn all_signed_case_stays_signed() {
    let (o, ok) = run("module t; reg signed [3:0] s;\n\
         initial begin s = 4'hF;\n\
           case (s) -1: $display(\"neg1\"); default: $display(\"none\"); endcase\n\
           #1 $finish; end endmodule");
    assert!(ok && o.contains("neg1"), "got:\n{o}");
}

/// A NARROW signed label that widens into the compare context also zero-extends
/// under a collectively-unsigned set (forcing the scrutinee unsigned makes the
/// pair unsigned, so the label zero-extends too — no residual).
#[test]
fn narrow_signed_label_zero_extends_when_collective_unsigned() {
    let (o, ok) = run("module t; reg signed [7:0] w;\n\
         initial begin w = 8'h0F;\n\
           case (w) 4'shF: $display(\"A\"); 8'h00: $display(\"B\"); default: $display(\"def\"); end\
           case\n #1 $finish; end endmodule");
    // 4'shF (=-1) zero-extends to 0x0F under collective-unsigned → matches 0x0F
    assert!(ok && o.contains("A") && !o.contains("def"), "got:\n{o}");
}

/// BYTE-IDENTITY: an UNSIGNED scrutinee is already collectively unsigned for any
/// label mix — the fix is a no-op (its pairs were already unsigned).
#[test]
fn unsigned_scrut_unchanged() {
    let (o, ok) = run(
        "module t; reg [3:0] u;\n\
         initial begin u = 4'hF;\n\
           case (u) -1: $display(\"neg1\"); 4'hF: $display(\"hF\"); default: $display(\"none\"); end\
           case\n #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("hF") && !o.contains("neg1"), "got:\n{o}");
}

/// casez with a signed scrutinee and an unsigned wildcard pattern resolves
/// collectively unsigned too (the wildcard match still fires).
#[test]
fn casez_signed_scrut_collective() {
    let (o, ok) = run(
        "module t; reg signed [3:0] s;\n\
         initial begin s = 4'b1010;\n\
           casez (s) 4'b1??0: $display(\"match\"); -1: $display(\"neg1\"); default: $display(\"def\"); end\
           case\n #1 $finish; end endmodule",
    );
    assert!(ok && o.contains("match"), "got:\n{o}");
}
