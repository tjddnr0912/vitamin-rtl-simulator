//! N7 class/OOP. S1 (this file's current scope): class declarations, data
//! members (correct per-field widths + IEEE §8.8 defaults: 0 for 2-state/int,
//! X for 4-state logic, null for handles), `new` allocation, `null`, and handle
//! ref-copy aliasing. Pure IR-0 (engine `class_heap` + sidecars; no sim-ir /
//! format_version change).
//!
//! Oracle: iverilog 13.0 supports the non-virtual surface (fields/`new`) and is
//! used differentially where it compiles. It does NOT compile handle COMPARISON
//! (`p == q`, `p == null` → "SORRY: Compare class handles not implemented"), so
//! those are hand-checked here against IEEE §8.4 (a handle is an object-id; two
//! handles compare equal iff they point at the same object; an unset handle is
//! null = 0).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_n7_{}_{n}", std::process::id()));
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
fn fields_default_init_and_widths() {
    // int field defaults to 0 (2-state), logic field to X (4-state); the field
    // read reports the FIELD width, not the 32-bit handle width.
    let (out, _) = run("class Packet;\n\
        int addr; logic [7:0] data;\n\
        endclass\n\
        module t; Packet p;\n\
        initial begin p = new;\n\
          $display(\"def addr=%0d data=%b w=%0d\", p.addr, p.data, $bits(p.data));\n\
        end endmodule\n");
    assert!(
        out.contains("def addr=0 data=xxxxxxxx w=8"),
        "field defaults/widths:\n{out}"
    );
}

#[test]
fn field_write_read() {
    let (out, _) = run("class C; int a; logic [7:0] b; endclass\n\
        module t; C c;\n\
        initial begin c = new; c.a = 5; c.b = 8'hAB;\n\
          $display(\"a=%0d b=%h\", c.a, c.b);\n\
        end endmodule\n");
    assert!(out.contains("a=5 b=ab"), "field write/read:\n{out}");
}

#[test]
fn ref_copy_aliasing() {
    // `q = p` copies the HANDLE (object-id), so q and p alias the SAME object —
    // a write through q is visible through p (IEEE §8.4 reference semantics).
    let (out, _) = run("class C; int a; endclass\n\
        module t; C p, q;\n\
        initial begin p = new; p.a = 7; q = p; q.a = 99;\n\
          $display(\"p.a=%0d q.a=%0d\", p.a, q.a);\n\
        end endmodule\n");
    assert!(out.contains("p.a=99 q.a=99"), "ref-copy aliasing:\n{out}");
}

#[test]
fn field_declaration_initializer_applied() {
    // SW1 (2026-06-22 audit): a class field declared `int x = 42` must read back
    // 42 at construction (IEEE §8.8: member initializers run as part of new()),
    // NOT the bare type default 0. Plain non-derived class — previously dropped
    // the initializer silently (x=0, exit 0).
    let (out, code) = run(
        "class C; int x = 42; logic [3:0] y = 4'hA; int z; endclass\n\
        module t; C c;\n\
        initial begin c = new;\n\
          $display(\"x=%0d y=%h z=%0d\", c.x, c.y, c.z);\n\
        end endmodule\n",
    );
    assert!(out.contains("x=42 y=a z=0"), "field init:\n{out}");
    assert_eq!(code, Some(0));
}

#[test]
fn auto_super_new_runs_base_ctor() {
    // SW2 (2026-06-22 audit): when a derived class has its OWN constructor that
    // omits super.new(), the compiler auto-inserts super.new() as the first
    // statement (IEEE §8.13 / spec 06-classes-oop.md:121), so the base ctor runs.
    // Previously the base ctor was silently skipped (x=0, exit 0).
    let (out, code) = run(
        "class Base; int x; function new(); x = 5; endfunction endclass\n\
        class Der extends Base; int y; function new(); y = 9; endfunction endclass\n\
        module t; Der d;\n\
        initial begin d = new;\n\
          $display(\"x=%0d y=%0d\", d.x, d.y);\n\
        end endmodule\n",
    );
    assert!(out.contains("x=5 y=9"), "auto super.new:\n{out}");
    assert_eq!(code, Some(0));
}

#[test]
fn explicit_super_new_still_works() {
    // Guard: an explicit super.new() must NOT double-run the base ctor.
    let (out, _) = run(
        "class Base; int x; function new(); x = x + 5; endfunction endclass\n\
        class Der extends Base; int y; function new(); super.new(); y = 9; endfunction endclass\n\
        module t; Der d;\n\
        initial begin d = new;\n\
          $display(\"x=%0d y=%0d\", d.x, d.y);\n\
        end endmodule\n",
    );
    // base ctor runs exactly once: x = 0 + 5 = 5 (not 10).
    assert!(out.contains("x=5 y=9"), "explicit super.new once:\n{out}");
}

