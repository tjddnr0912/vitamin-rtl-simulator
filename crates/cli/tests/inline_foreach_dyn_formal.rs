//! §4.5.174: `foreach` over an `input` dynamic-array formal in a STATIC (inline,
//! non-`automatic`) task/function. The parser desugars `foreach (b[i])` uniformly to
//! `__st = b.first/next(__foreach_i)`; the elaborator rewrites it by the array's KIND.
//! The dyn/queue dense-walk dispatch resolved the array with `dyn_handle`, which misses
//! a `dyn_subst` formal alias (a read-only `input` dyn-array formal is not a real net) —
//! so `b.first(__i)` fell through to the generic method path → E3009. A `foreach` READS
//! the array, so it now resolves via `dyn_handle_read` (which consults the alias),
//! routing the formal through the same dense walk a module dyn-array uses. iverilog
//! agrees; the `automatic` (frame) path already worked. Every non-formal array (fixed /
//! module dyn / queue / assoc) is byte-identical (`dyn_subst` is empty outside the body).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifdf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── the gap: a STATIC (inline) task foreach-summing a dyn-array formal (was E3009) ──
#[test]
fn static_task_foreach_dyn_formal_sum() {
    let o = run("module top;\n\
         task sm(input byte b[]);\n\
           int s=0; foreach(b[i]) s+=b[i]; $display(\"s=%0d\", s);\n\
         endtask\n\
         byte a[]; initial begin a=new[3]; a[0]=10;a[1]=20;a[2]=30; sm(a); end\n\
         endmodule\n");
    assert!(!o.contains("E3009"), "must not reject:\n{o}");
    assert!(o.contains("s=60"), "iverilog: s=60:\n{o}");
}

// ── the foreach index AND element are both usable ──
#[test]
fn foreach_index_and_element() {
    let o = run("module top;\n\
         task sm(input int b[]); foreach(b[i]) $display(\"b[%0d]=%0d\", i, b[i]); endtask\n\
         int a[]; initial begin a=new[3]; a[0]=5;a[1]=6;a[2]=7; sm(a); end\n\
         endmodule\n");
    assert!(
        o.contains("b[0]=5") && o.contains("b[1]=6") && o.contains("b[2]=7"),
        "index+element foreach:\n{o}"
    );
}

// ── signed byte elements sum correctly through foreach (-5+10-2 = 3) ──
#[test]
fn foreach_signed_byte() {
    let o = run("module top;\n\
         task sm(input byte b[]); int s=0; foreach(b[i]) s+=b[i]; $display(\"s=%0d\",s); endtask\n\
         byte a[]; initial begin a=new[3]; a[0]=-5;a[1]=10;a[2]=-2; sm(a); end\n\
         endmodule\n");
    assert!(o.contains("s=3"), "signed foreach sum = 3:\n{o}");
}

// ── two foreach loops over the same formal in one task body ──
#[test]
fn two_foreach_same_formal() {
    let o = run("module top;\n\
         task sm(input int b[]);\n\
           int s=0, p=1;\n\
           foreach(b[i]) s+=b[i];\n\
           foreach(b[i]) p=p*b[i];\n\
           $display(\"s=%0d p=%0d\", s, p);\n\
         endtask\n\
         int a[]; initial begin a=new[3]; a[0]=1;a[1]=2;a[2]=3; sm(a); end\n\
         endmodule\n");
    assert!(o.contains("s=6 p=6"), "both foreach loops run:\n{o}");
}

// ── follow-on RESOLVED: a FUNCTION with a `foreach` over a dyn-array formal is FRAMED
//    (a `foreach` is control flow → not R2-inlinable). When this slice (§4.5.174) landed,
//    the framed-function dyn-formal path was a separate follow-on and stayed loud. It was
//    since completed: §4.5.177 (direct-rhs `r = f(a)`) + §4.5.179 (this buried `$display`
//    arg, hoisted to a temp). So the case below now RUNS (sum=15). ──
#[test]
fn function_foreach_dyn_formal_supported() {
    let o = run("module top;\n\
         function automatic int fsum(input int c[]);\n\
           int s=0; foreach(c[i]) s+=c[i]; return s;\n\
         endfunction\n\
         int a[]; initial begin a=new[3]; a[0]=4;a[1]=5;a[2]=6; $display(\"sum=%0d\", fsum(a)); end\n\
         endmodule\n");
    assert!(
        !o.contains("E3009") && o.contains("sum=15"),
        "framed function's dyn-array formal buried in $display (§4.5.177/178) = 15:\n{o}"
    );
}

// ── regression: a fixed-size unpacked array foreach is unchanged ──
#[test]
fn regression_fixed_array_foreach() {
    let o = run("module top;\n\
         int a[0:3];\n\
         initial begin\n\
           foreach(a[i]) a[i]=i*2;\n\
           begin int s=0; foreach(a[i]) s+=a[i]; $display(\"s=%0d\",s); end\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("s=12"),
        "fixed-array foreach unchanged (0+2+4+6=12):\n{o}"
    );
}

// ── regression: a module-level dynamic array foreach is unchanged ──
#[test]
fn regression_module_dyn_foreach() {
    let o = run("module top;\n\
         byte a[];\n\
         initial begin a=new[3]; a[0]=10;a[1]=20;a[2]=30;\n\
           begin int s=0; foreach(a[i]) s+=a[i]; $display(\"s=%0d\",s); end\n\
         end\n\
         endmodule\n");
    assert!(o.contains("s=60"), "module dyn foreach unchanged:\n{o}");
}

// ── regression: a queue foreach is unchanged ──
#[test]
fn regression_queue_foreach() {
    let o = run("module top;\n\
         int q[$];\n\
         initial begin q.push_back(3); q.push_back(4);\n\
           begin int s=0; foreach(q[i]) s+=q[i]; $display(\"s=%0d\",s); end\n\
         end\n\
         endmodule\n");
    assert!(o.contains("s=7"), "queue foreach unchanged:\n{o}");
}
