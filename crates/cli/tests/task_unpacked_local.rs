//! An unpacked-array local in a (non-automatic) task (`task t; int arr[0:3]; …`)
//! was a loud E3009 ("scalar/packed locals only") because the inline-task local
//! path allocated a single scalar net (which would silently corrupt element
//! read/write). It now allocates real element storage like a module-level array.
//! Oracle: iverilog -g2012. A non-automatic task's locals are STATIC, so the
//! array persists across calls (iverilog parity).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_tul_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W1017") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

#[test]
fn inline_task_unpacked_array_local() {
    // The task fills a local array and copies it out to module nets; the $display
    // runs in the caller (a $systask isn't part of the inline-task body subset).
    let out = run("module top;\n\
        integer o0,o1,o2,o3;\n\
        task t;\n\
          integer arr[0:3];\n\
          integer i;\n\
          for (i=0;i<4;i=i+1) arr[i]=i*2;\n\
          o0=arr[0]; o1=arr[1]; o2=arr[2]; o3=arr[3];\n\
        endtask\n\
        initial begin t; $display(\"%0d %0d %0d %0d\", o0,o1,o2,o3); end\n\
      endmodule\n");
    assert_eq!(out, "0 2 4 6\n");
}

#[test]
fn inline_task_unpacked_array_nonzero_base() {
    // A non-zero lower bound (`[1:3]`) must address elements correctly.
    let out = run("module top;\n\
        integer a,b,c;\n\
        task t;\n\
          integer m[1:3];\n\
          m[1]=11; m[2]=22; m[3]=33;\n\
          a=m[1]; b=m[2]; c=m[3];\n\
        endtask\n\
        initial begin t; $display(\"%0d %0d %0d\", a,b,c); end\n\
      endmodule\n");
    assert_eq!(out, "11 22 33\n");
}

#[test]
fn inline_task_unpacked_array_descending() {
    let out = run("module top;\n\
        integer a,b,c,d;\n\
        task t;\n\
          integer m[3:0];\n\
          m[0]=5; m[1]=6; m[2]=7; m[3]=8;\n\
          a=m[0]; b=m[1]; c=m[2]; d=m[3];\n\
        endtask\n\
        initial begin t; $display(\"%0d %0d %0d %0d\", a,b,c,d); end\n\
      endmodule\n");
    assert_eq!(out, "5 6 7 8\n");
}

#[test]
fn inline_task_unpacked_array_static_persistence() {
    // A non-automatic task's array is STATIC: a write in the first call is visible
    // in the second (iverilog parity).
    let out = run("module top;\n\
        integer r1,r2;\n\
        task t(input integer set, input integer val, output integer got);\n\
          integer m[0:1];\n\
          if (set) m[0]=val;\n\
          got=m[0];\n\
        endtask\n\
        initial begin t(1,99,r1); t(0,0,r2); $display(\"%0d %0d\", r1,r2); end\n\
      endmodule\n");
    assert_eq!(out, "99 99\n");
}
