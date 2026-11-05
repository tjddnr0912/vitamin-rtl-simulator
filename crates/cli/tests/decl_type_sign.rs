//! A declaration's effective sign is not a function of its signing keyword and its
//! kind (§5.2 Q-DECLTYPE S12): `st v` writes no keyword, and `v` is signed because its
//! typedef is. `DataType::signed` carries the effective sign verbatim; a reader that
//! re-derived it from `DataType::signing()` and the kind would print `vlt=0 v=15`.
//!
//! iverilog 13.0, sv2v -> iverilog and verilator 5.052 all print `R vlt=1 wlt=1 v=-1`,
//! and so did vita before the record moved.
use std::process::Command;

#[test]
fn a_typedef_signed_variable_stays_signed_without_a_keyword() {
    let src = "module top;\n\
               typedef logic signed [3:0] st;\n\
               st v = 4'hF;\n\
               logic signed [3:0] w = 4'hF;\n\
               initial begin #1 $display(\"R vlt=%0d wlt=%0d v=%0d\", v < 0, w < 0, v); $finish; end\n\
               endmodule\n";
    let path = std::env::temp_dir().join(format!("vita_decl_type_sign_{}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let r: Vec<_> = s.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(r, ["R vlt=1 wlt=1 v=-1"], "{s}");
}

/// A function/task body-local of an unpacked record whose members are all 2-state
/// (`r_t p;`, lowered to one packed-vector local) is declared `bit`, so it reads 0
/// before any write and a write of `x` / `z` stores 0 (§4.5.192 ; S12 kept the kind,
/// lens S-F2 pins it).  Declared `logic` instead, `f` would read x and `h` / `k` would
/// keep the x / z they were handed.
///
/// verilator 5.052 prints `f=00000` (read-before-write variant) ; iverilog 13.0 refuses
/// the unpacked struct (`sorry: Unpacked structs not supported`).
#[test]
fn an_all_two_state_record_function_local_is_declared_bit() {
    let src = "module top;\n\
               typedef struct { bit [3:0] a; bit b; } r_t;\n\
               function automatic [4:0] f();\n\
               r_t p;\n\
               return {p.a, p.b};\n\
               endfunction\n\
               function automatic [4:0] h(input logic [3:0] v);\n\
               r_t p;\n\
               p.a = v; p.b = 1'bz;\n\
               return {p.a, p.b};\n\
               endfunction\n\
               function automatic [4:0] k(input logic [3:0] v);\n\
               r_t p;\n\
               p.a = v;\n\
               return {p.a + 4'd1, p.b};\n\
               endfunction\n\
               initial begin #1 $display(\"R f=%b h=%b k=%b\", f(), h(4'bx01z), k(4'bxxxx)); $finish; end\n\
               endmodule\n";
    let path = std::env::temp_dir().join(format!("vita_decl_type_k2_{}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let r: Vec<_> = s.lines().filter(|l| l.starts_with("R ")).collect();
    assert_eq!(r, ["R f=00000 h=00100 k=00010"], "{s}");
}
