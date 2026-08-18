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
fn an_output_string_formal_stays_loud_and_still_names_the_way_out() {
    // NARROWED, not deleted. The copy-OUT resolver takes a simple-net caller lvalue and
    // a `string` actual is not one, so this direction genuinely does not work here —
    // and `automatic` does (measured). Deleting the arm outright traded one actionable
    // message for a vaguer rejection plus a spurious E3010 cascade, because the formal
    // never gets bound and the body's reads then go unresolved.
    for dir in ["output", "inout"] {
        let out = run(&format!(
            "module tb; string r = \"in\";\n\
             \x20 task t({dir} string s); s = \"out\"; endtask\n\
             \x20 initial begin t(r); $display(\"s=%s\", r); end endmodule\n"
        ));
        assert!(out.contains("E3009"), "{dir} stays loud:\n{out}");
        // The message must name the direction it is actually about. Reporting "output"
        // for an `inout` sends the reader to the wrong port — the §3.4 defect this
        // round already paid for once.
        assert!(
            out.contains(&format!("`string` {dir} formal")),
            "the message must name the {dir} direction:\n{out}"
        );
        assert!(
            out.contains("automatic"),
            "…and must still name the way out that works:\n{out}"
        );
        assert!(
            out.contains("INPUT `string` formal is supported"),
            "…and must say the direction that now works, so the reader does not \
             rewrite an input too:\n{out}"
        );
        assert!(
            !out.contains("E3010"),
            "…and must not cascade into an unresolved-name error:\n{out}"
        );
    }
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
