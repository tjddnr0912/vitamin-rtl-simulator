//! N7 class member access control — `local` / `protected` (IEEE §8.18, G5).
//!
//! A `local` member is reachable ONLY from the declaring class's own methods; a
//! `protected` member from the declaring class AND its descendants; everything
//! else (no qualifier) is public. vita enforces this strictly (correct-or-loud):
//! an out-of-scope read/write/call of a `local`/`protected` member is a LOUD
//! E3009, never a silent read of inaccessible storage.
//!
//! Oracle = iverilog 13.0. The IN-SCOPE accesses (in-class, derived-protected)
//! match iverilog byte-for-byte. The REJECTIONS partially diverge from iverilog
//! 13.0, which UNDER-enforces access control: empirically iverilog rejects only
//! an out-of-class `local` PROPERTY read/write from a NON-method scope, and does
//! NOT reject (a) `local` from a derived class, (b) a `local` METHOD call, or
//! (c) any `protected` access from outside. vita follows IEEE §8.18 (the LRM is
//! unambiguous: local is invisible to derived classes; protected is invisible
//! outside the family) — these stricter rejections are a deliberate HAND-IEEE
//! PIN, the same posture as the documented iverilog virtual-dispatch divergence.
//! Do NOT relax vita to the more permissive (IEEE-illegal) oracle behavior.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_g5_{}_{n}", std::process::id()));
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

// ── In-scope access: must WORK and read the right value (iverilog parity) ──────