#[test]
fn null_deref_is_x_not_panic() {
    // An unset handle is null; dereferencing it reads X (+ a warn-once), never a
    // panic. `h == null` is TRUE for an unset handle (handle value 0).
    let (out, _) = run("class C; int a; endclass\n\
        module t; C h;\n\
        initial begin\n\
          $display(\"null a=%0d isnull=%0d\", h.a, (h == null));\n\
        end endmodule\n");
    // `a` reads a partial-unknown int (bit-0 x, zero-extended) → %0d prints an
    // UPPERCASE `X` per IEEE §21.2.1.2 (lowercase x is reserved for an entirely-
    // unknown field). Either way the deref is unknown and does not panic.
    assert!(out.contains("null a=X isnull=1"), "null deref:\n{out}");
}

#[test]
fn handle_equality_same_vs_different_object() {
    // Two distinct `new`s are different objects (p != q); an alias compares equal.
    let (out, _) = run("class C; int a; endclass\n\
        module t; C p, q, r;\n\
        initial begin p = new; q = new; r = p;\n\
          $display(\"pq=%0d pr=%0d pnull=%0d\", (p==q), (p==r), (p==null));\n\
        end endmodule\n");
    assert!(out.contains("pq=0 pr=1 pnull=0"), "handle equality:\n{out}");
}

#[test]
fn two_distinct_objects_independent_fields() {
    let (out, _) = run("class C; int a; endclass\n\
        module t; C p, q;\n\
        initial begin p = new; q = new; p.a = 1; q.a = 2;\n\
          $display(\"p=%0d q=%0d\", p.a, q.a);\n\
        end endmodule\n");
    assert!(out.contains("p=1 q=2"), "object independence:\n{out}");
}

// ── S2/S3: constructors + methods (iverilog differential) ──────────────────

#[test]
fn ctor_with_args_sets_fields() {
    let (out, _) = run("class C;\n\
        int a; logic [7:0] b;\n\
        function new(int x, logic [7:0] y); a = x; b = y; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new(42, 8'hAB); $display(\"a=%0d b=%h\", c.a, c.b); end\n\
        endmodule\n");
    assert!(out.contains("a=42 b=ab"), "ctor with args:\n{out}");
}

#[test]
fn value_method_reads_this_field() {
    let (out, _) = run("class C;\n\
        int v;\n\
        function new(int x); v = x; endfunction\n\
        function int doubled(); return v * 2; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new(21); $display(\"d=%0d\", c.doubled()); end\n\
        endmodule\n");
    assert!(out.contains("d=42"), "value method:\n{out}");
}

#[test]
fn void_task_method_mutates_field() {
    let (out, _) = run("class C;\n\
        int n;\n\
        function new(); n = 0; endfunction\n\
        task add(int by); n = n + by; endtask\n\
        function int get(); return n; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new; c.add(3); c.add(4); $display(\"n=%0d\", c.get()); end\n\
        endmodule\n");
    assert!(out.contains("n=7"), "void task method:\n{out}");
}

#[test]
fn method_calls_another_method() {
    // `this.m()` from within a method (nested call).
    let (out, _) = run("class C;\n\
        int v;\n\
        function new(int x); v = x; endfunction\n\
        function int base(); return v; endfunction\n\
        function int plus10(); return base() + 10; endfunction\n\
        endclass\n\
        module t; C c;\n\
        initial begin c = new(5); $display(\"r=%0d\", c.plus10()); end\n\
        endmodule\n");
    assert!(out.contains("r=15"), "nested method call:\n{out}");
}

// ── S4: single inheritance + super (iverilog differential, non-virtual) ────

#[test]
fn inherited_field_and_method() {
    let (out, _) = run("class Base;\n\
        int x;\n\
        function new(); x = 100; endfunction\n\
        function int getx(); return x; endfunction\n\
        endclass\n\
        class Derived extends Base;\n\
        int y;\n\
        function new(); super.new(); y = 7; endfunction\n\
        endclass\n\
        module t; Derived d;\n\
        initial begin d = new; $display(\"x=%0d getx=%0d y=%0d\", d.x, d.getx(), d.y); end\n\
        endmodule\n");
    assert!(
        out.contains("x=100 getx=100 y=7"),
        "inheritance+super:\n{out}"
    );
}

