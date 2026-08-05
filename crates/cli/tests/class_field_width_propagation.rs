//! A class field's width and sign stopped at the field itself: the moment an
//! OPERATOR touched it, the expression fell back to the 32-bit unsigned handle
//! net the field-read `Signal` actually sits on.
//!
//! `c.sb` (a `byte` holding -6) printed -6, but `~c.sb` printed 4294967045
//! instead of 5, `c.sb + 0` printed 250, and `(~c.si) < 0` was false for a
//! signed `int`. Cause: the engine built its self-width table from the language
//! rule alone and then swept vita's per-ExprId class-field sidecar over the
//! FINISHED table — which reaches the leaf and nothing above it, because every
//! parent had already been sized from the handle. The sidecar is now handed to
//! the pass and applied inline, so parents see the field (§4.5.309).
//!
//! No iverilog pin: iverilog 13 has no class support, so these are hand-IEEE
//! (§5.4.1 self-determined width / §5.5 signedness, §7.1 class properties).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cfwp_{}_{n}", std::process::id()));
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

/// The WIDTH half, and it needs `%b`.
///
/// The `%0d` rows below cannot see it: `c.sb + 0` and `~c.sb` print the same
/// number whether the field is read at eight signed bits or at thirty-two, so a
/// mutation that applies the sidecar's SIGN and forces `width: 32` passed all of
/// them (measured — the §4.5.309 differential review's M2). Each row here is
/// paired with the same expression over a PLAIN variable of the same declared
/// type: the class field must render identically to its twin, which is what
/// "the field's own width, not the 32-bit handle's" means.
#[test]
fn an_operator_over_a_class_field_renders_at_the_field_width() {
    let src = "class C; bit [3:0] n4; byte sb; endclass\n\
       module top;\n\
         reg [3:0] p4; reg signed [7:0] ps;\n\
         C c;\n\
         initial begin\n\
           c = new(); c.n4 = 4'b0010; c.sb = -8'sd6;\n\
           p4 = 4'b0010; ps = -8'sd6;\n\
           $display(\"I %b\", ~c.n4); $display(\"J %b\", ~p4);\n\
           $display(\"K %b\", ~c.sb); $display(\"L %b\", ~ps);\n\
           $finish;\n\
         end\n\
       endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "unexpected exit\n{out}");
    let line = |t: &str| {
        out.lines()
            .find_map(|l| l.strip_prefix(t).map(|r| r.trim().to_string()))
            .unwrap_or_else(|| panic!("no `{t}` line\n{out}"))
    };
    // Before the fix: `I` was 32 ones-and-zeros (`~` at the handle's width) while
    // `J` was `1101`. The pairing is the assertion — pinning `I` to a literal
    // would still pass if BOTH sides regressed to 32 bits.
    assert_eq!(
        line("I "),
        line("J "),
        "class `bit [3:0]` ≠ its plain twin\n{out}"
    );
    assert_eq!(
        line("K "),
        line("L "),
        "class `byte` ≠ its plain twin\n{out}"
    );
    // …and an anti-vacuity floor: the twin must actually be narrow, or the
    // equality above is satisfied by both being 32 bits.
    assert_eq!(line("J ").len(), 4, "expected a 4-bit render\n{out}");
    assert_eq!(line("L ").len(), 8, "expected an 8-bit render\n{out}");
}

/// Every line here was wrong before the fix EXCEPT `C` and `F` — they are the
/// control rows, the shapes an unsigned/32-bit fallback happens to get right,
/// and they are in the design so a regression that simply drops the sidecar
/// cannot pass by making the whole design print nothing.
#[test]
fn an_operator_over_a_class_field_keeps_the_field_width_and_sign() {
    let src = "class C; int si; byte sb; bit [7:0] bu; endclass\n\
       module top;\n\
         C c;\n\
         initial begin\n\
           c = new(); c.si = 5; c.sb = -8'sd6; c.bu = 8'd200;\n\
           $display(\"A %0d\", ~c.si);\n\
           $display(\"B %0d\", -c.si);\n\
           $display(\"C %0d\", c.si >>> 1);\n\
           $display(\"D %0d\", c.sb + 0);\n\
           $display(\"E %0d\", ~c.sb);\n\
           $display(\"F %0d\", c.bu * 2);\n\
           $display(\"G %0d\", (~c.si) < 0);\n\
           $display(\"H %0d\", c.sb);\n\
           $finish;\n\
         end\n\
       endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "unexpected exit\n{out}");
    // A: `int` is signed 32 → ~5 = -6 (was 4294967290, the unsigned reading).
    // B: unary minus on a signed 32 → -5 (was 4294967291).
    // C: control — 5 >>> 1 is 2 whether or not the sign survives.
    // D: `byte` is signed 8, sign-extended into the 32-bit add → -6 (was 250,
    //    zero-extended: the WIDTH was wrong too, not only the sign).
    // E: ~(-6) at the field's own 8 bits = 5 (was 4294967045 = ~250 in 32 bits).
    // F: control — 200 * 2 = 400 needs neither sign nor the narrow width.
    // G: the sign has to survive one operator to be observable in a compare.
    // H: the leaf itself — this one was already right, since the old sweep did
    //    reach it. It stays here so the test distinguishes "the sidecar is
    //    applied at all" from "it is applied where parents can see it".
    for (tag, want) in [
        ("A", "-6"),
        ("B", "-5"),
        ("C", "2"),
        ("D", "-6"),
        ("E", "5"),
        ("F", "400"),
        ("G", "1"),
        ("H", "-6"),
    ] {
        assert!(
            out.lines().any(|l| l == format!("{tag} {want}")),
            "line `{tag} {want}` missing\n{out}"
        );
    }
}
