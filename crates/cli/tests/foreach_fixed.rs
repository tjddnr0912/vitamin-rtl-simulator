//! A2 (Tier-ⓐ close): `foreach` over a FIXED-SIZE unpacked array. The parser
//! desugars `foreach (a[i])` uniformly to `__st = a.first/next(__i)`; for a
//! static array there is no `.first/.next`, so elaborate synthesizes a plain
//! index walk over the first dim `[lo, lo+size)`. Previously this hit E3009.
//! IR-0 (elaborate-only). Oracle = iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fef_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn foreach_fixed_ascending_write_then_read() {
    // int a[0:7]: write a[i]=i*10, then sum every element via a second foreach.
    let (out, err, code) = run("module top;\n\
         integer a[0:7];\n\
         integer sum;\n\
         initial begin\n\
           foreach (a[i]) a[i] = i * 10;\n\
           sum = 0;\n\
           foreach (a[i]) sum = sum + a[i];\n\
           $display(\"a0=%0d a7=%0d sum=%0d\", a[0], a[7], sum);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a0=0 a7=70 sum=280"), "got:\n{out}");
}

#[test]
fn foreach_fixed_descending_range_visits_all() {
    // int a[7:0]: descending decl. foreach must visit every element index 0..7.
    let (out, err, code) = run("module top;\n\
         integer a[7:0];\n\
         integer sum;\n\
         initial begin\n\
           foreach (a[i]) a[i] = i;\n\
           sum = 0;\n\
           foreach (a[i]) sum = sum + a[i];\n\
           $display(\"a0=%0d a3=%0d a7=%0d sum=%0d\", a[0], a[3], a[7], sum);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a0=0 a3=3 a7=7 sum=28"), "got:\n{out}");
}

#[test]
fn foreach_fixed_reg_array_nonzero_lo() {
    // reg [7:0] a[2:5]: lo=2, size=4. The index walks 2..5 (NOT 0..3).
    let (out, err, code) = run("module top;\n\
         reg [7:0] a[2:5];\n\
         integer sum;\n\
         initial begin\n\
           foreach (a[i]) a[i] = i + 1;\n\
           sum = 0;\n\
           foreach (a[i]) sum = sum + a[i];\n\
           $display(\"a2=%0d a5=%0d sum=%0d\", a[2], a[5], sum);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // a[2]=3, a[3]=4, a[4]=5, a[5]=6 -> sum 18.
    assert!(out.contains("a2=3 a5=6 sum=18"), "got:\n{out}");
}

#[test]
fn foreach_fixed_descending_iterates_in_declaration_order() {
    // int b[3:0]: IEEE §12.7.3 traverses the declared bounds left-to-right, so the
    // index goes i = 3,2,1,0 (NOT 0,1,2,3). An order-sensitive body must observe it.
    let (out, err, code) = run("module top;\n\
         int b[3:0];\n\
         integer acc;\n\
         initial begin\n\
           acc = 0;\n\
           foreach (b[i]) acc = acc*10 + i;\n\
           $display(\"acc=%0d\", acc);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("acc=3210"), "got:\n{out}");
}

#[test]
fn foreach_fixed_descending_nonzero_lo_order() {
    // reg [7:0] d[5:2]: descending, lo=2 hi=5 -> i = 5,4,3,2.
    let (out, err, code) = run("module top;\n\
         reg [7:0] d[5:2];\n\
         integer acc;\n\
         initial begin\n\
           acc = 0;\n\
           foreach (d[i]) acc = acc*10 + i;\n\
           $display(\"acc=%0d\", acc);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("acc=5432"), "got:\n{out}");
}

#[test]
fn foreach_fixed_single_element() {
    // a[0:0]: degenerate one-element array still iterates exactly once.
    let (out, err, code) = run("module top;\n\
         integer a[0:0];\n\
         integer n;\n\
         initial begin\n\
           n = 0;\n\
           foreach (a[i]) begin a[i] = 99; n = n + 1; end\n\
           $display(\"a0=%0d n=%0d\", a[0], n);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("a0=99 n=1"), "got:\n{out}");
}