#[test]
fn in_class_local_read_via_own_method() {
    // iverilog parity: `c.set(49)` writes a `local` field in-class, `c.get()`
    // reads it in-class → 49.
    let (out, code) = run("class C;\n\
        local int x;\n\
        function void set(int v); x = v; endfunction\n\
        function int get(); return x; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.set(49); $display(\"get=%0d\", c.get()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "in-class local must elaborate:\n{out}");
    assert!(out.contains("get=49"), "in-class local read:\n{out}");
}

#[test]
fn in_class_protected_read_via_own_method() {
    let (out, code) = run("class C;\n\
        protected int p;\n\
        function void set(int v); p = v; endfunction\n\
        function int get(); return p; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.set(33); $display(\"get=%0d\", c.get()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "in-class protected must elaborate:\n{out}");
    assert!(out.contains("get=33"), "in-class protected read:\n{out}");
}

#[test]
fn derived_reads_inherited_protected_field() {
    // iverilog parity: a DERIVED method reaches a base `protected` field → 77.
    let (out, code) = run("class Base;\n\
        protected int p;\n\
        function void setp(int v); p = v; endfunction\n\
        endclass\n\
        class Der extends Base;\n\
        function int getp(); return p; endfunction\n\
        endclass\n\
        module t; Der d;\n\
        initial begin d = new; d.setp(77); $display(\"getp=%0d\", d.getp()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "derived protected must elaborate:\n{out}");
    assert!(out.contains("getp=77"), "derived reads protected:\n{out}");
}

#[test]
fn deep_chain_protected_reachable_in_derived() {
    // protected on a GRANDPARENT is still reachable from the most-derived method.
    let (out, code) = run("class GrandBase;\n\
        protected int g;\n\
        function void setg(int v); g = v; endfunction\n\
        endclass\n\
        class Base extends GrandBase; endclass\n\
        class Der extends Base;\n\
        function int getg(); return g; endfunction\n\
        endclass\n\
        module t; Der d;\n\
        initial begin d = new; d.setg(42); $display(\"g=%0d\", d.getg()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "deep protected must elaborate:\n{out}");
    assert!(out.contains("g=42"), "deep-chain protected:\n{out}");
}

#[test]
fn protected_method_called_from_derived() {
    // A `protected` method is callable from a derived method (IEEE §8.18). Also a
    // bare inherited-method call — vita resolves it (iverilog 13 cannot, an
    // unrelated inheritance-resolution gap), so this is hand-IEEE.
    let (out, code) = run("class Base;\n\
        protected function int helper(); return 9; endfunction\n\
        endclass\n\
        class Der extends Base;\n\
        function int run(); return helper() + 1; endfunction\n\
        endclass\n\
        module t; Der d;\n\
        initial begin d = new; $display(\"run=%0d\", d.run()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "protected method must elaborate:\n{out}");
    assert!(
        out.contains("run=10"),
        "protected method from derived:\n{out}"
    );
}

#[test]
fn local_method_called_in_class() {
    let (out, code) = run("class C;\n\
        int v;\n\
        local function int secret(); return v * 3; endfunction\n\
        function int pub(); return secret() + 1; endfunction\n\
        function void setv(int x); v = x; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.setv(4); $display(\"pub=%0d\", c.pub()); end\n\
        endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "in-class local method must elaborate:\n{out}"
    );
    assert!(out.contains("pub=13"), "in-class local method:\n{out}");
}

#[test]
fn mixed_visibility_multiple_instances() {
    // public + local + protected in one class; two independent objects. In-class
    // access of all three works; per-instance storage is independent. (iverilog
    // parity.)
    let (out, code) = run("class C;\n\
        int pub;\n\
        local int loc;\n\
        protected int prot;\n\
        function void init(int a, int b, int d); pub = a; loc = b; prot = d; endfunction\n\
        function int sum(); return pub + loc + prot; endfunction\n\
        endclass\n\
        module t; C c1, c2;\n\
        initial begin\n\
          c1 = new; c2 = new;\n\
          c1.init(1, 2, 3); c2.init(10, 20, 30);\n\
          $display(\"pub=%0d s1=%0d s2=%0d\", c1.pub, c1.sum(), c2.sum());\n\
        end endmodule\n");
    assert_eq!(code, Some(0), "mixed visibility must elaborate:\n{out}");
    assert!(
        out.contains("pub=1 s1=6 s2=60"),
        "mixed + instances:\n{out}"
    );
}

#[test]
fn public_field_still_accessible_outside() {
    // Guard against OVER-rejection: an unqualified (public) field must remain
    // freely accessible from outside the class — the access-control check must
    // not fire on public members. (iverilog parity.)
    let (out, code) = run("class C; int pub; endclass\n\
        module t; C c;\n\
        initial begin c = new; c.pub = 7; $display(\"pub=%0d\", c.pub); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "public field must stay accessible:\n{out}");
    assert!(out.contains("pub=7"), "public out-of-class:\n{out}");
}

// ── Out-of-scope access: must be LOUD (no silent read of wrong storage) ─────────

#[test]
fn out_of_class_local_read_is_loud() {
    // iverilog 13.0 ALSO rejects this (the one case it enforces).
    let (out, code) = run("class C;\n\
        local int x;\n\
        function void set(int v); x = v; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.set(1); $display(\"x=%0d\", c.x); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "out-of-class local read must be loud:\n{out}"
    );
    assert!(
        !out.contains("x="),
        "must not read inaccessible storage:\n{out}"
    );
}

#[test]
fn out_of_class_local_write_is_loud() {
    let (out, code) = run("class C;\n\
        local int x;\n\
        function int getx(); return x; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.x = 9; $display(\"x=%0d\", c.getx()); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "out-of-class local write must be loud:\n{out}"
    );
}

#[test]
fn out_of_class_protected_read_is_loud() {
    // HAND-IEEE PIN: vita rejects (IEEE §8.18); iverilog 13.0 UNDER-enforces and
    // would print `p=0`. vita is the IEEE-correct, never-silent side.
    let (out, code) = run("class C; protected int p; endclass\n\
        module t; C c;\n\
        initial begin c = new; $display(\"p=%0d\", c.p); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "out-of-class protected read must be loud:\n{out}"
    );
    assert!(
        !out.contains("p="),
        "must not read protected storage:\n{out}"
    );
}

#[test]
fn out_of_class_protected_method_call_is_loud() {
    // HAND-IEEE PIN (iverilog 13.0 under-enforces).
    let (out, code) = run("class C;\n\
        protected function int hidden(); return 5; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; $display(\"h=%0d\", c.hidden()); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "out-of-class protected method must be loud:\n{out}"
    );
}

#[test]
fn local_from_derived_class_is_loud() {
    // HAND-IEEE PIN: a `local` member is INVISIBLE to derived classes (§8.18).
    // vita rejects; iverilog 13.0 wrongly allows it. Keep vita strict — silently
    // resolving to the base storage from a derived scope would be a soundness hole.
    let (out, code) = run("class Base;\n\
        local int x;\n\
        function void setx(int v); x = v; endfunction\n\
        endclass\n\
        class Der extends Base;\n\
        function int peek(); return x; endfunction\n\
        endclass\n\
        module t; Der d;\n\
        initial begin d = new; d.setx(5); $display(\"peek=%0d\", d.peek()); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "local-from-derived must be loud:\n{out}"
    );
    assert!(!out.contains("peek="), "must not read base local:\n{out}");
}

#[test]
fn local_method_from_outside_is_loud() {
    // HAND-IEEE PIN (iverilog 13.0 under-enforces local methods).
    let (out, code) = run("class C;\n\
        local function int secret(); return 7; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; $display(\"s=%0d\", c.secret()); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "local method from outside must be loud:\n{out}"
    );
}

// ── Parser-level rejections of unsupported qualifier combos ─────────────────────

#[test]
fn local_rand_combo_is_loud() {
    // `local rand` (access-controlled randomization) is outside this slice — the
    // parser rejects it loudly (never a silent drop of either qualifier).
    let (out, code) = run("class C; local rand int x; endclass\n\
        module t; initial $display(\"R\"); endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "local rand combo must be loud:\n{out}"
    );
}

#[test]
fn double_access_qualifier_is_loud() {
    // `local protected x` — a duplicate access qualifier is loud, never silently
    // taking the last.
    let (out, code) = run("class C; local protected int x; endclass\n\
        module t; initial $display(\"R\"); endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "double access qualifier must be loud:\n{out}"
    );
}

#[test]
fn static_member_still_loud() {
    // `static`/`const`/`pure`/`extern` remain deferred (loud) — this slice adds
    // ONLY local/protected.
    let (out, code) = run("class C; static int x; endclass\n\
        module t; initial $display(\"R\"); endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "static member must still be loud:\n{out}"
    );
}
