//! EXT2-E: a package scope-qualified type name `pkg::t` used in a TYPE position
//! (IEEE 1800 §26.3). vita previously parse-rejected every such use (E2002
//! "expected ')', found ColonColon" for ports; "expected identifier" for a body
//! decl the parser mis-read as an instantiation) while already accepting the bare
//! name and scoped VALUE refs (`pkg::CONST`). The fix resolves a scoped type at the
//! single classifier funnel (`peek_typedef_name`/`peek_block_typedef_decl`) and
//! skips the `pkg::` qualifier at every type-name consumer.
//!
//! Grounding (iverilog 13.0) expanded the recorded gap (port + body decl) to every
//! type position: ANSI/non-ANSI port, body var decl, function return + tf-arg type,
//! packed-struct member, and chained typedef base.
//!
//! CRITICAL soundness pin — cross-package same-name COLLISION: vita's typedef
//! registry is flat/bare-keyed, so a naive resolve of the final segment would make
//! `pa::t` (8-bit) and `pb::t` (16-bit) both resolve to the last-registered `t`
//! (silent-wrong). The fix registers a package-scoped twin key `"pkg::name"` for
//! each package typedef, so a scoped type resolves to the RIGHT package. The
//! `collision_*` tests pin this to iverilog and are the regression teeth.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scoped_{}_{n}", std::process::id()));
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
fn scoped_port_type_ansi() {
    // `input pk::byte_t a` — pinned to iverilog (y = a + 1 = 11).
    let (out, code) = run("package pk; typedef logic [7:0] byte_t; endpackage\n\
         module sub(input pk::byte_t a, output pk::byte_t b); assign b = a + 8'd1; endmodule\n\
         module top; logic [7:0] x, y; sub u(.a(x), .b(y));\n\
         initial begin x = 8'd10; #1 $display(\"y=%0d\", y); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped ANSI port type must parse+run:\n{out}"
    );
    assert!(out.contains("y=11"), "{out}");
}

#[test]
fn scoped_port_type_nonansi() {
    // Non-ANSI port body decl `input pk::byte_t a;` — pinned to iverilog (y=6).
    let (out, code) = run("package pk; typedef logic [7:0] byte_t; endpackage\n\
         module sub(a, b); input pk::byte_t a; output pk::byte_t b; assign b = a + 1; endmodule\n\
         module top; logic [7:0] x, y; sub u(.a(x), .b(y));\n\
         initial begin x = 8'd5; #1 $display(\"y=%0d\", y); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped non-ANSI port type must parse+run:\n{out}"
    );
    assert!(out.contains("y=6"), "{out}");
}