#[test]
fn super_method_call() {
    // `super.m()` dispatches statically to the base implementation.
    let (out, _) = run("class Base;\n\
        function int who(); return 1; endfunction\n\
        endclass\n\
        class Derived extends Base;\n\
        function int who(); return 2; endfunction\n\
        function int both(); return who() * 10 + super.who(); endfunction\n\
        endclass\n\
        module t; Derived d;\n\
        initial begin d = new; $display(\"both=%0d\", d.both()); end\n\
        endmodule\n");
    // who() (this, non-virtual static type = Derived) = 2; super.who() = 1 → 21.
    assert!(out.contains("both=21"), "super method call:\n{out}");
}

// ── S5: VIRTUAL dynamic dispatch — HAND-IEEE lane ──────────────────────────
// ⚠️ iverilog 13.0 has a CONFIRMED virtual-dispatch BUG: a base-class handle
// holding a derived object calls the STATIC-type method (verified live:
// `a.sound()` returns 0, IEEE-correct is 7). So these are NOT differential vs
// iverilog — the expected output is hand-authored from IEEE §8.20. Do NOT "fix"
// vita to match the buggy oracle.

#[test]
fn virtual_dispatch_base_handle_derived_object() {
    let (out, _) = run("class Animal;\n\
        virtual function int sound(); return 0; endfunction\n\
        endclass\n\
        class Dog extends Animal;\n\
        virtual function int sound(); return 7; endfunction\n\
        endclass\n\
        module t; Animal a; Dog d;\n\
        initial begin\n\
          d = new; a = d;\n\
          $display(\"a=%0d d=%0d\", a.sound(), d.sound());\n\
          a = new;\n\
          $display(\"base=%0d\", a.sound());\n\
        end endmodule\n");
    // a holds a Dog → dynamic dispatch to Dog::sound = 7 (IEEE; iverilog wrongly
    // gives 0). Then a holds a plain Animal → Animal::sound = 0.
    assert!(
        out.contains("a=7 d=7"),
        "virtual dispatch (dynamic type):\n{out}"
    );
    assert!(
        out.contains("base=0"),
        "virtual dispatch (base object):\n{out}"
    );
}

#[test]
fn non_virtual_uses_static_type() {
    // A NON-virtual method called through a base handle uses the STATIC (base)
    // type — the inverse of the virtual case (this one iverilog also agrees on).
    let (out, _) = run("class Animal;\n\
        function int kind(); return 1; endfunction\n\
        endclass\n\
        class Dog extends Animal;\n\
        function int kind(); return 2; endfunction\n\
        endclass\n\
        module t; Animal a; Dog d;\n\
        initial begin d = new; a = d; $display(\"a=%0d d=%0d\", a.kind(), d.kind()); end\n\
        endmodule\n");
    // a's static type is Animal → Animal::kind = 1; d's is Dog → 2.
    assert!(
        out.contains("a=1 d=2"),
        "non-virtual static dispatch:\n{out}"
    );
}

#[test]
fn virtual_super_calls_parent_impl() {
    // `super.vmethod()` is a STATIC call to the parent even for virtual methods.
    let (out, _) = run("class Base;\n\
        virtual function int f(); return 10; endfunction\n\
        endclass\n\
        class Sub extends Base;\n\
        virtual function int f(); return super.f() + 5; endfunction\n\
        endclass\n\
        module t; Base b; Sub s;\n\
        initial begin s = new; b = s; $display(\"b=%0d\", b.f()); end\n\
        endmodule\n");
    // b holds a Sub → virtual dispatch to Sub::f → super.f()(=10) + 5 = 15.
    assert!(out.contains("b=15"), "virtual + super:\n{out}");
}

// ── S6: deferred constructs must be LOUD (adversarial-found) ───────────────

#[test]
fn array_of_handles_member_is_loud() {
    // An array-of-handles class member used to be SILENTLY treated as a scalar
    // handle (the `[N]` dropped) — now loud, not silent-wrong.
    let (out, code) = run("class C; int x; endclass\n\
        class D; C arr[4]; endclass\n\
        module t; initial $display(\"R\"); endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "array-of-handles member must be loud:\n{out}"
    );
}

#[test]
fn output_method_port_is_loud() {
    // An output/inout method port has no copy-out in the discarded-call model —
    // loud, not a silent dropped write-back.
    let (out, code) = run("class C; task t(output int o); o = 1; endtask endclass\n\
        module t; initial $display(\"R\"); endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "output method port must be loud:\n{out}"
    );
}

// ── Adversarial-found silent-wrong fixes (workflow weuhnohqj) ──────────────

