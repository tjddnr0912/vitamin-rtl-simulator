//! Loop control: `break` / `continue` (SV §11.5).
//!
//! vita parse-routed `break`/`continue` as undeclared task calls (E3010);
//! iverilog supports them. They desugar in the PARSER to the classic Verilog
//! `disable <label>` idiom: a loop that uses `continue` wraps its body in a
//! synthetic `begin : $continue$N … end` (continue → disable that block →
//! jumps to the loop's continue point), and a loop that uses `break` wraps the
//! whole loop in `begin : $break$N … end` (break → disable → jumps past the
//! loop). This reuses vita's existing disable→Goto lowering, so there is NO
//! AST/IR change (existing `Stmt::Disable`/`Stmt::Block` nodes). A loop with no
//! break/continue is left unwrapped ⇒ byte-identical. break/continue outside a
//! loop, or crossing a fork, is a loud error. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bc_{}_{n}", std::process::id()));
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
fn break_in_for() {
    // Sum 0..4 then break at i==5 → 0+1+2+3+4 = 10.
    let (out, code) = run("module top; integer i,s;\n\
         initial begin s=0; for(i=0;i<10;i=i+1) begin if(i==5) break; s=s+i; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=10"), "for+break; got:\n{out}");
}

#[test]
fn continue_in_for() {
    // Skip even i → 1+3+5 = 9.
    let (out, code) = run("module top; integer i,s;\n\
         initial begin s=0; for(i=0;i<6;i=i+1) begin if(i%2==0) continue; s=s+i; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=9"), "for+continue; got:\n{out}");
}

#[test]
fn break_continue_in_while() {
    let (out, code) = run("module top; integer i,s,t;\n\
         initial begin\n\
           s=0; i=0; while(i<100) begin if(i==7) break; s=s+i; i=i+1; end\n\
           t=0; i=0; while(i<6) begin i=i+1; if(i==3) continue; t=t+i; end\n\
           $display(\"R s=%0d t=%0d\",s,t); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    // break: 0+1+..+6 = 21; continue: 1+2+4+5+6 = 18.
    assert!(
        out.contains("R s=21 t=18"),
        "while break/continue; got:\n{out}"
    );
}

#[test]
fn break_in_forever_and_repeat() {
    let (out, code) = run("module top; integer i,s,r;\n\
         initial begin\n\
           s=0; i=0; forever begin if(i==5) break; s=s+i; i=i+1; end\n\
           r=0; i=0; repeat(10) begin if(i==4) break; r=r+i; i=i+1; end\n\
           $display(\"R s=%0d r=%0d\",s,r); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    // forever break: 0+1+2+3+4 = 10; repeat break: 0+1+2+3 = 6.
    assert!(
        out.contains("R s=10 r=6"),
        "forever/repeat break; got:\n{out}"
    );
}

#[test]
fn nested_break_exits_inner_only() {
    // Inner break stops j at 2; outer i still runs 0,1,2 → 3 iters × 2 = 6.
    let (out, code) = run("module top; integer i,j,s;\n\
         initial begin s=0; for(i=0;i<3;i=i+1) for(j=0;j<3;j=j+1) begin if(j==2) break; s=s+1; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R s=6"),
        "nested break inner-only; got:\n{out}"
    );
}

#[test]
fn continue_targets_innermost_loop() {
    // Outer continue at i==1 skips the inner loop that iteration: i=0 (+2), i=2 (+2) = 4.
    let (out, code) = run("module top; integer i,j,s;\n\
         initial begin s=0; for(i=0;i<3;i=i+1) begin if(i==1) continue; for(j=0;j<2;j=j+1) s=s+1; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=4"), "outer continue; got:\n{out}");
}

#[test]
fn break_in_foreach() {
    let (out, code) = run("module top; integer a[0:4]; integer s,k;\n\
         initial begin for(k=0;k<5;k=k+1) a[k]=k; s=0;\n\
           foreach(a[k]) begin if(a[k]==3) break; s=s+a[k]; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=3"), "foreach break; got:\n{out}"); // 0+1+2
}

#[test]
fn continue_in_typed_for() {
    // for(int i=0; …; i++) with continue — the typed-init rename must descend
    // through the synthetic $continue block.
    let (out, code) = run("module top; integer s;\n\
         initial begin s=0; for(int i=0;i<6;i++) begin if(i%2==0) continue; s=s+i; end\n\
           $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=9"), "typed-for continue; got:\n{out}");
}

#[test]
fn break_outside_loop_is_loud() {
    let (_o, code) = run("module top; initial begin break; end endmodule\n");
    assert_ne!(code, Some(0), "break outside a loop must be loud");
}

#[test]
fn continue_outside_loop_is_loud() {
    let (_o, code) = run("module top; initial begin if(1) continue; end endmodule\n");
    assert_ne!(code, Some(0), "continue outside a loop must be loud");
}

#[test]
fn break_continue_in_do_while() {
    // do-while is a loop too — break/continue must target IT, not an enclosing
    // loop (adversarial review caught this: a do-while nested in another loop
    // silently targeted the outer loop). Break at a==3 leaves a=3; continue skips
    // j==2 → 1+3+4+5 = 13 (separate vars so each result is observed).
    let (out, code) = run("module top; integer a,j,s;\n\
         initial begin\n\
           a=0; do begin if(a==3) break; a=a+1; end while(a<10);\n\
           s=0; j=0; do begin j=j+1; if(j==2) continue; s=s+j; end while(j<5);\n\
           $display(\"R a=%0d s=%0d\",a,s); #100 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R a=3 s=13"),
        "do-while break/continue; got:\n{out}"
    );
}

#[test]
fn do_while_in_outer_loop_targets_inner() {
    // The exact silent-wrong the review found: break in a do-while inside a while
    // must exit only the do-while; the outer while must still advance i to 3.
    let (out, code) = run("module top; integer i,j;\n\
         initial begin i=0;\n\
           while(i<3) begin j=0; do begin if(j==1) break; j=j+1; end while(j<5); i=i+1; end\n\
           $display(\"R i=%0d\",i); #200 $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R i=3"),
        "do-while break inside while; got:\n{out}"
    );
}

#[test]
fn plain_loop_unaffected() {
    // BYTE-IDENTITY sanity: a loop with NO break/continue runs exactly as before
    // (the parser leaves it unwrapped). Sum 0..4 = 10.
    let (out, code) = run("module top; integer i,s;\n\
         initial begin s=0; for(i=0;i<5;i=i+1) s=s+i; $display(\"R s=%0d\",s); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R s=10"), "plain loop; got:\n{out}");
}
