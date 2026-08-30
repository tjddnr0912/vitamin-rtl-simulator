//! The dirty-net set is a two-level bitmap (`native/dirty.rs::DirtyBits`), and
//! its SECOND summary word is a path no workload in the corpus reaches: the
//! largest pinned design (biriscv) has 1611 nets, and one summary word covers
//! 4096. This builds a design past that boundary and pins the value.
//!
//! What it is really asserting is the property that let the per-delta
//! `sort_unstable()` be deleted: the drain visits members in ascending net
//! order at all three levels (summary word, bit within it, bit within the data
//! word), so wake order — and therefore every `$display` interleaving — is the
//! same order the sort used to reconstruct. A design this wide gets that wrong
//! the moment the summary walk is not index-ordered.
//!
//! Values pinned to iverilog 13.0.
use std::process::Command;

fn run(src: &str, args: &[&str]) -> (String, bool) {
    let d = std::env::temp_dir().join(format!(
        "vita_dirtyscale_{}_{}",
        std::process::id(),
        args.join("_")
    ));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.v");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

/// A chain of `n` continuous assigns, so the design has more than `n` nets and
/// more than `n` cont-assigns — both sets cross the 4096 boundary at once.
fn wide_design(n: usize) -> String {
    let mut s = String::from("module big;\n  reg clk = 0;\n  always #1 clk = ~clk;\n");
    for i in 0..n {
        s.push_str(&format!("  wire w{i};\n"));
    }
    s.push_str("  reg [31:0] seed = 32'h1;\n  assign w0 = seed[0];\n");
    for i in 1..n {
        s.push_str(&format!("  assign w{i} = w{} ^ seed[{}];\n", i - 1, i % 32));
    }
    s.push_str(&format!(
        "  integer c = 0;\n  reg [63:0] acc = 0;\n\
         \x20 always @(posedge clk) begin\n\
         \x20   seed <= {{seed[30:0], seed[31]^seed[21]^seed[1]^seed[0]}};\n\
         \x20   acc <= acc ^ {{w{}, w4097, w4096, w4095, w63, w64, w0}} ^ {{32'd0, seed}};\n\
         \x20   c = c + 1;\n\
         \x20   if (c > 200) begin $display(\"ACC=%h C=%0d\", acc, c); $finish; end\n\
         \x20 end\n\
         endmodule\n",
        n - 1
    ));
    s
}

/// ⭐ 4200 nets ⇒ `summary.len() == 2`. The value is iverilog's.
#[test]
fn a_design_past_the_second_summary_word_matches_the_oracle() {
    let (o, ok) = run(&wide_design(4200), &[]);
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("ACC=000000008507cb38 C=201"),
        "got:\n{}",
        o.lines()
            .filter(|l| l.starts_with("ACC="))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The same design on the VM backend, which this slice did not touch: it is the
/// in-tree oracle for the compiled path's ordering, and a divergence here is a
/// divergence the corpus digests cannot see.
#[test]
fn the_untouched_vm_backend_agrees_past_the_boundary() {
    let (native, ok1) = run(&wide_design(4200), &["--backend", "native"]);
    let (vm, ok2) = run(&wide_design(4200), &["--backend", "vm"]);
    assert!(
        ok1 && ok2,
        "a backend failed:\nnative:\n{native}\nvm:\n{vm}"
    );
    assert_eq!(
        native
            .lines()
            .filter(|l| l.starts_with("ACC="))
            .collect::<Vec<_>>(),
        vm.lines()
            .filter(|l| l.starts_with("ACC="))
            .collect::<Vec<_>>()
    );
}
