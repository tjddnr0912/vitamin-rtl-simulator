//! An INPUT `string` formal in a STATIC task.
//!
//! External aes_top round-3 §3.7. `task t(input string s);` was
//! `E3009 … (declare the task automatic)`. The reporter called the message very good
//! — it names a working way out — and the feature simply did not exist.
//!
//! The inline (static-lifetime) path binds each formal to a formal-width local net,
//! and that local was allocated with `map_net_kind_or_wire`, which sends `string` to a
//! 1-bit `Wire`. So the copy-in truncated the actual to one bit. The FRAME path can
//! leave the slot 1-bit because it has a `FuncMeta` and the engine reads the
//! `str_params` mask to learn the slot holds a heap handle; an inlined task has no
//! `FuncMeta`, so nothing carries that fact. The fix is one net kind:
//! `frame_local_net_kind`, which is what a frame-local `string` already uses.
//!
//! ORACLE: iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stsf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn an_input_string_formal_carries_the_whole_string() {
    for (src, want) in [
        // a literal actual
        (
            "module tb; task t(input string s); $display(\"s=%s\", s); endtask\n\
             \x20 initial t(\"hello\"); endmodule\n",
            "s=hello",
        ),
        // a string VARIABLE actual
        (
            "module tb; string g = \"world\";\n\
             \x20 task t(input string s); $display(\"s=%s\", s); endtask\n\
             \x20 initial t(g); endmodule\n",
            "s=world",
        ),
        // beside a non-string formal — the other kinds must be unchanged
        (
            "module tb; task t(input string s, input int n); $display(\"s=%s%0d\", s, n); endtask\n\
             \x20 initial t(\"x\", 7); endmodule\n",
            "s=x7",
        ),
        // a string METHOD in the body reads the formal as a real string
        (
            "module tb; task t(input string s); $display(\"s=%0d\", s.len()); endtask\n\
             \x20 initial t(\"abcd\"); endmodule\n",
            "s=4",
        ),
    ] {
        let out = run(src);
        assert!(out.contains(want), "expected {want} (iverilog's value):\n{out}");
        assert!(!out.contains("E3009"), "must not be loud:\n{out}");
    }
}

#[test]
fn two_calls_with_different_lengths_each_carry_their_own() {
    // DISCRIMINATOR for the truncation this fixes AND for the static-storage reuse:
    // the formal-local is allocated ONCE per task name (§13.4.1 — a static task's
    // formals are a single instance) and reused at every call site, so a shorter
    // second string must not leave the first one's tail behind.
    let out = run(
        "module tb; task t(input string s); $display(\"s=%s\", s); endtask\n\
         \x20 initial begin t(\"aa\"); t(\"bbbb\"); t(\"c\"); end endmodule\n",
    );
    for want in ["s=aa", "s=bbbb", "s=c"] {
        assert!(out.contains(want), "expected {want}:\n{out}");
    }
}

#[test]
fn an_output_string_formal_now_works_and_the_survivors_still_name_their_reason() {
    // ⚠️ This test asserted the WORDING of a refusal. Round 34 (R6) removed the
    // refusal: the gate decided from the FORMAL alone, and every piece of copy-out
    // machinery it claimed was missing was already present in the Output/Inout arm.
    // It is NARROWED now, at the point where the ACTUAL is known — so a wording
    // assertion here became a VALUE assertion, which is strictly stronger.
    //
    // Values are LIVE iverilog 13.0's.
    for dir in ["output", "inout"] {
        let out = run(&format!(
            "module tb; string r = \"in\";\n\
             \x20 task t({dir} string s); s = \"out\"; endtask\n\
             \x20 initial begin t(r); $display(\"s=%s\", r); end endmodule\n"
        ));
        assert!(
            out.contains("s=out"),
            "a bare string variable actual is copied out ({dir}):\n{out}"
        );
        assert!(!out.contains("E3009"), "{dir} must not be loud:\n{out}");
    }

    // ⚠️ The narrowing's other half — what STAYS loud, and why. A select is not a
    // whole string variable, so its copy-out target has no representation; the
    // message has to say THAT rather than repeat the old blanket sentence.
    let out = run("module tb; string r = \"in\"; logic [7:0] v;\n\
         \x20 task t(output string s); s = \"out\"; endtask\n\
         \x20 initial begin t(v); $display(\"v=%02h\", v); end endmodule\n");
    assert!(
        out.contains("E3009"),
        "a NON-string actual must stay loud:\n{out}"
    );
    assert!(
        out.contains("string"),
        "…and the message must still be about the string formal:\n{out}"
    );
}

#[test]
fn the_automatic_spelling_is_unchanged() {
    // The frame path was always correct here; this slice must not have moved it.
    let out = run(
        "module tb; task automatic t(input string s); $display(\"s=%s\", s); endtask\n\
         \x20 initial t(\"auto\"); endmodule\n",
    );
    assert!(out.contains("s=auto"), "frame path unchanged:\n{out}");
}
