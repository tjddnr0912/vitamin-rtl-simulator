//! R16 §3.3: a fixed-size unpacked array declared `automatic` in a procedural block.
//!
//! Two shapes were loud, and between them they made the type essentially unusable:
//!
//!   (a) a `'{…}` DECLARATION INITIALIZER. The identical declaration with a DYNAMIC
//!       `[]` dim was accepted with the same contents and the same element type — the
//!       only thing missing was the per-entry classifier arm. Re-initializing a fixed
//!       array is the whole-array assign `a = '{…}`, a statement that already lowered
//!       correctly including under `automatic`, so nothing new had to be emitted.
//!
//!   (b) an ELEMENT-BY-ELEMENT fill with no initializer. The definite-assignment walk
//!       counted only a WHOLE-array write as a first write, so `a[0]=…; a[3]=…;`
//!       followed by a read was rejected as read-before-write although every element
//!       had been written.
//!
//! WHAT (b) IS NOT. The obvious fix — mark the local per-entry and reset it to the
//! type default on each block entry — is WRONG, and was reverted after measurement.
//! Automatic storage is created per ACTIVATION, not per block entry: in iverilog, a
//! block inside an `automatic` task entered by three loop iterations prints
//! `xx, 10, 11` (the leftover survives), while three separate CALLS print `xx, xx, xx`.
//! An initializer does re-run on each entry (`w=11` every iteration) — which is why
//! (a) is a per-entry local and (b) is not. So (b) is closed by PROVING coverage
//! instead: literal indices, at the top level of the block, with an rhs that cannot
//! read the array. Anything less provable stays loud.
//!
//! Every accepted case here is pinned against iverilog, using the `automatic`-task
//! form (an un-keyworded local inside one IS automatic, IEEE 1800 §6.21) since
//! iverilog rejects the explicit lifetime override.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fauto_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

fn runs(src: &str, want: &[&str]) {
    let (o, ok) = run(src);
    assert!(ok, "expected acceptance, got:\n{o}");
    let got: Vec<&str> = o.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(got, want, "output mismatch:\n{o}");
}

fn loud(src: &str, who: &str) {
    let (o, ok) = run(src);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
    assert!(
        o.contains(&format!("`{who}`")),
        "expected the diagnostic to name `{who}`, got:\n{o}"
    );
}

/// The report's reproducer, all four element types at once. iverilog prints `R PASS`.
#[test]
fn decl_init_all_element_types() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic byte        msg  []  = '{8'h61, 8'h62, 8'h63};
                 automatic logic [4:0] modes[4] = '{5'h1C, 5'h1D, 5'h1E, 5'h1F};
                 automatic string      names[4] = '{"N1", "N2", "N3", "N4"};
                 automatic int         ints [4] = '{1, 2, 3, 4};
                 if (msg[2] == 8'h63 && modes[3] == 5'h1F && names[3] == "N4" && ints[3] == 4)
                   $display("R PASS");
               end
             end
           endmodule"#,
        &["R PASS"],
    );
}

/// The report's PASS boundary: the same contents with a dynamic dim already worked.
#[test]
fn dynamic_dim_boundary_still_works() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic byte msg [] = '{8'h61, 8'h62, 8'h63};
                 $display("R %0h %0d", msg[2], msg.size());
               end
             end
           endmodule"#,
        &["R 63 3"],
    );
}

/// A single-element array — the report noted `[1]` failed too.
#[test]
fn single_element_array() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int one [1] = '{7};
                 $display("R %0d", one[0]);
               end
             end
           endmodule"#,
        &["R 7"],
    );
}

/// A declared RANGE rather than a size, and a descending one. iverilog prints
/// `R 1 4` (the pattern fills left-to-right over the declared order).
#[test]
fn declared_range_dim() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int r [3:0] = '{1, 2, 3, 4};
                 $display("R %0d %0d", r[3], r[0]);
               end
             end
           endmodule"#,
        &["R 1 4"],
    );
}

/// (b): a complete element-by-element fill. iverilog prints `R 1 4`.
#[test]
fn complete_element_fill_is_supported() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int a [4];
                 a[0] = 1; a[1] = 2; a[2] = 3; a[3] = 4;
                 $display("R %0d %0d", a[0], a[3]);
               end
             end
           endmodule"#,
        &["R 1 4"],
    );
}

/// (b) over a declared range whose low bound is not zero — the coverage set is keyed
/// on declared indices, not word slots.
#[test]
fn complete_element_fill_over_a_declared_range() {
    runs(
        r#"module t;
             initial begin
               begin
                 automatic int a [3:1];
                 a[1] = 1; a[2] = 2; a[3] = 3;
                 $display("R %0d", a[3]);
               end
             end
           endmodule"#,
        &["R 3"],
    );
}

/// SOUNDNESS PIN. An INCOMPLETE fill followed by a read of an unwritten element is a
/// genuine read-before-write on the flatten — it would observe the previous block
/// entry's leftover.
#[test]
fn partial_element_fill_stays_loud() {
    loud(
        r#"module t;
             initial begin
               begin
                 automatic int a [4];
                 a[0] = 1; a[1] = 2;
                 if (a[3] == 0) $display("R read-unwritten");
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. The report's own leftover case: a block entered twice writing only
/// element 0, then reading element 1. This is exactly where the per-entry reset would
/// have been observable, and where iverilog shows the leftover surviving — so vita
/// must stay loud rather than pick either answer silently.
#[test]
fn leftover_across_entries_stays_loud() {
    loud(
        r#"module t;
             initial begin
               for (int e = 0; e < 2; e++) begin
                 automatic logic [7:0] a [3];
                 a[0] = e;
                 $display("R e=%0d a1=%0h", e, a[1]);
                 a[1] = 8'hAA;
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. A COMPUTED index proves nothing about coverage.
#[test]
fn computed_index_fill_stays_loud() {
    loud(
        r#"module t;
             initial begin
               begin
                 automatic int a [4];
                 automatic int k;
                 k = 0;
                 a[k] = 1; a[k+1] = 2; a[k+2] = 3; a[k+3] = 4;
                 $display("R %0d", a[3]);
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. An element write whose rhs READS the array observes an element that
/// has not been written yet.
#[test]
fn self_referencing_element_write_stays_loud() {
    loud(
        r#"module t;
             initial begin
               begin
                 automatic int a [2];
                 a[0] = a[1];
                 a[1] = 5;
                 $display("R %0d", a[0]);
               end
             end
           endmodule"#,
        "a",
    );
}

/// SOUNDNESS PIN. Writes hidden inside a conditional are not unconditional coverage.
#[test]
fn conditional_element_fill_stays_loud() {
    loud(
        r#"module t;
             int c = 0;
             initial begin
               begin
                 automatic int a [2];
                 if (c) begin a[0] = 1; a[1] = 2; end
                 $display("R %0d", a[0]);
               end
             end
           endmodule"#,
        "a",
    );
}