#[test]
fn scoped_body_var_decl() {
    // `pk::byte_t v;` as a module-body decl (was mis-read as an instantiation).
    let (out, code) = run("package pk; typedef logic [7:0] byte_t; endpackage\n\
         module top; pk::byte_t v;\n\
         initial begin v = 8'd42; #1 $display(\"v=%0d\", v); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "scoped body var decl must parse+run:\n{out}");
    assert!(out.contains("v=42"), "{out}");
}

#[test]
fn scoped_body_decl_init_and_comma_list() {
    // decl-init + comma list + a scoped VALUE ref baseline (already supported).
    let (out, code) = run(
        "package pk; typedef logic [7:0] byte_t; localparam int W = 5; endpackage\n\
         module top; pk::byte_t v = 8'd7; pk::byte_t a, b; logic [7:0] base = pk::W;\n\
         initial begin a = 8'd1; b = 8'd2; #1\n\
         $display(\"v=%0d a=%0d b=%0d base=%0d\", v, a, b, base); $finish; end endmodule\n",
    );
    assert_eq!(
        code,
        Some(0),
        "scoped decl-init/comma-list must parse+run:\n{out}"
    );
    assert!(out.contains("v=7 a=1 b=2 base=5"), "{out}");
}

#[test]
fn scoped_function_return_and_arg_type() {
    // `function pk::byte_t f(input pk::byte_t x)` — return AND arg scoped types.
    let (out, code) = run("package pk; typedef logic [7:0] byte_t; endpackage\n\
         module top;\n\
         function automatic pk::byte_t addf(input pk::byte_t x); addf = x + 8'd3; endfunction\n\
         pk::byte_t r;\n\
         initial begin r = addf(8'd9); #1 $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped fn return+arg type must parse+run:\n{out}"
    );
    assert!(out.contains("r=12"), "{out}");
}

#[test]
fn scoped_struct_member_and_field() {
    // A scoped SIMPLE type as a packed-struct member, plus a scoped struct TYPE as
    // a body decl with field access — both pinned to iverilog.
    let (out, code) = run("package pk; typedef struct packed { logic [3:0] hi; logic [3:0] lo; } pr_t; endpackage\n\
         module top; pk::pr_t p;\n\
         initial begin p.hi = 4'd5; p.lo = 4'd9; #1 $display(\"hi=%0d lo=%0d\", p.hi, p.lo); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped struct type + field must parse+run:\n{out}"
    );
    assert!(out.contains("hi=5 lo=9"), "{out}");
}

#[test]
fn scoped_chained_typedef() {
    // `typedef pk::byte_t my_t;` — a chained alias of a scoped base type.
    let (out, code) = run("package pk; typedef logic [7:0] byte_t; endpackage\n\
         module top; typedef pk::byte_t my_t; my_t v;\n\
         initial begin v = 8'd99; #1 $display(\"v=%0d\", v); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped chained typedef must parse+run:\n{out}"
    );
    assert!(out.contains("v=99"), "{out}");
}

#[test]
fn scoped_enum_type_as_vector() {
    // A scoped enum TYPE resolves to its base vector; a qualified label assigns.
    let (out, code) = run(
        "package pk; typedef enum logic [1:0] { A, B, C } e_t; endpackage\n\
         module top; pk::e_t s;\n\
         initial begin s = pk::C; #1 $display(\"s=%0d\", s); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "scoped enum type must parse+run:\n{out}");
    assert!(out.contains("s=2"), "{out}");
}

// ---- CRITICAL: cross-package same-name collision (silent-wrong regression teeth) ----

#[test]
fn collision_simple_type_resolves_per_package() {
    // pa::t = logic[7:0] (8-bit → 0xABCD truncates to CD); pb::t = logic[15:0]
    // (16-bit → ABCD). A flat bare-keyed resolve would give both the last `t`
    // (16-bit) → x=abcd (silent-wrong). Pinned to iverilog: x=cd, y=abcd.
    let (out, code) = run("package pa; typedef logic [7:0]  t; endpackage\n\
         package pb; typedef logic [15:0] t; endpackage\n\
         module top; pa::t x; pb::t y;\n\
         initial begin x = 16'hABCD; y = 16'hABCD; #1 $display(\"x=%h y=%h\", x, y); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "cross-package collision must parse+run:\n{out}"
    );
    assert!(
        out.contains("x=cd y=abcd"),
        "scoped type must resolve to its OWN package (silent-wrong guard):\n{out}"
    );
}

#[test]
fn collision_struct_layout_resolves_per_package() {
    // pa::s and pb::s are DIFFERENT packed-struct layouts. Each scoped access must
    // use its own package's layout. Pinned to iverilog: a.x=ab, b.hi=1, b.lo=2.
    let (out, code) = run(
        "package pa; typedef struct packed { logic [7:0] x; } s; endpackage\n\
         package pb; typedef struct packed { logic [3:0] lo; logic [3:0] hi; } s; endpackage\n\
         module top; pa::s a; pb::s b;\n\
         initial begin a.x = 8'hAB; b.hi = 4'd1; b.lo = 4'd2; #1\n\
         $display(\"a.x=%h b.hi=%0d b.lo=%0d\", a.x, b.hi, b.lo); $finish; end endmodule\n",
    );
    assert_eq!(
        code,
        Some(0),
        "cross-package struct collision must parse+run:\n{out}"
    );
    assert!(
        out.contains("a.x=ab b.hi=1 b.lo=2"),
        "scoped struct must use its OWN package's layout (silent-wrong guard):\n{out}"
    );
}

#[test]
fn collision_mixed_kind_vector_not_stale_struct() {
    // MIXED-kind collision (regression teeth for the adversarial-review finding):
    // `pa::t` is a STRUCT, `pb::t` is a plain VECTOR. A naive per-sub-map twin copy
    // would leak pa's struct LAYOUT onto `pb::t` (its `struct_layouts` entry is never
    // overwritten by pb's plain alias), so `pb::t v; v.x` would silently read pa's
    // layout (silent-wrong). `pb::t` must resolve as a bare 4-bit vector.
    let (out, code) = run(
        "package pa; typedef struct packed { logic [7:0] x; logic [7:0] y; } t; endpackage\n\
         package pb; typedef logic [3:0] t; endpackage\n\
         module top; pb::t v;\n\
         initial begin v = 4'hA; #1 $display(\"v=%h\", v); $finish; end endmodule\n",
    );
    assert_eq!(
        code,
        Some(0),
        "mixed-kind scoped vector must parse+run:\n{out}"
    );
    assert!(
        out.contains("v=a"),
        "pb::t must be a plain 4-bit vector:\n{out}"
    );
}

#[test]
fn collision_mixed_kind_stale_struct_field_is_loud() {
    // Same mixed-kind collision — a field access on the plain-vector `pb::t` must be
    // LOUD (iverilog: "does not have a field named x"), never silently desugared
    // against pa's stale struct layout.
    let (out, code) = run(
        "package pa; typedef struct packed { logic [7:0] x; logic [7:0] y; } t; endpackage\n\
         package pb; typedef logic [3:0] t; endpackage\n\
         module top; pb::t v;\n\
         initial begin v = 4'hA; #1 $display(\"vx=%h\", v.x); $finish; end endmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "field access on a plain-vector scoped type must be loud (no stale layout):\n{out}"
    );
}

#[test]
fn collision_mixed_kind_reverse_order() {
    // Reverse ordering (vector `pa::t` first, struct `pb::t` second): both must
    // resolve to their own package. Pinned to iverilog: v=5, wx=ab, wy=cd.
    let (out, code) = run("package pa; typedef logic [3:0] t; endpackage\n\
         package pb; typedef struct packed { logic [7:0] x; logic [7:0] y; } t; endpackage\n\
         module top; pa::t v; pb::t w;\n\
         initial begin v = 4'h5; w.x = 8'hAB; w.y = 8'hCD; #1\n\
         $display(\"v=%h wx=%h wy=%h\", v, w.x, w.y); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "reverse-order mixed-kind must parse+run:\n{out}"
    );
    assert!(out.contains("v=5 wx=ab wy=cd"), "{out}");
}

#[test]
fn scoped_chained_alias_of_struct() {
    // A package chained-alias of another package's struct, referenced scoped
    // (`pb::alias_s`), must carry the struct layout (the `Alias` node is fresh
    // because this package body wrote its layout). Pinned to iverilog.
    let (out, code) = run("package pa; typedef struct packed { logic [7:0] x; logic [7:0] y; } s; endpackage\n\
         package pb; typedef pa::s alias_s; endpackage\n\
         module top; pb::alias_s v;\n\
         initial begin v.x = 8'hAB; v.y = 8'hCD; #1 $display(\"x=%h y=%h\", v.x, v.y); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "scoped chained-alias-of-struct must parse+run:\n{out}"
    );
    assert!(out.contains("x=ab y=cd"), "{out}");
}

#[test]
fn collision_foldable_enum_methods_resolve_per_package() {
    // Foldable enum `n` in two packages with DIFFERENT labels. `pb::n` methods must
    // use pb's labels. A `contains_key`-based twin would leak pa's labels; the
    // value-compare twin resolves correctly. Pinned to iverilog.
    let (out, code) = run("package pa; typedef enum logic [1:0] { A=0, B=1 } n; endpackage\n\
         package pb; typedef enum logic [3:0] { X=5, Y=6 } n; endpackage\n\
         module top; pb::n e;\n\
         initial begin e = pb::Y; #1 $display(\"e=%0d first=%0d num=%0d\", e, e.first, e.num); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "foldable enum collision methods must parse+run:\n{out}"
    );
    assert!(
        out.contains("e=6 first=5 num=2"),
        "pb::n methods must use pb's labels:\n{out}"
    );
}

#[test]
fn collision_localparam_enum_methods_use_their_own_packages_labels() {
    // ⚠️ This asserted a LOUD reject until §0 T2 let an enum label fold a module-scope
    // `localparam`. The property it was really guarding is stronger and is now checked
    // POSITIVELY: a `contains_key`-based twin would copy the foldable same-name enum's
    // labels from package `pa` (three of them), so `num` proves which package's labels
    // bound. It is 2 — pb's — not 3.
    let (out, code) = run("package pa; typedef enum logic [1:0] { A, B, C } n; endpackage\n\
         package pb; localparam int BASE = 5; typedef enum logic [3:0] { X = BASE, Y } n; endpackage\n\
         module top; pb::n e;\n\
         initial begin e = pb::X; #1 $display(\"m=%0d num=%0d first=%0d\", e, e.num, e.first); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "a localparam label folds now:\n{out}");
    assert!(
        out.contains("m=5 num=2 first=5"),
        "pb's own labels must bind (pa's would give num=3):\n{out}"
    );
}

#[test]
fn collision_nonfoldable_enum_methods_are_loud() {
    // The half that is still non-foldable, and the one where the staleness hazard
    // actually lives: a `parameter` label. It cannot fold at parse time because an
    // instance override changes the label values (measured — `#(.K(9))` moves them), so
    // the enum stays out of `enum_defs`, and a `contains_key`-based twin would copy the
    // same-name enum's STALE labels from `pa`. The value-compare twin skips the stale
    // entry, so the methods are honest-loud instead (the plain VALUE still resolves
    // through the unconditional TypeInfo twin — see the test below).
    // §4.5.414: a PACKAGE `parameter` is a localparam (IEEE §6.20.1 — nothing can
    // override it), so `BASE` now folds at parse time and pb's enum registers: `num`
    // is pb's 2 (pa's stale labels would answer 3). Both oracles: 2.
    let (out, code) = run("package pa; typedef enum logic [1:0] { A, B, C } n; endpackage\n\
         package pb; parameter int BASE = 5; typedef enum logic [3:0] { X = BASE, Y } n; endpackage\n\
         module top; pb::n e;\n\
         initial begin e = pb::X; #1 $display(\"m=%0d\", e.num); $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("m=2"),
        "pb's own label count, not pa's stale 3:\n{out}"
    );
    // The half that is STILL non-foldable: a module HEADER parameter label — an
    // instance override changes the label values (`#(.K(9))` moves them), so the
    // enum stays out of `enum_defs` and the methods are honest-loud.
    let (out, code) = run("module c #(parameter int BASE = 5) ();\n\
         typedef enum logic [3:0] { X = BASE, Y } n; n e;\n\
         initial begin e = X; #1 $display(\"m=%0d\", e.num); end endmodule\n\
         module top; c u(); initial #2 $finish; endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "an overridable parameter label must keep the methods loud:\n{out}"
    );
}

#[test]
fn collision_nonfoldable_enum_value_resolves() {
    // The plain VALUE of a non-foldable scoped enum still resolves to the right
    // package's width via the unconditional TypeInfo twin (only the label-method
    // binding is gated). Pinned to iverilog: e=6.
    let (out, code) = run("package pa; typedef enum logic [1:0] { A, B, C } n; endpackage\n\
         package pb; localparam int BASE = 5; typedef enum logic [3:0] { X = BASE, Y } n; endpackage\n\
         module top; pb::n e;\n\
         initial begin e = pb::Y; #1 $display(\"e=%0d\", e); $finish; end endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "non-foldable scoped enum value must resolve:\n{out}"
    );
    assert!(out.contains("e=6"), "{out}");
}

// ---- loud stays loud (correct-or-loud; not silently mis-parsed) ----

#[test]
fn scoped_struct_port_supported() {
    // EXT2-E1: a PACKED-struct typedef used via its SCOPED name (`pk::s_t`) as a
    // module port is supported (loud→supported) — the scoped path resolves the
    // layout and `a.field` desugars to a part-select. iverilog-oracled: s_t{x[7:0]};
    // a=8'h5A ⇒ a.x = 5a.
    let (out, code) = run(
        "package pk; typedef struct packed { logic [7:0] x; } s_t; endpackage\n\
         module sub(input pk::s_t a, output logic [7:0] o); assign o=a.x; endmodule\n\
         module top; logic [7:0] c,o; sub u(.a(c),.o(o));\n\
         initial begin c=8'h5A; #1 $display(\"R:%0h\",o); $finish; end endmodule\n",
    );
    assert_eq!(
        code,
        Some(0),
        "scoped struct port must be supported:\n{out}"
    );
    assert!(
        out.contains("R:5a"),
        "scoped struct port a.x must read 5a:\n{out}"
    );
}

#[test]
fn scoped_unknown_package_type_is_loud() {
    // `nope::t` where no package `nope` defines `t` — the scoped key is unregistered,
    // so it is NOT treated as a type (falls through to a loud parse error), never a
    // silent resolve to a same-named type from another scope.
    let (out, code) = run("package pk; typedef logic [7:0] t; endpackage\n\
         module top; nope::t v;\n\
         initial begin #1 $finish; end endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "unknown scoped package type must be loud:\n{out}"
    );
}

// ── §0 T2: an enum label may name a module-scope `localparam` ────────────────

/// ⚠️ The queue line called this residue "sized-literal enum label", and the census
/// refuted that: a sized literal folds fine (`enum bit[7:0] { A = 8'hFF }` runs, and
/// so do `.name`/`.first`/`.next`/`.num`). What was refused is a label naming a
/// CONSTANT — `A = L`, `A = L+1`, `A = L*2` — which left the whole enum out of
/// `enum_defs` and made every method on it loud with a misleading "hierarchical
/// function call" message. Both oracles fold it.
#[test]
fn an_enum_label_folds_a_localparam() {
    // (labels, expected "VAL=<B> <first>")
    for (labels, want) in [
        ("A = L, B = L+1", "VAL=6 5"),
        ("A = 1, B = L", "VAL=5 1"),
        ("A = L*2, B = L*3", "VAL=15 10"),
    ] {
        let (out, code) = run(&format!(
            "module top;\n  localparam L = 5;\n\
             typedef enum bit[7:0] {{ {labels} }} e_t; e_t x;\n\
             initial begin x = B; #1 $display(\"VAL=%0d %0d\", x, x.first); $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(code, Some(0), "`{labels}` must fold:\n{out}");
        assert!(
            out.contains(want),
            "`{labels}` must give `{want}` (both oracles do):\n{out}"
        );
    }
}

/// A `parameter` label must NOT fold, and this is the measurement that says why rather
/// than a rule taken on faith: an instance override CHANGES the label values. With
/// `m #(.K(9))` on `enum { A = K, B = K+1 }`, iverilog prints 10 and `first=9` — not 4
/// and 3. The parser folds before any override is known, so folding one there would be
/// silently wrong; `const_locals` already encodes the distinction ("a `parameter` is
/// overridable → never recorded") and the label fold reuses exactly that predicate.
#[test]
fn an_enum_label_does_not_fold_an_overridable_parameter() {
    let (out, code) = run("module m #(parameter K = 3) ();\n\
           typedef enum bit[7:0] { A = K, B = K+1 } e_t; e_t x;\n\
           initial begin x = B; $display(\"VAL=%0d %0d\", x, x.first); end\n\
         endmodule\n\
         module t; m #(.K(9)) u(); initial #1 $finish; endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "folding an overridable parameter at parse time would print 4/3 where both \
         oracles print 10/9:\n{out}"
    );
}
