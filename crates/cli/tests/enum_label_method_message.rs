//! V33-3: **the rejection named a different construct.** `pk::LA.name()` — an enum
//! method on a package-scoped enum LABEL — was refused with *"a chained method call is
//! supported only on a string-returning method result (e.g. `s.substr(a,b).atoi()`)"*,
//! and there is no method before `.name()` in that source at all: `pk::LA` is a
//! scope-resolved NAME (§26.3) that folds to a constant, so it fell into the arm meant
//! for `s.substr(a,b).atoi()`. The bare spelling `LA.name()` had the twin defect from
//! the other end: it reached the deferred hierarchical-call resolver, whose enum-method
//! receiver test asked `symbols` (NETS ONLY), so a label — never a net — could not match
//! it and got *"unsupported hierarchical function call"*. The receiver test now also
//! accepts a bound enum label, and the label case has its own message; the second arm's
//! *"the enum type of `X` was not registered"* sentence is FALSE for a label (the enum IS
//! registered — that is why `mv.name()` works on a variable of the type).
//!
//! **It stays LOUD, measured against both oracles.** iverilog 13.0 aborts on every
//! label spelling (`elab_expr.cc:3297: failed assertion sub_expr`) and verilator 5.050
//! errors "Can't find definition of task/function: 'name'" — for `pk::LA.name()`,
//! `pk::LB.next()`, module-scope `LA.name()` and wildcard-imported `LA.name()` alike.
//! There is no oracle answer to implement, so only the wording changed.
//!
//! The value assertions below are the other half: the neighbouring spellings that DO
//! work are pinned to LIVE iverilog 13.0 output (`v=LB r=1 num=2`, `5`, `3`), so a
//! future attempt to widen this cell cannot quietly break the cells around it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_elmm_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

const PKG: &str = "package pk; typedef enum logic [7:0] { LA = 8'h1, LB = 8'h2 } e_t; \
                   endpackage\n";

/// The reported cell. The message must name the label and the enum method, and must NOT
/// mention chained string methods — `substr`/`atoi` in this output means the author is
/// being sent to the wrong feature again.
#[test]
fn a_package_scoped_label_method_is_described_as_an_enum_label_not_a_chained_method() {
    let (out, code) = run(&format!(
        "{PKG}module tb; initial begin $display(\"%s\", pk::LA.name()); end endmodule\n"
    ));
    assert_ne!(
        code,
        Some(0),
        "both oracles reject it; vita must too:\n{out}"
    );
    assert!(
        out.contains("`pk::LA.name()`"),
        "the message must quote what the author wrote:\n{out}"
    );
    assert!(
        out.contains("enum LABEL `LA`"),
        "…and say the receiver is a label:\n{out}"
    );
    assert!(
        !out.contains("substr") && !out.contains("atoi"),
        "the chained-string-method wording describes a different construct:\n{out}"
    );
    assert!(
        !out.contains("hierarchical"),
        "and so does the hierarchical-call wording:\n{out}"
    );
    // The enum's REAL type name, not an invented one — the message tells the author to
    // declare a variable of it, so a made-up `e_t` would be a second wrong instruction.
    assert!(
        out.contains("of `pk::e_t`"),
        "the message must name the enum type that declared the label:\n{out}"
    );
    // What DOES work has to be in the message, or the author has nowhere to go.
    assert!(
        out.contains("pk::e_t v = pk::LA; v.name()"),
        "the message must show the spelling that works:\n{out}"
    );
}

