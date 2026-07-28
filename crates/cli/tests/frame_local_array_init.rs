//! R16-X1: a fixed-size `string` array with a `'{…}` declaration initializer inside a
//! FRAME body (a function/task, or a `begin…end` within one).
//!
//! A fixed string array is stored dyn-backed, so its `'{…}` initializer lowers to
//! `f = new[N]` followed by element writes. The guard that keeps a USER `new[n]` from
//! resizing a fixed array read that synthesized allocation as a resize and rejected
//! the declaration — but only on the frame path, because the module-scope t0 var-init
//! flush marks its own lowering with `lowering_decl_init` and the frame path marked
//! only its pre-size, not the initializer that follows.
//!
//! Found while sweeping §3.1, and pre-existing at 6b6b8ef. iverilog runs every
//! accepted case here, so these are live differential pins, not hand-IEEE ones.
//!
//! The exclusion of a user-written `new[…]` initializer is MEASURED: exempting it too
//! made `string f[3] = new[5];` inside a task print an empty element at exit 0 where
//! 6b6b8ef rejected it (and where iverilog aborts on an internal assertion). Only an
//! allocation the lowering synthesizes earns the exemption.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_flai_{}_{n}", std::process::id()));
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

/// The declaration sits directly in the task body. iverilog prints `R b`.
#[test]
fn task_top_level_string_array_decl_init() {
    runs(
        r#"module t;
             task automatic run;
               string f [3] = '{"a", "b", "c"};
               $display("R %s", f[1]);
             endtask
             initial run();
           endmodule"#,
        &["R b"],
    );
}

/// The declaration sits in a nested `begin…end` inside the task body. iverilog
/// prints `R b`.
#[test]
fn nested_block_string_array_decl_init() {
    runs(
        r#"module t;
             task automatic run;
               begin
                 string f [3] = '{"a", "b", "c"};
                 $display("R %s", f[1]);
               end
             endtask
             initial run();
           endmodule"#,
        &["R b"],
    );
}

/// A function body, and a sibling dynamic array allocated in the same block — the
/// shape that first exposed this, where the diagnostic named `f` while the user was
/// looking at `md`. iverilog prints `R 2 a`.
#[test]
fn function_body_with_sibling_dynamic_array() {
    runs(
        r#"module t;
             function automatic int run;
               begin
                 string f [3] = '{"a", "skip", "c"};
                 byte   md [];
                 md = new[2];
                 $display("R %0d %s", md.size(), f[0]);
                 return 0;
               end
             endfunction
             int z;
             initial z = run();
           endmodule"#,
        &["R 2 a"],
    );
}

/// The module-scope twin, which already worked — pinned so the frame fix is not
/// mistaken for the thing that made this pass. iverilog prints `R b`.
#[test]
fn module_scope_block_still_works() {
    runs(
        r#"module t;
             initial begin
               begin
                 string f [3] = '{"a", "b", "c"};
                 $display("R %s", f[1]);
               end
             end
           endmodule"#,
        &["R b"],
    );
}

/// SOUNDNESS PIN. A legal dynamic-array `new[n]` initializer in a frame is still
/// allocated, not swallowed. iverilog prints `R 4 7`.
#[test]
fn frame_local_dynamic_array_new_still_allocates() {
    runs(
        r#"module t;
             task automatic run;
               byte b [] = new[4];
               b[3] = 8'h7;
               $display("R %0d %0d", b.size(), b[3]);
             endtask
             initial run();
           endmodule"#,
        &["R 4 7"],
    );
}

/// SOUNDNESS PIN. A user-written `new[n]` on a FIXED string array must stay loud —
/// this is the shape the narrowed exemption exists to protect.
#[test]
fn frame_local_user_new_on_fixed_string_array_stays_loud() {
    let (o, ok) = run(r#"module t;
             task automatic run;
               string f [3] = new[5];
               $display("R %s", f[1]);
             endtask
             initial run();
           endmodule"#);
    assert!(!ok, "expected a diagnostic, got acceptance:\n{o}");
    assert!(o.contains("E3009"), "expected E3009, got:\n{o}");
    assert!(
        o.contains("cannot be resized"),
        "expected the resize reject, got:\n{o}"
    );
}
