//! Slice A.5 — class cast `Base'(d)` UP-CAST (IEEE 1800-2017 §6.24.2 / §8.16 /
//! §8.20). Pure IR-0 (elaborate-only; `CastTarget::Named` already exists).
//!
//! Oracle: iverilog 13.0 does NOT compile a class cast (`Base'(d)` → syntax
//! error / unsupported), so every expected value here is hand-authored from the
//! IEEE 1800-2017 §6.24.2 up-cast semantics:
//!   * A class handle is an object-id; the heap object carries its CONCRETE
//!     class_id, fixed at `new` and never changed by a cast.
//!   * An UP-cast (target is the operand's class or an ancestor) is a pure
//!     IDENTITY on the handle value — it only narrows the STATIC type.
//!   * VIRTUAL dispatch reads the dynamic (heap) class → the most-derived
//!     override runs regardless of the static cast type (§8.20).
//!   * A NON-virtual call uses the STATIC type → after `Base b = Base'(d)`, a
//!     call `b.nmeth()` runs the BASE implementation (the static type of `b` is
//!     `Base`, taken from `b`'s OWN declaration, not from the cast).
//!
//! v1 LOUD scope (correct-or-loud): a static DOWN-cast `Derived'(base)`, a cast
//! of an unresolvable operand, an UNRELATED cast, and a cast used directly as a
//! method receiver `(Base'(d)).foo()` are all rejected LOUDLY (never
//! silent-wrong). iverilog rejecting class casts confirms there is no oracle to
//! pin a wrong answer against — loud is the only safe behavior.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_a5cast_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

// ── UP-CAST + VIRTUAL base method: dispatch unchanged by the cast ───────────
#[test]
fn upcast_virtual_dispatch_most_derived() {
    // d holds a Dog; `Base b = Animal'(d)` up-casts (identity). `b.sound()` is
    // VIRTUAL → dynamic (heap) type Dog → Dog::sound = 7 (§8.20). The cast does
    // not change dispatch.
    let (out, _) = run("class Animal;\n\
        virtual function int sound(); return 0; endfunction\n\
        endclass\n\
        class Dog extends Animal;\n\
        virtual function int sound(); return 7; endfunction\n\
        endclass\n\
        module t; Animal b; Dog d;\n\
        initial begin\n\
          d = new;\n\
          b = Animal'(d);\n\
          $display(\"v=%0d\", b.sound());\n\
        end endmodule\n");
    assert!(out.contains("v=7"), "upcast virtual dispatch:\n{out}");
}

// ── UP-CAST + NON-VIRTUAL base method: STATIC (dest) type wins ──────────────
#[test]
fn upcast_nonvirtual_uses_static_type() {
    // `b.kind()` is NON-virtual → uses the STATIC type of `b` (= Animal, from
    // b's own declaration) → Animal::kind = 1, NOT Dog::kind = 2. The cast's job
    // is just to yield the handle value; the static type comes from `b`'s net.
    let (out, _) = run("class Animal;\n\
        function int kind(); return 1; endfunction\n\
        endclass\n\
        class Dog extends Animal;\n\
        function int kind(); return 2; endfunction\n\
        endclass\n\
        module t; Animal b; Dog d;\n\
        initial begin\n\
          d = new;\n\
          b = Animal'(d);\n\
          $display(\"nv=%0d\", b.kind());\n\
        end endmodule\n");
    assert!(
        out.contains("nv=1"),
        "upcast non-virtual static type:\n{out}"
    );
}

// ── identity up-cast: target == operand class (legal, no-op) ────────────────
#[test]
fn identity_cast_same_class() {
    let (out, _) = run("class C;\n\
        function int v(); return 42; endfunction\n\
        endclass\n\
        module t; C a, b;\n\
        initial begin\n\
          a = new;\n\
          b = C'(a);\n\
          $display(\"id=%0d\", b.v());\n\
        end endmodule\n");
    assert!(out.contains("id=42"), "identity cast:\n{out}");
}