/// ⭐ The message is only worth its words if the spelling it prescribes RUNS. Both
/// suggestions — the package-qualified type and a module-local one — are executed here
/// and pinned to live iverilog 13.0 (`MA LA`).
#[test]
fn the_spelling_the_message_prescribes_actually_runs() {
    let (out, code) = run(&format!(
        "{PKG}module tb;\n  \
         typedef enum logic [7:0] {{ MA = 8'h5, MB = 8'h6 }} m_t;\n  \
         m_t mv = MA;\n  pk::e_t pv = pk::LA;\n  \
         initial begin $display(\"%s %s\", mv.name(), pv.name()); end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("MA LA"),
        "live iverilog prints `MA LA`:\n{out}"
    );
}

/// `.next()` takes the same arm — the defect was per-RECEIVER, not per-method.
#[test]
fn every_enum_method_on_a_package_label_gets_the_label_message() {
    let (out, code) = run(&format!(
        "{PKG}module tb; initial begin $display(\"%0d\", pk::LB.next()); end endmodule\n"
    ));
    assert_ne!(code, Some(0), "{out}");
    assert!(out.contains("`pk::LB.next()`"), "{out}");
    assert!(out.contains("enum LABEL `LB`"), "{out}");
    assert!(!out.contains("substr"), "{out}");
}

/// The bare spellings — a module-body label and a wildcard-imported one — reach the
/// OTHER site (the deferred hierarchical-call resolver). Both used to be described as
/// an unsupported hierarchical function call.
#[test]
fn a_bare_label_method_is_not_called_a_hierarchical_function_call() {
    for src in [
        "module tb;\n  typedef enum logic [7:0] { LA = 8'h1, LB = 8'h2 } e_t;\n  \
         initial begin $display(\"%s\", LA.name()); end\nendmodule\n"
            .to_string(),
        format!(
            "{PKG}module tb;\n  import pk::*;\n  \
             initial begin $display(\"%s\", LA.name()); end\nendmodule\n"
        ),
    ] {
        let (out, code) = run(&src);
        assert_ne!(code, Some(0), "{out}");
        assert!(out.contains("`LA.name()`"), "{out}");
        assert!(out.contains("enum LABEL `LA`"), "{out}");
        assert!(out.contains("e_t v = LA; v.name()"), "{out}");
        assert!(
            !out.contains("hierarchical"),
            "the hierarchical-call wording describes a different construct:\n{out}"
        );
        // FALSE for a label: the enum IS registered — `v.name()` on a variable works.
        assert!(
            !out.contains("was not registered"),
            "a label's enum type IS registered:\n{out}"
        );
    }
}

/// ⭐ The three arms this receiver test sits between must keep their own messages. A
/// genuine hierarchical call to a non-framed function, a variable whose enum type
/// really was not registered (a `parameter`-valued label, §4.5.379), and a real chained
/// method on a non-string result are each a different diagnosis.
#[test]
fn the_neighbouring_rejections_keep_their_own_wording() {
    let (out, code) = run(
        "module sub;\n  function automatic int f(input int a); return a+1; endfunction\n\
         endmodule\nmodule tb;\n  sub u1();\n  \
         initial begin $display(\"%0d\", u1.name(3)); end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    assert!(
        out.contains("unsupported hierarchical function call `u1.name`"),
        "a real hierarchical call keeps the hierarchical wording:\n{out}"
    );

    let (out, code) = run(
        "module tb #(parameter int K = 9);\n  typedef enum int { A = K, B } e_t;\n  \
         e_t v;\n  initial begin v = A; $display(\"%s\", v.name()); end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    assert!(
        out.contains("was not registered"),
        "an unregistered enum TYPE on a NET keeps its own message:\n{out}"
    );

    let (out, code) = run("module tb;\n  string s = \"42\";\n  \
         initial begin $display(\"%0d\", s.atoi().len()); end\nendmodule\n");
    assert_ne!(code, Some(0), "{out}");
    assert!(
        out.contains("a chained method call is supported only on a string-returning"),
        "an actual chained method keeps the chained wording:\n{out}"
    );
}

/// The cells AROUND the residue, pinned to live iverilog 13.0 (`v=LB r=1 num=2`).
/// These bound the defect to one spelling: reading the label as a VALUE, and calling
/// enum methods on a VARIABLE of the type, both work through the same package.
#[test]
fn the_working_neighbours_still_answer_with_the_oracle_values() {
    let (out, code) = run(&format!(
        "{PKG}module tb;\n  import pk::*;\n  e_t v;\n  logic [7:0] r;\n  \
         initial begin\n    v = LB;\n    r = pk::LA;\n    \
         $display(\"v=%s r=%0d num=%0d\", v.name(), r, v.num());\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("v=LB r=1 num=2"),
        "live iverilog prints this:\n{out}"
    );
}

/// `pk::S.len()` on a package STRING parameter and `s.len()` on a local string are the
/// two method-on-a-name spellings that already worked; iverilog prints 5 and 3.
#[test]
fn a_package_string_parameter_and_a_local_string_keep_their_methods() {
    let (out, code) = run("package pk; parameter string S = \"hello\"; endpackage\n\
         module tb;\n  string s = \"abc\";\n  \
         initial begin $display(\"%0d %0d\", pk::S.len(), s.len()); end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("5 3"), "live iverilog prints `5 3`:\n{out}");
}
