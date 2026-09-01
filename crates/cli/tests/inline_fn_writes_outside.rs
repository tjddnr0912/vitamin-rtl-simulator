//! An inlined function body that assigned a MODULE NET dropped the write silently.
//!
//! `fold_straight_line` reduces a straight-line function body by substitution: each
//! `local = expr;` becomes a binding and `fname = expr;` records the return value. It
//! decided which of the two a statement was from the LHS spelling alone, so a write to
//! a name the function does not own — a module net — was pushed onto the substitution
//! stack, nothing ever wrote the net, and the call returned the right value with the
//! net unchanged at exit 0:
//!
//! ```text
//! logic [7:0] seq = 0;
//! function logic [3:0] f(); seq = seq + 7; f = 4'h3; endfunction
//! a = f();          // vita: a=3 seq=0   ·   iverilog and verilator: a=3 seq=7
//! ```
//!
//! ⭐ The `automatic` spelling of the SAME body was already honest: it takes the frame
//! path, whose subset check says *"uses an assignment to a net outside the function"*.
//! So one rule had two answers depending on a keyword, and the static half was the one
//! below the ladder. ROADMAP §2 row 6.
//!
//! ⚠️ This raises the static spelling to LOUD; it does not implement the write.
//! Performing it means emitting a statement from an expression-position fold, which is
//! the hoisting machinery — recorded in ROADMAP §2, not attempted here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inlw_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

const OUTSIDE: &str = "module t;\n  \
       logic [7:0] seq = 0;\n  logic [3:0] a;\n  \
       function logic [3:0] f(); seq = seq + 7; f = 4'h3; endfunction\n  \
       initial begin a = f(); $display(\"a=%0d seq=%0d\", a, seq); #1 $finish; end\n\
     endmodule\n";

/// It must be LOUD, and the message must name the real cause — not "control flow",
/// which this body does not have.
#[test]
fn a_static_function_writing_a_module_net_is_loud_and_says_why() {
    let (o, e, ok) = run(OUTSIDE);
    assert!(!ok, "must not run:\n{o}{e}");
    let all = format!("{o}{e}");
    assert!(all.contains("assigns `seq`"), "names the target:\n{all}");
    assert!(
        all.contains("not one of its own formals or locals"),
        "names the cause:\n{all}"
    );
    assert!(
        !all.contains("control flow"),
        "and NOT the wrong cause — this body has none:\n{all}"
    );
}

/// The `automatic` twin was already loud and still is. Both spellings of one body now
/// give one answer.
#[test]
fn the_automatic_twin_is_loud_too() {
    let (o, e, ok) = run(&OUTSIDE.replace("function logic", "function automatic logic"));
    assert!(!ok, "must not run:\n{o}{e}");
    assert!(
        format!("{o}{e}").contains("outside the frame-call subset"),
        "{o}{e}"
    );
}

/// CONTROL: a body that touches only its own formals, its own locals and a
/// `begin/end` block-local still inlines and still folds. `local_dims` could not have
/// served as the ownership set — it deliberately omits `string`/`class`/`event`
/// locals — so this cell also covers a `string` local, whose absence from that map
/// must not be read as "not mine".
#[test]
fn a_body_that_writes_only_its_own_names_still_inlines() {
    let (o, e, ok) = run("module t;\n  \
           logic [7:0] a;\n  \
           function logic [7:0] g(input logic [7:0] x);\n    \
             logic [7:0] tmp; string s;\n    \
             s = \"hi\"; tmp = x + 8'd1;\n    \
             begin logic [7:0] u; u = tmp * 8'd2; g = u; end\n  \
           endfunction\n  \
           initial begin a = g(8'd3); $display(\"a=%0d\", a); #1 $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}{e}");
    // (3+1)*2 = 8. iverilog-pinned.
    assert!(o.contains("a=8"), "got:\n{o}");
}
