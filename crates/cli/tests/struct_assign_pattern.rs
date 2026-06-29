//! Packed-struct positional assignment pattern `'{e0,…,eN}` (IEEE §10.9.1).
//! vita desugars it (at parse time, where struct layout lives) to the
//! field-width-cast concat `{w0'(e0), …, wN'(eN)}` — field 0 is the MSB
//! (leftmost), and each element is sized to its FIELD width, NOT
//! self-determined, so an unsized or fill (`'1`/`'x`/`'z`) element grows to the
//! field (`'{5,6}` ≠ `{5,6}`). Reuses the size-cast lowering (which, since the
//! #4.5.34 fix, sizes a fill operand in the cast width), so it is IR-0. Pinned
//! to iverilog 13.0 across decl-init, statement `=`/`<=`, unsized/fill/signed
//! elements, and count-mismatch loud-reject.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through `vita` and return (stdout, success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_structpat_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

const ST: &str = "module top; typedef struct packed \
     { logic [3:0] a; logic [7:0] b; logic c; } st_t; st_t s;";

#[test]
fn stmt_assign_sized() {
    let (out, ok) = run(&format!(
        "{ST} initial begin s='{{4'h3,8'h5A,1'b1}}; \
         $display(\"%h %h %b %h\",s.a,s.b,s.c,s); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("3 5a 1 06b5"), "got:\n{out}");
}

#[test]
fn decl_init_module_scope() {
    // `st_t z = '{…};` as a module item — the first field is the MSB.
    let (out, ok) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [7:0] b; } st_t;\n\
         st_t z = '{4'hA, 8'hBC};\n\
         initial begin $display(\"%h %h %h\", z.a, z.b, z); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("a bc abc"), "got:\n{out}");
}

#[test]
fn unsized_int_sizes_to_field() {
    // `'{3,200,0}` — each element sized to its FIELD width, not self-determined:
    // 200 = 0xC8 fills the 8-bit field b. (Plain `{3,200,0}` would be wrong.)
    let (out, ok) = run(&format!(
        "{ST} initial begin s='{{3,200,0}}; \
         $display(\"%h %h %b %h\",s.a,s.b,s.c,s); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("3 c8 0 0790"), "got:\n{out}");
}

#[test]
fn fill_elements_grow_to_field() {
    // `'1`/`'x`/`'z` each fill their field — exercises the #4.5.34 cast-fill fix.
    let (o1, k1) = run(&format!(
        "{ST} initial begin s='{{'1,'1,'1}}; \
         $display(\"%h %h %b %h\",s.a,s.b,s.c,s); #1 $finish; end endmodule\n"
    ));
    assert!(k1 && o1.contains("f ff 1 1fff"), "fill-1 got:\n{o1}");
    let (ox, kx) = run(&format!(
        "{ST} initial begin s='{{'x,'0,'1}}; \
         $display(\"%h %h %b %h\",s.a,s.b,s.c,s); #1 $finish; end endmodule\n"
    ));
    assert!(kx && ox.contains("x 00 1 xX01"), "fill-x got:\n{ox}");
}

#[test]
fn nonblocking_assign() {
    let (out, ok) = run(&format!(
        "{ST} initial begin s<='{{4'h7,8'h99,1'b0}}; \
         #1 $display(\"%h %h %b %h\",s.a,s.b,s.c,s); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("7 99 0 0f32"), "got:\n{out}");
}

#[test]
fn signed_member_sign_extends() {
    // A signed `byte` field: `8'(-3)` sign-extends to 0xFD within the field.
    let (out, ok) = run(
        "module top; typedef struct packed { byte sa; logic [3:0] b; } sg_t; sg_t g;\n\
         initial begin g='{-3,4'hF}; $display(\"%0d %h %h\",g.sa,g.b,g); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("-3 f fdf"), "got:\n{out}");
}

#[test]
fn variable_elements() {
    let (out, ok) = run(&format!(
        "{ST} logic [3:0] x; logic [7:0] y; \
         initial begin x=4'h9; y=8'hE1; s='{{x,y,1'b1}}; \
         $display(\"%h %h %b\",s.a,s.b,s.c); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("9 e1 1"), "got:\n{out}");
}

#[test]
fn single_field_struct() {
    let (out, ok) = run(
        "module top; typedef struct packed { logic [7:0] v; } o_t; o_t o;\n\
         initial begin o='{8'hC7}; $display(\"%h %h\",o.v,o); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("c7 c7"), "got:\n{out}");
}

#[test]
fn count_mismatch_too_few_is_loud() {
    let (_out, ok) = run(&format!(
        "{ST} initial begin s='{{4'h3,8'h5A}}; $display(\"%h\",s); #1 $finish; end endmodule\n"
    ));
    assert!(
        !ok,
        "a 2-element pattern for a 3-field struct must be a loud error"
    );
}

#[test]
fn count_mismatch_too_many_is_loud() {
    let (_out, ok) = run(&format!(
        "{ST} initial begin s='{{1,2,3,4}}; $display(\"%h\",s); #1 $finish; end endmodule\n"
    ));
    assert!(
        !ok,
        "a 4-element pattern for a 3-field struct must be a loud error"
    );
}

#[test]
fn two_state_field_coerces_x_z_to_zero() {
    // IEEE §6.11.3: a 2-state field (`byte`) coerces X/Z→0 on assignment. The
    // native `'{8'hxx,…}` does this; a plain bit-concat would not. vita squashes
    // via `longint'(e)` before sizing. The 4-state `logic` field keeps X/Z.
    let (ox, kx) = run(
        "module top; typedef struct packed { byte b; logic [7:0] l; } m_t; m_t s;\n\
         initial begin s='{8'hxx, 8'hAA}; $display(\"b=%h l=%h\", s.b, s.l); #1 $finish; end endmodule\n",
    );
    assert!(kx && ox.contains("b=00 l=aa"), "x-coerce got:\n{ox}");
    let (oz, kz) = run(
        "module top; typedef struct packed { byte b; logic [7:0] l; } m_t; m_t s;\n\
         initial begin s='{8'hzz, 8'hAA}; $display(\"b=%h l=%h\", s.b, s.l); #1 $finish; end endmodule\n",
    );
    assert!(kz && oz.contains("b=00 l=aa"), "z-coerce got:\n{oz}");
    // A non-X value through the same 2-state path is unchanged.
    let (on, kn) = run(
        "module top; typedef struct packed { byte b; logic [7:0] l; } m_t; m_t s;\n\
         initial begin s='{8'h3C, 8'hAA}; $display(\"b=%h l=%h\", s.b, s.l); #1 $finish; end endmodule\n",
    );
    assert!(kn && on.contains("b=3c l=aa"), "normal got:\n{on}");
}

#[test]
fn two_state_field_wider_than_64_is_loud() {
    // A 2-state field > 64 bits cannot be X/Z-squashed via `longint'` — honest-loud
    // rather than silent-wrong.
    let (_o, ok) = run(
        "module top; typedef struct packed { bit [99:0] big; logic [3:0] t; } w_t; w_t s;\n\
         initial begin s='{100'd5, 4'hF}; $display(\"x\"); #1 $finish; end endmodule\n",
    );
    assert!(
        !ok,
        "a 2-state field wider than 64 bits in a pattern must be loud"
    );
    // But a 4-STATE field > 64 bits is fine (no coercion needed).
    let (o4, ok4) = run(
        "module top; typedef struct packed { logic [99:0] big; logic [3:0] t; } w_t; w_t s;\n\
         initial begin s='{100'h123456789ABCDEF012345, 4'hF}; \
         $display(\"big=%h t=%h\", s.big, s.t); #1 $finish; end endmodule\n",
    );
    assert!(
        ok4 && o4.contains("big=0000123456789abcdef012345 t=f"),
        "4-state >64 field must work; got:\n{o4}"
    );
}

#[test]
fn packed_union_pattern_is_loud_not_silent_truncation() {
    // A packed union shares the layout map (for `u.field` reads) but its overlay
    // is NOT a packed concat — `'{…}` on a union must stay loud, not silently
    // concatenate-and-truncate (which would give a wrong value).
    let (_o, ok) = run(
        "module top; typedef union packed { logic [7:0] a; logic [3:0] b; } u_t; u_t x;\n\
         initial begin x='{8'hAB, 4'hC}; $display(\"%h\", x); #1 $finish; end endmodule\n",
    );
    assert!(
        !ok,
        "a packed-union `'{{…}}` pattern must be loud, not silent truncation"
    );
    // The union field READ path is unaffected (var_struct still populated).
    let (o2, ok2) = run(
        "module top; typedef union packed { logic [7:0] a; logic [3:0] b; } u_t; u_t x;\n\
         initial begin x=8'h5A; $display(\"a=%h b=%h\", x.a, x.b); #1 $finish; end endmodule\n",
    );
    assert!(ok2 && o2.contains("a=5a b=a"), "union read got:\n{o2}");
}

#[test]
fn four_state_field_keeps_x_only_two_state_coerces() {
    // The 2-state flag must track the RIGHT kinds: `integer`/`time` are 4-state
    // (keep X), `int`/`longint` are 2-state (coerce X→0). A confusion here would
    // wrongly coerce a 4-state field or leak X into a 2-state field.
    let (o, ok) = run(
        "module top; typedef struct packed { integer i4; int i2; } d_t; d_t s;\n\
         initial begin s='{32'hxxxxxxxx, 32'hxxxxxxxx}; \
         $display(\"i4=%h i2=%h\", s.i4, s.i2); #1 $finish; end endmodule\n",
    );
    assert!(
        ok && o.contains("i4=xxxxxxxx i2=00000000"),
        "integer keeps X, int coerces; got:\n{o}"
    );
}

#[test]
fn cross_module_same_name_union_then_struct() {
    // A union `foo` in one module and a struct `foo` in a later module (same source
    // unit): the struct's pattern must still desugar (struct_layouts is
    // last-writer-wins, and union_type_names is retracted to match). A stale union
    // entry would wrongly loud-reject the valid struct pattern.
    let (o, ok) = run(
        "module modA; typedef union packed { logic [15:0] all; logic [15:0] alt; } foo;\n\
         foo u; initial begin u.all=16'hDEAD; $display(\"A=%h\", u); end endmodule\n\
         module modB; typedef struct packed { logic [7:0] a; logic [7:0] b; } foo;\n\
         foo s; initial begin s='{8'hAB,8'hCD}; $display(\"B=%h\", s); end endmodule\n\
         module top; modA u0(); modB u1(); initial #1 $finish; endmodule\n",
    );
    assert!(
        ok && o.contains("A=dead") && o.contains("B=abcd"),
        "got:\n{o}"
    );
}

#[test]
fn array_of_struct_uses_array_path_not_struct_concat() {
    // `st_t arr[2]` is an ARRAY of structs, not a scalar struct, so `arr = '{…}`
    // must stay on the 1-D unpacked-array path (each element → arr[i]), NOT be
    // desugared to a packed-struct concat. Here arr[0]=8'h12, arr[1]=8'h34 — the
    // array semantics. A struct-concat misfire would give a different value (or a
    // width error). (iverilog cannot oracle array-of-struct — it aborts — so this
    // pins vita's §4.5.33 array-pattern behavior, confirming the guard.)
    let (out, ok) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;\n\
         st_t arr[2];\n\
         initial begin arr='{8'h12,8'h34}; \
         $display(\"arr0=%h arr1=%h\", arr[0], arr[1]); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("arr0=12 arr1=34"), "got:\n{out}");
}

const ST2: &str = "module top; typedef struct packed \
     { logic [3:0] a; logic [7:0] b; } st_t; st_t s;";

#[test]
fn continuous_assign_pattern() {
    // `assign s = '{…}` on a whole struct net (IEEE §10.9.1) — same desugar as a
    // procedural assign, applied at the continuous-assign parse site.
    let (out, ok) = run(&format!(
        "{ST2} assign s = '{{4'hA, 8'hBC}}; \
         initial begin #1 $display(\"%h %h %h\", s, s.a, s.b); #10 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("abc a bc"), "got:\n{out}");
}

#[test]
fn procedural_assign_pattern() {
    // Procedural continuous assign `assign s = '{…}` inside a block.
    let (out, ok) = run(&format!(
        "{ST2} initial begin assign s = '{{4'h7, 8'h89}}; \
         #1 $display(\"%h %h %h\", s, s.a, s.b); #10 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("789 7 89"), "got:\n{out}");
}

#[test]
fn force_pattern() {
    // `force s = '{…}` overrides with the pattern value.
    let (out, ok) = run(&format!(
        "{ST2} initial begin force s = '{{4'h5, 8'h67}}; \
         #1 $display(\"%h %h %h\", s, s.a, s.b); #10 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("567 5 67"), "got:\n{out}");
}

#[test]
fn continuous_assign_two_state_coerces() {
    // The 2-state coercion (`w'(longint'(e))`) is a plain expression, so it works
    // identically in a continuously-evaluated assign.
    let (out, ok) = run(
        "module top; typedef struct packed { byte b; logic [7:0] l; } m_t; m_t s;\n\
         assign s = '{8'hxx, 8'hAA};\n\
         initial begin #1 $display(\"b=%h l=%h\", s.b, s.l); #10 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("b=00 l=aa"), "got:\n{out}");
}

#[test]
fn continuous_assign_non_pattern_unchanged() {
    // A non-pattern continuous assign is byte-identical (the hook returns rhs).
    let (out, ok) = run(&format!(
        "{ST2} assign s = 12'hABC; \
         initial begin #1 $display(\"%h\", s); #10 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("abc"), "got:\n{out}");
}

#[test]
fn for_init_and_step_pattern() {
    // A struct pattern in both the for-init and the for-step clause (IEEE §10.9.1
    // works wherever an `lvalue = rhs` assignment does).
    let (out, ok) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t s;\n\
         initial begin\n\
           for (s = '{4'h0, 4'h1}; s.a < 3; s = '{s.a+4'h1, s.b}) $display(\"s=%h\", s);\n\
           #1 $finish;\n\
         end endmodule\n",
    );
    assert!(
        ok && out.contains("s=01") && out.contains("s=11") && out.contains("s=21"),
        "got:\n{out}"
    );
}

const STA: &str = "module top; typedef struct packed \
     { logic [3:0] a; logic [3:0] b; } st_t; st_t arr[3];";

#[test]
fn array_element_1d_pattern() {
    // `arr[i] = '{…}` on a 1-D struct array — the element is a scalar struct, so
    // it desugars to the same field-width-cast concat (IEEE §10.9.1).
    let (out, ok) = run(&format!(
        "{STA} initial begin arr[0]='{{4'h1,4'h2}}; arr[1]='{{4'h3,4'h4}}; arr[2]='{{4'h5,4'h6}}; \
         $display(\"%h %h %h\", arr[0], arr[1], arr[2]); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("12 34 56"), "got:\n{out}");
}

#[test]
fn array_element_variable_index() {
    // A runtime index in a loop.
    let (out, ok) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t arr[4];\n\
         integer i;\n\
         initial begin for (i=0;i<4;i=i+1) arr[i]='{i[3:0], i[3:0]+4'h1};\n\
           $display(\"%h %h %h %h\", arr[0],arr[1],arr[2],arr[3]); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("01 12 23 34"), "got:\n{out}");
}

#[test]
fn array_element_two_state_coerces() {
    // A 2-state `byte` field in an array element coerces X/Z→0 (verified via the
    // whole-element read, since array-of-struct field reads aren't oracle-backed).
    let (out, ok) = run(
        "module top; typedef struct packed { byte a; logic [7:0] b; } st_t; st_t arr[2];\n\
         initial begin arr[0]='{8'hxx,8'hAA}; $display(\"%h\", arr[0]); #1 $finish; end endmodule\n",
    );
    assert!(ok && out.contains("00aa"), "got:\n{out}");
}

#[test]
fn array_element_non_pattern_unchanged() {
    // A non-pattern array-element assign is byte-identical (the hook returns rhs).
    let (out, ok) = run(&format!(
        "{STA} initial begin arr[0]=8'h5A; arr[1]=8'h3C; \
         $display(\"%h %h\", arr[0], arr[1]); #1 $finish; end endmodule\n"
    ));
    assert!(ok && out.contains("5a 3c"), "got:\n{out}");
}

#[test]
fn multidim_and_scalar_bitselect_stay_loud() {
    // A multi-dim element `arr[i][j] = '{…}` (nested BitSelect base) stays loud —
    // only 1-D arrays are supported.
    let (_o1, ok1) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t arr[2][2];\n\
         initial begin arr[0][0]='{4'h1,4'h2}; $display(\"x\"); #1 $finish; end endmodule\n",
    );
    assert!(!ok1, "multi-dim array element must stay loud");
    // A scalar struct's bit-select `s[i] = '{…}` (not an array element) stays loud
    // — iverilog rejects it too.
    let (_o2, ok2) = run(
        "module top; typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t; st_t s;\n\
         initial begin s[3]='{4'h1,4'h2}; $display(\"x\"); #1 $finish; end endmodule\n",
    );
    assert!(!ok2, "scalar struct bit-select must stay loud");
}

#[test]
fn union_array_element_stays_loud() {
    // A union ARRAY element `arr[i] = '{…}` must stay loud (overlay != concat),
    // like the scalar union case.
    let (_o, ok) = run(
        "module top; typedef union packed { logic [7:0] a; logic [3:0] b; } u_t; u_t arr[2];\n\
         initial begin arr[0]='{8'hAB,4'hC}; $display(\"x\"); #1 $finish; end endmodule\n",
    );
    assert!(
        !ok,
        "union array element must stay loud, not silently truncate"
    );
}
