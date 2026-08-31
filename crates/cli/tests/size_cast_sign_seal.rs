//! A size cast's sign seal, across sign and width (R8, aes_top reviewer).
//!
//! `expr_cast` wraps every `n'(e)` result in `$signed`/`$unsigned` — a stamp, not
//! a computation — and the compiled lane declined the node, so a cast anywhere in
//! an expression sent the whole expression to the generic evaluator. The reviewer
//! took **14,616,553** of those in a workload whose source never writes `$signed`.
//! A 1.6M-iteration cast loop measured **0.678 s sealed vs 0.480 s unsealed**;
//! with the lane's seal arm the two are equal.
//!
//! The behaviour that must not move is which bits come out, and the trap is the
//! FILL: `$signed` sign-fills iff the CONTEXT is signed (the stamp is applied
//! before the resize), and `$unsigned` always zero-fills — neither reads the
//! operand's own sign. Widening, narrowing and equal width are all here, from
//! both signed and unsigned sources.
//!
//! Values pinned to iverilog 13.0 (a 768-cell sweep over the same axes agreed
//! cell for cell; this is the readable subset).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, backend: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_seal_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

const SRC: &str = "module t;\n\
   logic        [7:0] u = 8'hA5;   // unsigned source, MSB set\n\
   logic signed [7:0] s = 8'hA5;   // same bits, signed\n\
   logic signed [15:0] w_su;  logic [15:0] w_uu;\n\
   logic signed [15:0] w_ss;  logic [15:0] w_us;\n\
   logic        [3:0]  n_u;   logic signed [3:0] n_s;\n\
   logic signed [7:0]  e_s;   logic [7:0] e_u;\n\
   initial begin\n\
     w_su = 8'(s);  w_uu = 8'(u);  w_ss = 16'(s);  w_us = 16'(u);\n\
     n_u  = 4'(u);  n_s  = 4'(s);  e_s  = 8'(s);   e_u = 8'(u);\n\
     #1 $display(\"A=%h B=%h C=%h D=%h E=%h F=%h G=%h H=%h\",\n\
                 w_su, w_uu, w_ss, w_us, n_u, n_s, e_s, e_u);\n\
     $finish;\n\
   end\n\
 endmodule\n";

/// The values. `16'(s)` on a signed source sign-extends (`ffa5`); `16'(u)` on an
/// unsigned one zero-extends (`00a5`) — the seal's stamp, not the destination.
#[test]
fn the_seal_keeps_the_oracles_bits_across_sign_and_width() {
    let (o, ok) = run(SRC, "native");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("A=ffa5 B=00a5 C=ffa5 D=00a5 E=5 F=5 G=a5 H=a5"),
        "got:\n{o}"
    );
}

/// The VM and the interpreter never enter the compiled lane, so they are the
/// in-tree oracle for anything the lane newly admits.
#[test]
fn the_untouched_backends_agree_with_the_compiled_lane() {
    let line = |b: &str| {
        let (o, ok) = run(SRC, b);
        assert!(ok, "{b} failed:\n{o}");
        o.lines()
            .find(|l| l.starts_with("A="))
            .unwrap_or_default()
            .to_string()
    };
    let n = line("native");
    assert_eq!(n, line("vm"), "native vs vm");
    assert_eq!(n, line("interp"), "native vs interp");
}
