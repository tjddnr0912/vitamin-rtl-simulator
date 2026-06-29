//! `string s = expr;` declaration initializer (IEEE §6.16 / §6.8). vita rejected
//! it ("a string declaration initializer is outside the v7 scope"); it now
//! registers the string net and emits the initializer as a one-time t0
//! assignment (a synthesized pre-sweep `initial s = expr;`, collected in
//! declaration order with the other variable initializers so it runs before user
//! `initial` blocks and sees earlier initializers). Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strinit_{}_{n}", std::process::id()));
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

#[test]
fn string_decl_init_basic() {
    let (out, code) = run("module top; string s=\"Hello\"; \
         initial begin $display(\"s=%s\",s); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("s=Hello"), "string decl init; got:\n{out}");
}

#[test]
fn string_decl_init_then_modify() {
    // The initialized string is a normal variable afterwards.
    let (out, _c) = run(
        "module top; string s=\"abc\"; \
         initial begin $display(\"s1=%s\",s); s={s,\"d\"}; $display(\"s2=%s\",s); #1 $finish; end endmodule\n",
    );
    assert!(
        out.contains("s1=abc") && out.contains("s2=abcd"),
        "init then modify; got:\n{out}"
    );
}

#[test]
fn multi_string_decl_init() {
    let (out, _c) = run("module top; string a=\"x\", b=\"y\"; \
         initial begin $display(\"%s%s\",a,b); #1 $finish; end endmodule\n");
    assert!(out.contains("xy"), "multi string init; got:\n{out}");
}

#[test]
fn empty_string_decl_init() {
    let (out, _c) = run("module top; string s=\"\"; \
         initial begin $display(\"len=%0d\",s.len()); #1 $finish; end endmodule\n");
    assert!(out.contains("len=0"), "empty string init; got:\n{out}");
}

#[test]
fn string_init_declaration_order() {
    // `b`'s initializer reads `a`, declared earlier — the synthesized t0 assigns
    // must run in declaration order (a before b), not string-first.
    let (out, _c) = run("module top; string a=\"x\"; string b={a,\"y\"}; \
         initial begin #1 $display(\"a=%s b=%s\",a,b); $finish; end endmodule\n");
    assert!(
        out.contains("a=x b=xy"),
        "declaration-order init; got:\n{out}"
    );
}

#[test]
fn string_init_runs_before_user_initial() {
    // The pre-sweep `initial` runs before user blocks, so a user `initial` reads
    // the initialized value, not the empty default.
    let (out, _c) = run("module top; string s=\"init\"; initial $display(\"u=%s\",s); endmodule\n");
    assert!(out.contains("u=init"), "pre-sweep ordering; got:\n{out}");
}

#[test]
fn string_init_is_one_time_not_continuous() {
    // The initializer is a ONE-TIME t0 assignment, NOT a continuous driver: a
    // later procedural write to the source must NOT flow back. An adversarial
    // review caught the first cut spuriously creating `assign s = init`, so a
    // reassignment of the source was undone. `b={a,"y"}` is evaluated once.
    let (out, _c) = run("module top; string a=\"x\"; string b={a,\"y\"}; \
         initial begin #1 a=\"ZZZ\"; #1 $display(\"a=%s b=%s\",a,b); $finish; end endmodule\n");
    assert!(
        out.contains("a=ZZZ b=xy"),
        "one-time init, no continuous track; got:\n{out}"
    );
}

#[test]
fn block_local_string_init_works() {
    // A string initializer inside a procedural block-local is now supported: it
    // rides the same t0 pre-sweep `pending_var_inits` path as a block-local int
    // init (STATIC-lifetime, applied once at time 0). (It was loud in §4.5.31; a
    // follow-on slice added the block-local collection.)
    let (out, code) = run("module top; \
         initial begin string s=\"hi\"; #1 $display(\"s=%s\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("s=hi"), "block-local string init; got:\n{out}");
}

#[test]
fn block_local_string_init_then_modify() {
    // The block-local string is a normal variable after its one-time init.
    let (out, _c) = run("module top; \
         initial begin string s=\"abc\"; s={s,\"d\"}; #1 $display(\"s=%s\",s); $finish; end endmodule\n");
    assert!(
        out.contains("s=abcd"),
        "block-local init then modify; got:\n{out}"
    );
}

#[test]
fn block_local_string_init_reads_module_string() {
    // A block-local string initializer that reads a MODULE-scope string must see
    // the module string's initialized value, not its empty default. An adversarial
    // review caught the first cut ordering the block-local assignment before the
    // module one in the shared t0 pre-sweep (b came out empty); block-local string
    // inits are now deferred to run after the module-scope inits.
    let (out, _c) = run("module top; string m=\"HELLO\"; \
         initial begin string b=m; #1 $display(\"b=%s m=%s\",b,m); $finish; end endmodule\n");
    assert!(
        out.contains("b=HELLO m=HELLO"),
        "block reads module string; got:\n{out}"
    );
}

#[test]
fn coalescing_block_local_string_is_loud() {
    // Two sibling blocks declaring the same-named string coalesce to one
    // module-flattened net with no per-block heap — a shared-heap leak — so it
    // stays loud (correct-or-loud), unaffected by enabling block-local string init.
    let (_o, code) = run("module top; initial begin \
         begin string s=\"a\"; end begin string s=\"b\"; #1 $display(\"%s\",s); end \
         $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "coalescing block-local string must stay loud"
    );
}

#[test]
fn dimensioned_string_init_is_loud() {
    // A string with packed/unpacked dims is unsupported (v7) — it must stay loud,
    // not silently accept the initializer.
    let (_o, code) = run("module top; string s[2]=\"x\"; initial $finish; endmodule\n");
    assert_ne!(code, Some(0), "dimensioned string init must be loud");
}
