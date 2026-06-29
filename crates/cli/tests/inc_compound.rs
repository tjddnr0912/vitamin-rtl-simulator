//! Increment/decrement (`i++` `++i` `i--` `--i`) and compound assignment
//! (`+= -= *= /= %= &= |= ^= <<= >>= <<<= >>>=`) — SV §11.4.1/§11.4.2.
//!
//! vita PARSE-rejected these ("expected '=' or '<=' after lvalue"); iverilog
//! accepts them. As a STATEMENT (or for-loop init/step) they are pure shorthand:
//! `i += e` ≡ `i = i + e`, `i++` ≡ `i = i + 1` (the pre/post distinction is
//! invisible when the value is discarded). The parser desugars each to the
//! existing `Stmt::Blocking` (no AST/IR change). Expression-embedded forms
//! (`a = i++`, with side-effect ordering) are NOT supported → loud parse error
//! (correct-or-loud). Purely additive (rejected before) ⇒ byte-identical.
//! Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inc_{}_{n}", std::process::id()));
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
fn compound_all_ops() {
    // All 12 compound operators, oracle-pinned to iverilog.
    let (out, code) = run("module top; integer a,b,c,d,e,f,g,h,i,j;\n\
         initial begin\n\
           a=20; a+=3; b=20; b-=3; c=20; c*=3; d=20; d/=3; e=20; e%=3;\n\
           f=12; f&=10; g=12; g|=10; h=12; h^=10; i=1; i<<=3; j=64; j>>=2;\n\
           $display(\"R %0d %0d %0d %0d %0d %0d %0d %0d %0d %0d\",a,b,c,d,e,f,g,h,i,j);\n\
           $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    // +=23 -=17 *=60 /=6 %=2 &=8 |=14 ^=6 <<=8 >>=16
    assert!(
        out.contains("R 23 17 60 6 2 8 14 6 8 16"),
        "compound ops; got:\n{out}"
    );
}

#[test]
fn arithmetic_shift_assign() {
    // `>>>=` is the ARITHMETIC right shift (sign-extending): -16 >>> 2 = -4.
    let (out, code) = run("module top; integer x;\n\
         initial begin x=-16; x>>>=2; $display(\"R x=%0d\",x); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R x=-4"), "arithmetic >>>=; got:\n{out}");
}

#[test]
fn increment_decrement_pre_post() {
    // i++ / ++i / i-- / --i as statements all land on i±1 (value discarded).
    let (out, code) = run("module top; integer p,q,r,s;\n\
         initial begin\n\
           p=5; p++; q=5; ++q; r=5; r--; s=5; --s;\n\
           $display(\"R %0d %0d %0d %0d\",p,q,r,s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R 6 6 4 4"), "inc/dec; got:\n{out}");
}

#[test]
fn for_loop_step_forms() {
    // for-step accepts i++, ++i, and i+=2.
    let (out, code) = run("module top; integer i,s1,s2,s3;\n\
         initial begin\n\
           s1=0; for(i=0;i<5;i++) s1+=i;\n\
           s2=0; for(i=0;i<5;++i) s2+=i;\n\
           s3=0; for(i=0;i<10;i+=2) s3++;\n\
           $display(\"R %0d %0d %0d\",s1,s2,s3); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R 10 10 5"), "for-step forms; got:\n{out}");
}

#[test]
fn compound_on_array_lvalue() {
    // A compound op on a non-trivial lvalue (`m[2] += 7; m[2]++`) reads and writes
    // the same element. 0 + 7 + 1 = 8.
    let (out, code) = run("module top; integer m[0:3];\n\
         initial begin m[0]=0;m[1]=0;m[2]=0;m[3]=0; m[2]+=7; m[2]++;\n\
           $display(\"R m2=%0d\",m[2]); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R m2=8"), "array-lvalue compound; got:\n{out}");
}

#[test]
fn increment_wraps_at_width() {
    // `++` desugars to `= +1` and inherits the assignment's width truncation:
    // 8'hFE++ = FF, 4'hF++ = 0 (wrap).
    let (out, code) = run("module top; logic [7:0] v; bit [3:0] b;\n\
         initial begin v=8'hFE; v++; b=4'hF; b++; $display(\"R v=%h b=%h\",v,b); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R v=ff b=0"), "width wrap; got:\n{out}");
}

#[test]
fn expression_embedded_increment_is_loud() {
    // `a = i++` has side-effect ordering semantics vita does not implement — it
    // must stay a LOUD parse error, never silently mis-evaluate (correct-or-loud).
    let (_out, code) = run("module top; integer i,a;\n\
         initial begin i=5; a=i++; $display(\"R a=%0d\",a); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "a=i++ must be loud, not silently accepted");
}

#[test]
fn compound_desugars_like_explicit_assign() {
    // Strongest guard (vita-internal differential, no oracle dependence): a
    // compound assignment on a non-trivial lvalue must produce BYTE-IDENTICAL
    // output to the explicit `lvalue = lvalue <op> e` it desugars to. Locks the
    // desugar-exactness for part-select + bit-select lvalues.
    let body = "\n\
         initial begin v=16'h00FF; %FORM%; $display(\"R v=%h\", v); $finish; end endmodule\n";
    let compound = "module top; logic [15:0] v;".to_string()
        + &body.replace("%FORM%", "v[11:4] += 8'h10; v[0] ^= 1'b1");
    let explicit = "module top; logic [15:0] v;".to_string()
        + &body.replace("%FORM%", "v[11:4] = v[11:4] + 8'h10; v[0] = v[0] ^ 1'b1");
    let (co, c1) = run(&compound);
    let (ex, c2) = run(&explicit);
    assert_eq!(c1, Some(0));
    assert_eq!(c2, Some(0));
    assert_eq!(co, ex, "compound must equal explicit form");
}

#[test]
fn compound_or_does_not_break_sva_implication() {
    // BYTE-IDENTITY GUARD: the new `|=` token must not steal the SVA `|=>` /
    // `|->` implication operators (logos longest-match). This design uses `|=>`;
    // it must still parse and run (vita-internal — iverilog rejects SVA). A clean
    // exit 0 with the marker proves the implication operator still lexes.
    let (out, code) = run("module top; reg clk,a,b;\n\
         initial begin clk=0;a=0;b=0; #1 a=1; #1 b=1; #10 $display(\"R ok\"); $finish; end\n\
         always #1 clk=~clk;\n\
         assert property (@(posedge clk) a |=> b);\n\
         endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R ok"), "SVA |=> still parses; got:\n{out}");
}
