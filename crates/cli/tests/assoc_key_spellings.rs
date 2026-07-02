//! Associative-array key-type spellings (IEEE 1800 §7.8) — `[int]`,
//! `[longint]`, `[shortint]`, `[byte]` join the existing `[integer]`/`[time]`/
//! `[string]`. Every integral spelling shares vitamin's documented signed-i64
//! key domain (the v5-⑥ design pin: keys are NOT truncated to the declared
//! width — `[integer]`/`[time]` already behave this way; Icarus rejects assoc
//! declarations entirely, so this whole family is hand-IEEE). The wildcard
//! `[*]` stays a loud reject with the updated spelling list.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ak_{}_{n}", std::process::id()));
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
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn all_integral_key_spellings_share_the_i64_domain() {
    for key in ["int", "longint", "shortint", "byte", "integer", "time"] {
        let (out, _, code) = run(&format!(
            "module t; integer aa[{key}]; integer k; initial begin\n\
               aa[3] = 7; aa[-2] = 9;\n\
               $display(\"v=%0d neg=%0d n=%0d ex=%0d\", aa[3], aa[-2], aa.num(), aa.exists(3));\n\
               aa.delete(3);\n\
               $display(\"del n=%0d ex=%0d\", aa.num(), aa.exists(3));\n\
               $finish; end endmodule\n"
        ));
        assert_eq!(code, 0, "[{key}] must elaborate");
        assert!(
            out.contains("v=7 neg=9 n=2 ex=1"),
            "[{key}] rw/negative/num/exists:\n{out}"
        );
        assert!(out.contains("del n=1 ex=0"), "[{key}] delete:\n{out}");
    }
}

#[test]
fn foreach_and_first_next_walk_int_keyed() {
    // (`aa.first(k)` stays a direct-rhs special — the `if (aa.first(k))`
    // condition form is a separate pre-existing loud, spelling-independent.)
    let (out, _, code) = run(
        "module t; integer aa[int]; integer k; integer st; integer s; initial begin\n\
           aa[5] = 50; aa[1] = 10; aa[9] = 90; s = 0;\n\
           foreach (aa[k2]) s += aa[k2];\n\
           $display(\"sum=%0d\", s);\n\
           st = aa.first(k);\n\
           $display(\"first st=%0d k=%0d\", st, k);\n\
           $finish; end endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(out.contains("sum=150"), "foreach over [int]:\n{out}");
    assert!(out.contains("first st=1 k=1"), "key order walk:\n{out}");
}

#[test]
fn whole_copy_across_spellings_of_the_same_domain() {
    // `[int]` and `[integer]` lower to the SAME Assoc kind + i64 domain, so
    // the §7.9 whole-copy applies between them (deep copy).
    let (out, _, code) = run(
        "module t; integer aa[int]; integer bb[integer]; initial begin\n\
           aa[5] = 50; aa[7] = 70;\n\
           bb = aa;\n\
           aa[5] = 99;\n\
           $display(\"bb5=%0d bbn=%0d aa5=%0d\", bb[5], bb.num(), aa[5]);\n\
           $finish; end endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(
        out.contains("bb5=50 bbn=2 aa5=99"),
        "cross-spelling copy:\n{out}"
    );
}

#[test]
fn wildcard_key_stays_loud_with_updated_message() {
    let (_, err, code) = run("module t; integer aa[*]; initial begin aa[1]=1; end endmodule\n");
    assert_ne!(code, 0);
    assert!(
        err.contains("concrete assoc key type") && err.contains("[int]"),
        "wildcard loud + spelling list:\n{err}"
    );
}
