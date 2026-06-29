//! A function with a user-defined type name as its return type — `function b_t f;`
//! where `b_t` is a typedef/enum/struct — was a parse error (E2002, "expected ';'
//! after function header"): the function-header parser only recognized built-in
//! return-type keywords. Fixed by adding a `<typedef_name> <function_name>` arm
//! that maps the typedef's resolved `TypeInfo` onto the return fields
//! (range/signed/ret_two_state/ret_type), mirroring the built-in arm. Pinned to
//! iverilog 13.0 across vector / signed / enum / struct / atom return types.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frt_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn return_logic_vector_typedef() {
    let o = run(
        "module top; typedef logic [7:0] b_t; function b_t f; f=8'hCD; endfunction\n\
         initial begin $display(\"%h\",f()); #1 $finish; end endmodule",
    );
    assert!(o.contains("cd"), "got:\n{o}");
}

#[test]
fn return_signed_vector_typedef() {
    let o = run(
        "module top; typedef logic signed [7:0] sb_t; function sb_t f; f=-3; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule",
    );
    assert!(o.contains("-3"), "got:\n{o}");
}

#[test]
fn return_enum_typedef() {
    // Enum with a packed base (2-bit) and without (int storage).
    let ob = run(
        "module top; typedef enum logic[1:0]{A,B,C} e; function e f; f=B; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule",
    );
    assert!(ob.contains('1'), "enum-base got:\n{ob}");
    let on = run(
        "module top; typedef enum {A,B,C} e; function e f; f=C; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule",
    );
    assert!(on.contains('2'), "enum-nobase got:\n{on}");
}

#[test]
fn return_struct_typedef() {
    let o = run(
        "module top; typedef struct packed {logic[3:0] a; logic[3:0] b;} s_t;\n\
         function s_t f; f=8'h35; endfunction\n\
         initial begin $display(\"%h\",f()); #1 $finish; end endmodule",
    );
    assert!(o.contains("35"), "got:\n{o}");
}

#[test]
fn return_atom_typedefs() {
    // byte (8-bit signed), int (32-bit), longint (64-bit).
    let ob = run(
        "module top; typedef byte b_t; function b_t f; f=-5; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule",
    );
    assert!(ob.contains("-5"), "byte got:\n{ob}");
    let oi = run(
        "module top; typedef int i_t; function i_t f; f=-1000000; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule",
    );
    assert!(oi.contains("-1000000"), "int got:\n{oi}");
    let ol = run(
        "module top; typedef longint l_t; function l_t f; f=64'hFFFFFFFFFF; endfunction\n\
         initial begin $display(\"%h\",f()); #1 $finish; end endmodule",
    );
    assert!(ol.contains("000000ffffffffff"), "longint got:\n{ol}");
}

#[test]
fn builtin_and_implicit_returns_unchanged() {
    // Byte-identity: built-in keyword return types and an implicit return are
    // unaffected by the new typedef arm.
    let ol = run("module top; function logic [7:0] f; f=8'hAB; endfunction\n\
         initial begin $display(\"%h\",f()); #1 $finish; end endmodule");
    assert!(ol.contains("ab"), "builtin-logic got:\n{ol}");
    let oi = run("module top; function int f; f=42; endfunction\n\
         initial begin $display(\"%0d\",f()); #1 $finish; end endmodule");
    assert!(oi.contains("42"), "builtin-int got:\n{oi}");
    let oimp = run("module top; function f; f=1'b1; endfunction\n\
         initial begin $display(\"%b\",f()); #1 $finish; end endmodule");
    assert!(oimp.contains('1'), "implicit got:\n{oimp}");
}

#[test]
fn multidim_packed_typedef_return_is_loud() {
    // A multi-dim packed typedef return (`typedef logic [3:0][7:0] m_t`) can't be
    // represented by the single-`range` return field, so it is loud-rejected rather
    // than silently returning only the first dimension's width (correct-or-loud).
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frt_loud_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top; typedef logic [3:0][7:0] m_t; function m_t f; f=32'h11223344; endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule",
    )
    .unwrap();
    let ok = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita")
        .status
        .success();
    assert!(!ok, "a multi-dim packed typedef return must be loud");
}