#[test]
fn forged_handle_from_integer_is_loud() {
    // HIGH (use-after-free): forging a handle from an integer used to silently
    // alias + corrupt a live object. Now a loud type-gate reject.
    let (out, code) = run("class C; int x; endclass\n\
        module top; C a, forged;\n\
        initial begin a = new; a.x = 5; forged = 1; forged.x = 999;\n\
          $display(\"a.x=%0d\", a.x); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "forging a handle from an int must be loud:\n{out}"
    );
    assert!(
        !out.contains("a.x=999"),
        "must not corrupt the live object:\n{out}"
    );
}

#[test]
fn formal_shadows_field() {
    // HIGH: a method formal that shadows a class field must win (innermost-wins,
    // IEEE §13.4) — a bare reference is the FORMAL, not the property.
    let (out, _) = run("class C;\n\
        int v;\n\
        function int echo(int v); return v; endfunction\n\
        endclass\n\
        module t; C c; int r;\n\
        initial begin c = new; r = c.echo(7); $display(\"r=%0d\", r); end\n\
        endmodule\n");
    assert!(out.contains("r=7"), "formal must shadow field:\n{out}");
}

#[test]
fn ctor_this_field_eq_formal_idiom() {
    // HIGH: the canonical `this.v = v` constructor idiom — RHS `v` is the FORMAL,
    // LHS `this.v` is the property. Previously left the field at default 0.
    let (out, _) = run("class C;\n\
        int v;\n\
        function new(int v); this.v = v; endfunction\n\
        function int get(); return v; endfunction\n\
        endclass\n\
        module t; C c; int r;\n\
        initial begin c = new(99); r = c.get(); $display(\"r=%0d\", r); end\n\
        endmodule\n");
    assert!(out.contains("r=99"), "this.v=v ctor idiom:\n{out}");
}

#[test]
fn keyword_less_virtual_override_dispatches() {
    // HIGH (hand-IEEE §8.20): an override that omits `virtual` still dispatches
    // dynamically (virtuality is inherited). iverilog 13 wrongly prints 1.
    let (out, _) = run(
        "class Base; virtual function int m(); return 1; endfunction endclass\n\
        class Der extends Base; function int m(); return 2; endfunction endclass\n\
        module top; Base b; Der d;\n\
        initial begin d = new; b = d; $display(\"b=%0d\", b.m()); end\n\
        endmodule\n",
    );
    assert!(
        out.contains("b=2"),
        "keyword-less override is virtual:\n{out}"
    );
}

#[test]
fn wide_field_not_truncated() {
    // HIGH: a >32-bit field write used to truncate to 32 bits (chunk_width = the
    // 32-bit handle width). Now the full field width is preserved.
    let (out, _) = run("class C; logic [63:0] big; endclass\n\
        module t; C c;\n\
        initial begin c = new; c.big = 64'hAABBCCDD_EEFF0011; $display(\"big=%h\", c.big); end\n\
        endmodule\n");
    assert!(
        out.contains("big=aabbccddeeff0011"),
        "64-bit field must not truncate:\n{out}"
    );
}

#[test]
fn field_shadowing_distinct_storage() {
    // ⓑ-breadth: a derived field redeclaring a base field name (IEEE §8.14) now
    // gets DISTINCT storage — a derived/external reference reaches the derived
    // field, a base method reaches the base field. (Was loud.) See
    // class_field_shadow.rs for the full characterization.
    let (out, code) = run("class Base; int x;\n\
          function int gx(); return x; endfunction\n\
          function void sx(int v); x = v; endfunction endclass\n\
        class Der extends Base; int x; endclass\n\
        module t; Der d;\n\
        initial begin d = new; d.sx(1); d.x = 2; $display(\"d=%0d b=%0d\", d.x, d.gx()); end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "shadowing must elaborate:\n{out}");
    assert!(out.contains("d=2 b=1"), "distinct storage:\n{out}");
}

#[test]
fn handle_leaked_to_int_is_loud() {
    // MEDIUM: assigning a handle to an integral leaks the internal object-id.
    let (out, code) = run("class C; int x; endclass\n\
        module top; C a; int id;\n\
        initial begin a = new; id = a; $display(\"id=%0d\", id); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "handle→int leak must be loud:\n{out}"
    );
}

#[test]
fn null_in_arithmetic_is_loud() {
    // MEDIUM: `null` used as integer 0 in a non-handle context.
    let (out, code) = run("module top; int x;\n\
        initial begin x = null + 5; $display(\"x=%0d\", x); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "null in arithmetic must be loud:\n{out}"
    );
}

#[test]
fn handle_arithmetic_is_loud() {
    // MEDIUM: arithmetic on a handle (computes on the internal object-id).
    let (out, code) = run("class C; int x; endclass\n\
        module top; C a; int r;\n\
        initial begin a = new; r = a + 5; $display(\"r=%0d\", r); end\n\
        endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "handle arithmetic must be loud:\n{out}"
    );
}
