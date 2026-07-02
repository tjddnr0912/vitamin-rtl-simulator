//! Optional `: name` end-label on CONTAINER declarations (IEEE 1800 §9.3.4/§26/
//! §27): `endmodule : m`, `endpackage : p`, `endinterface : i`, `endprogram : p`.
//! vita previously parse-rejected these (E2002 "expected 'module', found Colon")
//! while already accepting the label on endfunction/endtask/endclass/block/
//! generate ends — so 14/18 files of a real external design failed to parse.
//!
//! The fix consumes the optional label after the container end keyword and
//! ignores it (accept-and-ignore), matching the established policy for every
//! other end-label in the parser. A mismatched label is NOT silent-wrong: the
//! container name is already fixed by the header, so the label text cannot change
//! elaboration. Supported cases are pinned to iverilog 13.0; the no-label form is
//! byte-identical (the label consumer is a no-op without a colon).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_lblend_{}_{n}", std::process::id()));
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
fn endmodule_labeled() {
    let (out, code) = run("module m;\n\
         initial begin $display(\"M\"); $finish; end\n\
         endmodule : m\n");
    assert_eq!(code, Some(0), "endmodule : m must parse+run:\n{out}");
    assert!(out.contains("M"), "{out}");
}

#[test]
fn endpackage_labeled() {
    let (out, code) = run("package p;\n\
         localparam int W = 4;\n\
         endpackage : p\n\
         module m; initial begin $display(\"P=%0d\", p::W); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "endpackage : p must parse+run:\n{out}");
    assert!(out.contains("P=4"), "{out}");
}

#[test]
fn endinterface_labeled() {
    let (out, code) = run("interface ifc; logic x; endinterface : ifc\n\
         module m; initial begin $display(\"I\"); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "endinterface : ifc must parse+run:\n{out}");
    assert!(out.contains("I"), "{out}");
}

#[test]
fn endprogram_labeled() {
    let (out, code) = run("program p; initial begin $display(\"PR\"); end endprogram : p\n");
    assert_eq!(code, Some(0), "endprogram : p must parse+run:\n{out}");
    assert!(out.contains("PR"), "{out}");
}

#[test]
fn endprimitive_labeled() {
    // UDP end-label `endprimitive : name` (IEEE 1800 §29.3) — same accept-and-
    // ignore, handled in parse_udp_decl. Pinned to iverilog: the AND UDP outputs
    // 1 then 0.
    let (out, code) = run(
        "primitive myand(o, a, b);\n\
         output o; input a, b;\n\
         table 1 1 : 1; 0 ? : 0; ? 0 : 0; endtable\n\
         endprimitive : myand\n\
         module m;\n\
         reg a, b; wire o; myand u(o, a, b);\n\
         initial begin a=1; b=1; #1 $display(\"UDP=%b\", o); a=0; #1 $display(\"UDP=%b\", o); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "endprimitive : myand must parse+run:\n{out}");
    assert!(out.contains("UDP=1"), "{out}");
    assert!(out.contains("UDP=0"), "{out}");
}

#[test]
fn endmodule_mismatched_label_accepted() {
    // Accept-and-ignore, consistent with the established policy for every other
    // end-label (endfunction/endtask/block); the container name is `m` regardless
    // of the label text, so the output is identical to the matching-label form.
    let (out, code) = run("module m;\n\
         initial begin $display(\"MM\"); $finish; end\n\
         endmodule : wrongname\n");
    assert_eq!(code, Some(0), "mismatched end-label accepted:\n{out}");
    assert!(out.contains("MM"), "{out}");
}

#[test]
fn endmodule_unlabeled_still_ok() {
    // The label consumer is a no-op without a colon: the plain form is unchanged.
    let (out, code) = run("module m;\n\
         initial begin $display(\"U\"); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "plain endmodule unchanged:\n{out}");
    assert!(out.contains("U"), "{out}");
}
