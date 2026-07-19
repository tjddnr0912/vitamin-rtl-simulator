//! An ANSI `#(…)` parameter-port COMMA-LIST continuation silently lost the
//! leading param's TYPE PREFIX. In `#(parameter [3:0] A = 20, B = 20)` the
//! header loop re-parsed each comma item from scratch, so `B` came out as a
//! value-sized IMPLICIT 32-bit param instead of inheriting `[3:0]` — the
//! narrow-width truncation (`20 & 4'hF = 4`) was dropped and `B` printed 20.
//! Same for `signed`: `#(parameter signed [7:0] A = -1, B = 200)` left `B`
//! unsigned 32-bit (200) instead of the signed 8-bit wrap (-56). Silent-wrong
//! (width/signedness), found by iverilog 13.0 differential. Fix: the header
//! loop parses the type prefix ONCE per group (`parse_param_prefix`) and applies
//! it to every unadorned continuation (`finish_param_assignment`); a comma
//! followed by a fresh prefix keyword (`, parameter …`) starts a new group
//! (IEEE §6.20.1). Discriminator: a narrow declared width truncating a wider
//! initializer, observed only when the width is actually inherited.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_phci_{}_{n}", std::process::id()));
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

/// A `#(<hdr>)` module body that prints the params via `%0d`.
fn hdr(hdr: &str, args: &str, fmt: &str) -> String {
    format!(
        "module m #({hdr}) ();\n\
         initial begin $display(\"{fmt}\", {args}); #1 $finish; end endmodule\n"
    )
}

#[test]
fn continuation_inherits_narrow_width() {
    // `B` inherits `[3:0]` ⇒ 20 truncates to 4 (was 20 — implicit 32-bit).
    let (out, code) = run(&hdr(
        "parameter [3:0] A = 20, B = 20",
        "A, B",
        "A=%0d B=%0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("A=4 B=4"), "B must inherit [3:0], got:\n{out}");
}

#[test]
fn continuation_inherits_signedness_and_width() {
    // `B` inherits `signed [7:0]` ⇒ 200 wraps to -56 (was 200 — unsigned 32-bit).
    let (out, code) = run(&hdr(
        "parameter signed [7:0] A = -1, B = 200",
        "A, B",
        "A=%0d B=%0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=-1 B=-56"),
        "B must inherit signed [7:0], got:\n{out}"
    );
}

#[test]
fn three_names_share_one_prefix() {
    // All three inherit `[3:0]` ⇒ 17,18,19 truncate to 1,2,3.
    let (out, code) = run(&hdr(
        "parameter [3:0] A = 17, B = 18, C = 19",
        "A, B, C",
        "%0d %0d %0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 2 3"), "3-name shared prefix, got:\n{out}");
}

#[test]
fn fresh_prefix_keyword_starts_new_group() {
    // A comma followed by `parameter` starts a NEW group: {A,B}=[3:0], {C,D}=[7:0].
    let (out, code) = run(&hdr(
        "parameter [3:0] A = 20, B = 20, parameter [7:0] C = 300, D = 300",
        "A, B, C, D",
        "%0d %0d %0d %0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("4 4 44 44"), "two groups, got:\n{out}");
}

#[test]
fn header_localparam_comma_list() {
    // `localparam` in the header also shares its prefix across the comma-list.
    let (out, code) = run(&hdr("localparam [3:0] A = 20, B = 20", "A, B", "%0d %0d"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("4 4"), "header localparam list, got:\n{out}");
}

#[test]
fn untyped_comma_list_unchanged() {
    // An untyped comma-list stays implicit/value-sized — the fix must not perturb
    // the pre-existing (already-correct) untyped case.
    let (out, code) = run(&hdr("parameter A = 1, B = 2", "A, B", "%0d %0d"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("1 2"), "untyped list, got:\n{out}");
}

#[test]
fn typed_int_comma_list_unchanged() {
    // `parameter int A = 3, B = 5` — both 32-bit signed; product folds normally.
    let (out, code) = run(&hdr(
        "parameter int A = 3, B = 5",
        "A, B, A * B",
        "%0d %0d %0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("3 5 15"), "int list, got:\n{out}");
}
