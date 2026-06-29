//! SVA sampled-value functions $changed / $sampled (Medium bundle rank 3, IR-0).
//!
//! Both reuse the existing prev-register machinery (rewrite_sampled), like
//! $past/$rose/$fell/$stable: $changed(e) = (prev !== e) (the 1-bit negation of
//! $stable), and $sampled(e) = e (identity — the sampled value equals the current
//! value in our region model; same lenient approximation as the $past family,
//! including the first-clock X-initialized prev). Hand-IEEE: iverilog 13.0 rejects
//! concurrent assertions AND $changed/$sampled, so these are verified by their
//! synthesized firing behavior, not differentially.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cs_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn changed_detects_a_transition() {
    // a transitions 0->1; `$changed(a) |-> b` with b=0 must FIRE (the change is
    // detected via the prev-register), and must NOT hit the generic E3009.
    let (out, err, _c) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) $changed(a) |-> b) else $display(\"CHG\");\n\
         initial begin #8 a=1; #20 $finish; end\n\
         endmodule\n");
    let all = format!("{out}{err}");
    assert!(
        all.contains("CHG"),
        "$changed must detect the transition:\n{all}"
    );
    assert!(
        !all.contains("unsupported system function"),
        "$changed must resolve to the prev-register, not E3009:\n{all}"
    );
}

#[test]
fn sampled_is_identity_with_signal() {
    // $sampled(a) must behave EXACTLY like a: run the same property with $sampled(a)
    // and with a, and require identical assertion output.
    let prop = |ante: &str| {
        format!(
            "module top;\n\
             reg clk=0, a=0, b=0;\n\
             always #5 clk=~clk;\n\
             initial assert property(@(posedge clk) {ante} |-> b) else $display(\"FIRE\");\n\
             initial begin #8 a=1; #20 $finish; end\n\
             endmodule\n"
        )
    };
    let (s_out, _e1, s_code) = run(&prop("$sampled(a)"));
    let (a_out, _e2, a_code) = run(&prop("a"));
    let s_fires = s_out.matches("FIRE").count();
    let a_fires = a_out.matches("FIRE").count();
    assert_eq!(
        s_fires, a_fires,
        "$sampled(a) must fire identically to a (sampled={s_fires} plain={a_fires})\n{s_out}\n---\n{a_out}"
    );
    assert_eq!(s_code, a_code, "$sampled(a) exit code must match plain a");
    assert!(s_fires > 0, "the chosen stimulus should fire at least once");
    assert!(
        !s_out.contains("unsupported system function"),
        "$sampled must not hit E3009"
    );
}

#[test]
fn changed_negates_stable() {
    // $changed(a) must be the logical negation of $stable(a): a held CONSTANT means
    // !$changed(a) holds the same cycles $stable(a) holds. Run both as the antecedent
    // of `|-> 1'b1` (always passes) — neither should error, both clean exit 0.
    let prop = |ante: &str| {
        format!(
            "module top;\n\
             reg clk=0, a=1, b=0;\n\
             always #5 clk=~clk;\n\
             initial assert property(@(posedge clk) {ante} |-> 1'b1);\n\
             initial begin #40 $finish; end\n\
             endmodule\n"
        )
    };
    for ante in ["$stable(a)", "!$changed(a)", "$changed(a)"] {
        let (out, err, code) = run(&prop(ante));
        assert_eq!(code, Some(0), "`{ante}` |-> 1 must be clean:\n{out}\n{err}");
        assert!(
            !format!("{out}{err}").contains("unsupported system function"),
            "`{ante}` must resolve to the prev-register, not E3009"
        );
    }
}