// ── NULL operand up-cast → identity (value 0), no crash ─────────────────────
#[test]
fn null_operand_upcast_identity() {
    // `d` is an unset (null) handle. `Animal'(d)` is identity → null (0). The
    // assignment is legal; reading the handle as null compares equal to null.
    let (out, _) = run("class Animal; endclass\n\
        class Dog extends Animal; endclass\n\
        module t; Animal b; Dog d;\n\
        initial begin\n\
          b = Animal'(d);\n\
          $display(\"isnull=%0d\", (b == null));\n\
        end endmodule\n");
    assert!(out.contains("isnull=1"), "null operand upcast:\n{out}");
}

// ── LOUD: static DOWN-cast `Derived'(base_handle)` (v1 loud) ────────────────
#[test]
fn downcast_is_loud() {
    let (out, code) = run("class Animal; endclass\n\
        class Dog extends Animal; endclass\n\
        module t; Animal a; Dog b;\n\
        initial begin\n\
          a = new;\n\
          b = Dog'(a);\n\
        end endmodule\n");
    assert!(out.contains("VITA-E"), "down-cast must be loud:\n{out}");
    assert_ne!(code, Some(0), "down-cast must exit non-zero:\n{out}");
}

// ── LOUD: UNRELATED cast (two unrelated classes) ────────────────────────────
#[test]
fn unrelated_cast_is_loud() {
    let (out, code) = run("class A; endclass\n\
        class B; endclass\n\
        module t; A x; B y;\n\
        initial begin\n\
          y = new;\n\
          x = A'(y);\n\
        end endmodule\n");
    assert!(
        out.contains("VITA-E"),
        "unrelated cast must be loud:\n{out}"
    );
    assert_ne!(code, Some(0), "unrelated cast must exit non-zero:\n{out}");
}

// ── LOUD: cast of an UNRESOLVABLE operand (arbitrary expr) ──────────────────
#[test]
fn unresolvable_operand_cast_is_loud() {
    // The operand is a method CALL — its static class cannot be resolved by the
    // net_class path → cannot validate the relationship → loud (correct-or-loud).
    let (out, code) = run("class Animal;\n\
        function Dog mk(); endfunction\n\
        endclass\n\
        class Dog extends Animal; endclass\n\
        module t; Animal a; Dog d;\n\
        initial begin\n\
          d = new;\n\
          a = Animal'(d.mk());\n\
        end endmodule\n");
    assert!(
        out.contains("VITA-E"),
        "unresolvable operand must be loud:\n{out}"
    );
    assert_ne!(
        code,
        Some(0),
        "unresolvable operand must exit non-zero:\n{out}"
    );
}

// ── LOUD (clean): cast used directly as a method receiver ───────────────────
#[test]
fn cast_as_method_receiver_is_loud_not_silent() {
    // `(Base'(d)).foo()` — a cast cannot serve as a method receiver in v1 (the
    // static type for `.foo` would have to ride on a lowered value, which vita
    // does not track). The parser already rejects this form (no method-call on a
    // paren/cast receiver) → a CLEAN loud error, not a silent mis-resolution.
    let (out, code) = run("class Base;\n\
        virtual function void foo(); $display(\"base\"); endfunction\n\
        endclass\n\
        class Der extends Base;\n\
        virtual function void foo(); $display(\"der\"); endfunction\n\
        endclass\n\
        module t; Der d;\n\
        initial begin\n\
          d = new();\n\
          (Base'(d)).foo();\n\
        end endmodule\n");
    assert!(
        out.contains("VITA-E"),
        "cast-as-receiver must be loud:\n{out}"
    );
    // The method bodies print "base"/"der" on their OWN line via $display; assert
    // no such line ran (a substring check would false-match the temp path
    // ".../folders/..." → "der").
    assert!(
        !out.lines().any(|l| l.trim() == "base" || l.trim() == "der"),
        "cast-as-receiver must NOT silently run a method:\n{out}"
    );
    assert_ne!(code, Some(0), "cast-as-receiver must exit non-zero:\n{out}");
}
