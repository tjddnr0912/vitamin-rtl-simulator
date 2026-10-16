//! §5.2 row 27: `$readmemh` / `$readmemb` into an associative array, local or
//! hierarchical — the VeeR EH1 / EL2 testbench idiom
//! `$readmemh("program.hex", lmem.mem)` with `bit [7:0] mem [bit [31:0]]`
//! (`tb_top.sv:422–423` EH1, `:2552–2553` EL2). Each address the file supplies is
//! one entry (IEEE 1800-2017 §21.4); entries the file does not name keep their value.
//!
//! Oracles. Icarus Verilog 13.0 cannot declare an associative array with a type
//! key (`Type names are not valid expressions here.`) and sv2v 0.0.13 has none
//! (`Parse error: unexpected token ']'`). Verilator 5.052 (`--binary`) is the one
//! tool; its outputs are quoted beside each test. Where it splits from the LRM it is
//! disqualified by its own FIXED-array twin, measured against iverilog 13 (the
//! oracle vita's fixed-memory loader is pinned to):
//!
//! * verilator ignores the FINISH address — for a fixed array too
//!   (`$readmemh(f, m, 4, 5)` with three words loads `m[6]`; iverilog: `Too many words
//!   in the file for the requested range [4:5].`, `m[6]=00`) — and ignores a finish
//!   below the start (loads upward; iverilog loads 6, 5, 4);
//! * verilator loads an `@` address outside the start/finish window (iverilog:
//!   `address (0x2) is out of range [0x4:0x6]`, nothing loaded; §21.4 "the load
//!   operation is terminated");
//! * verilator is 2-state: an x / z digit into a 4-state element reads 0 (§21.4
//!   loads x and z digits as such).
//!
//! An address that is not a value of the key type has no single reading (verilator
//! truncates it to the key: `@1fe` into `[bit [7:0]]` is key `fe`, the address after
//! `@ff` wraps to 0, `@80` into `[byte]` is -128; §21.4 stops at "the highest address
//! in the memory"; a keyword key's index is read uncast), and neither has a word
//! before any address into a SIGNED key with no start argument (§21.4's default start
//! is "the lowest address in the memory": the key type's minimum, or verilator's 0).
//! Both are refused at run time by name (W4023, the load stops there).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` with `files` beside it; `backend` adds `--backend <b>`.
fn run_with(src: &str, files: &[(&str, &str)], backend: Option<&str>) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rma_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    for (name, text) in files {
        std::fs::write(d.join(name), text).unwrap();
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    if let Some(b) = backend {
        cmd.args(["--backend", b]);
    }
    let out = cmd.arg("t.sv").current_dir(&d).output().expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn run(src: &str, files: &[(&str, &str)]) -> (String, String, i32) {
    run_with(src, files, None)
}

/// The program's own lines (no `simulation ended` / summary lines).
fn lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
        .collect()
}

/// A module declaring `decl`, running `call`, then walking the array in key order
/// with an iteration variable of type `kt` and printing `num()`.
fn walk(decl: &str, kt: &str, call: &str) -> String {
    format!(
        "module top;
  {decl}
  initial begin
    {call}
    begin {kt} k; int st; st = mem.first(k);
      while (st != 0) begin $display(\"k=%0h v=%h\", k, mem[k]); st = mem.next(k); end
    end
    $display(\"num=%0d\", mem.num());
    $finish;
  end
endmodule
"
    )
}

/// The value the walk printed, the run clean (exit 0, no W4023).
fn loads(decl: &str, kt: &str, call: &str, files: &[(&str, &str)], want: &[&str]) {
    let (out, err, code) = run(&walk(decl, kt, call), files);
    assert_eq!(code, 0, "{err}");
    assert!(!err.contains("W4023"), "unexpected W4023:\n{err}");
    assert_eq!(lines(&out), want, "stderr:\n{err}");
}

/// The load stops with a W4023 naming `needle`; the walk prints `want`.
fn stops(decl: &str, kt: &str, call: &str, files: &[(&str, &str)], needle: &str, want: &[&str]) {
    let (out, err, code) = run(&walk(decl, kt, call), files);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains("VITA-W4023") && err.contains(needle),
        "expected a W4023 naming `{needle}`:\n{err}"
    );
    assert_eq!(lines(&out), want, "stderr:\n{err}");
}

const INT: &str = "bit [7:0] mem [int];";
const B32: &str = "bit [7:0] mem [bit [31:0]];";

/// The row-9 repro (`r9__B-veer__repros__readmemh_assoc_local__t`), verbatim.
/// verilator: `b0=5a b1=a5 n=2`. PRE: E3009 `a dynamic-storage handle has no
/// whole-value surface`.
#[test]
fn row9_local_repro() {
    let (out, err, code) = run(
        "module top;
  bit [7:0] mem [int];                 // associative byte memory
  initial begin
    $readmemh(\"p.hex\", mem);           // $readmemh into an associative array (IEEE 1800-2017 21.4.1)
    $display(\"b0=%h b1=%h n=%0d\", mem[32'h10], mem[32'h11], mem.num());
    $finish;
  end
endmodule
",
        &[("p.hex", "@10\n5a a5\n")],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["b0=5a b1=a5 n=2"]);
}

/// The hierarchical row-9 repro (`…readmemh_hier_assoc__t`). verilator `b0=5a b1=a5`.
/// PRE: E3009 `hierarchical read of lmem.mem is unsupported`.
#[test]
fn row9_hierarchical_repro() {
    let (out, err, code) = run(
        "module memm;
  bit [7:0] mem [int];
endmodule
module top;
  memm lmem ();
  initial begin
    $readmemh(\"p.hex\", lmem.mem);
    $display(\"b0=%h b1=%h\", lmem.mem[32'h10], lmem.mem[32'h11]);
    $finish;
  end
endmodule
",
        &[("p.hex", "@10\n5a a5\n")],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(lines(&out), ["b0=5a b1=a5"]);
}

/// The VeeR tb idiom, first-party: two `ahb_sif`-shaped children with
/// `bit [7:0] mem [bit [31:0]]`, one hex file loaded into both, read back as
/// little-endian words the way `preload_dccm` does (`addr = 'hffff_fff8`), plus
/// each child's own `num` / `exists`. verilator 5.052, verbatim:
/// `saddr=f0000000`, `80000000: d0580137`, `80000004: 100001b7`,
/// `80000008: 00010113`, `8000000c: 00000000`, `tail=deadbeef`,
/// `top.imem n=20 e=1 1`, `top.lmem n=20 e=1 1` (the two `%m` lines in either
/// order: they run in one time slot).
#[test]
fn veer_tb_idiom_two_instances() {
    let src = "module ahb_sif;
  bit [7:0] mem [bit [31:0]];
  initial begin #1 $display(\"%m n=%0d e=%0d %0d\", mem.num(), mem.exists(32'h80000003), mem.exists(32'h80000004)); end
endmodule
module top;
  ahb_sif lmem ();
  ahb_sif imem ();
  bit [31:0] addr, saddr, w;
  initial begin
    $readmemh(\"program.hex\",  lmem.mem);
    $readmemh(\"program.hex\",  imem.mem);
    addr = 'hffff_fff8;
    saddr = {lmem.mem[addr+3], lmem.mem[addr+2], lmem.mem[addr+1], lmem.mem[addr]};
    $display(\"saddr=%h\", saddr);
    for (addr = 32'h80000000; addr < 32'h80000010; addr += 4) begin
      w = {imem.mem[addr+3], imem.mem[addr+2], imem.mem[addr+1], imem.mem[addr]};
      $display(\"%h: %h\", addr, w);
    end
    $display(\"tail=%h\", {lmem.mem[32'hd0580003], lmem.mem[32'hd0580002], lmem.mem[32'hd0580001], lmem.mem[32'hd0580000]});
    #2 $finish;
  end
endmodule
";
    let hex = "@80000000\n37 01 58 D0 B7 01 00 10\n13 01 01 00\n@D0580000\nEF BE AD DE\n@FFFFFFF8\n00 00 00 F0\n";
    let want = [
        "saddr=f0000000",
        "80000000: d0580137",
        "80000004: 100001b7",
        "80000008: 00010113",
        "8000000c: 00000000",
        "tail=deadbeef",
        "top.lmem n=20 e=1 1",
        "top.imem n=20 e=1 1",
    ];
    let mut first = None;
    for b in [None, Some("native"), Some("interp"), Some("vm")] {
        let (out, err, code) = run_with(src, &[("program.hex", hex)], b);
        assert_eq!(code, 0, "{b:?}: {err}");
        let mut got = lines(&out);
        let mut exp = want.to_vec();
        got.sort_unstable();
        exp.sort_unstable();
        assert_eq!(got, exp, "{b:?}");
        // The three executors agree byte for byte.
        match &first {
            None => first = Some(out),
            Some(f) => assert_eq!(f, &out, "{b:?}"),
        }
    }
}

/// Typed `[bit [31:0]]` key, addresses at and above 2^31 up to 2^32-1, with a gap:
/// verilator `k=80000000 v=5a`, `k=80000001 v=a5`, `k=ffffffff v=77`, `num=3`.
/// No `@` and no start into an UNSIGNED key starts at 0 (its lowest value;
/// verilator `k=0 v=01`, `k=1 v=02`).
#[test]
fn typed_key_high_and_sparse_addresses() {
    loads(
        B32,
        "bit [31:0]",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@80000000 5a a5\n@ffffffff 77\n")],
        &[
            "k=80000000 v=5a",
            "k=80000001 v=a5",
            "k=ffffffff v=77",
            "num=3",
        ],
    );
    loads(
        B32,
        "bit [31:0]",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "01 02\n")],
        &["k=0 v=01", "k=1 v=02", "num=2"],
    );
    // `[int unsigned]`. verilator `k=80000000 v=5a`, `k=ffffffff v=77`.
    loads(
        "bit [7:0] mem [int unsigned];",
        "int unsigned",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@80000000 5a @ffffffff 77\n")],
        &["k=80000000 v=5a", "k=ffffffff v=77", "num=2"],
    );
}

/// Entries already present are kept unless the file names their address, which it
/// overwrites; a second load overwrites the first. verilator `k=0 v=11`,
/// `k=80000000 v=aa`, `k=80000001 v=bb`, `k=80000005 v=22`, `num=4`;
/// `k=0 v=aa`, `k=1 v=11`, `k=2 v=cc`, `num=3`.
#[test]
fn existing_entries_kept_named_ones_overwritten() {
    loads(
        B32,
        "bit [31:0]",
        "mem[0] = 8'h11; mem[32'h80000005] = 8'h22; mem[32'h80000000] = 8'h33;
    $readmemh(\"p.hex\", mem);",
        &[("p.hex", "@80000000 aa bb\n")],
        &[
            "k=0 v=11",
            "k=80000000 v=aa",
            "k=80000001 v=bb",
            "k=80000005 v=22",
            "num=4",
        ],
    );
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem); $readmemh(\"q.hex\", mem);",
        &[("p.hex", "@0 aa bb cc\n"), ("q.hex", "@1 11\n")],
        &["k=0 v=aa", "k=1 v=11", "k=2 v=cc", "num=3"],
    );
}

/// Keyword keys within their range: `[int]`, `[shortint]`, `[time]`, `[integer]`.
/// verilator `k=7ffffffe v=11`, `k=7fffffff v=5a`; `k=10 v=33`, `k=7ffe v=11`,
/// `k=7fff v=22`; `k=10 v=5a`, `k=100000000 v=77`; `[integer]` with a 4-state
/// element and an `x1` word: verilator `k=11 v=01` (2-state), hand-IEEE §21.4 `x1`.
#[test]
fn keyword_keys_in_range() {
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@7ffffffe 11 5a\n")],
        &["k=7ffffffe v=11", "k=7fffffff v=5a", "num=2"],
    );
    loads(
        "bit [7:0] mem [shortint];",
        "shortint",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@7ffe 11 22 @10 33\n")],
        &["k=10 v=33", "k=7ffe v=11", "k=7fff v=22", "num=3"],
    );
    loads(
        "bit [7:0] mem [time];",
        "time",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@10 5a @100000000 77\n")],
        &["k=10 v=5a", "k=100000000 v=77", "num=2"],
    );
    loads(
        "logic [7:0] mem [integer];",
        "integer",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@10 5a x1\n")],
        &["k=10 v=5a", "k=11 v=x1", "num=2"],
    );
}

/// A start argument, and a NEGATIVE one into a signed key. verilator `k=4 v=11`,
/// `k=5 v=22`, `k=6 v=33`; `[int]` from -2: `k=0 v=33`, `k=fffffffe v=11`,
/// `k=ffffffff v=22` (its walk is unsigned; §7.8.4 walks a signed key in signed
/// order); `[logic signed [15:0]]` from -2: `k=fffe v=11`, `k=ffff v=22`, `k=0 v=33`.
#[test]
fn start_address_and_negative_start() {
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 4);",
        &[("p.hex", "11 22 33\n")],
        &["k=4 v=11", "k=5 v=22", "k=6 v=33", "num=3"],
    );
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, -2);",
        &[("p.hex", "11 22 33\n")],
        &["k=fffffffe v=11", "k=ffffffff v=22", "k=0 v=33", "num=3"],
    );
    loads(
        "bit [7:0] mem [logic signed [15:0]];",
        "logic signed [15:0]",
        "$readmemh(\"p.hex\", mem, -2);",
        &[("p.hex", "11 22 33\n")],
        &["k=fffe v=11", "k=ffff v=22", "k=0 v=33", "num=3"],
    );
}

/// The start / finish window is the fixed-memory loader's (iverilog-pinned; see the
/// module doc for verilator's split): a finish below the start loads downward
/// (iverilog fixed twin `m[4]=33 m[5]=22 m[6]=11`), a word past the finish stops
/// the load, an `@` outside the window stops it, and a file shorter than an explicit
/// window warns and leaves the rest alone.
#[test]
fn start_finish_window() {
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 6, 4);",
        &[("p.hex", "11 22 33\n")],
        &["k=4 v=33", "k=5 v=22", "k=6 v=11", "num=3"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 4, 5);",
        &[("p.hex", "11 22 33\n")],
        "address 6 outside the load range; stopped",
        &["k=4 v=11", "k=5 v=22", "num=2"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 4, 6);",
        &[("p.hex", "@2 11 22\n")],
        "address 2 outside the load range; stopped",
        &["num=0"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 4, 8);",
        &[("p.hex", "11 22 33\n")],
        "not enough words for the range [4:8] (3 of 5); rest unchanged",
        &["k=4 v=11", "k=5 v=22", "k=6 v=33", "num=3"],
    );
    stops(
        B32,
        "bit [31:0]",
        "$readmemh(\"p.hex\", mem, 2, 0);",
        &[("p.hex", "11 22 33 44\n")],
        "address -1 outside the load range; stopped",
        &["k=0 v=33", "k=1 v=22", "k=2 v=11", "num=3"],
    );
}

/// An address that is not a value of the key type is refused by name and the load
/// stops there (module doc: no settled reading). verilator, for the record: `[int]`
/// `@7fffffff 11 5a` → `k=7fffffff v=11`, `k=80000000 v=5a` (-2^31);
/// `[bit [7:0]]` `@1fe` → `k=fe`; `[byte]` `@7e 11 22 33` → keys 7e, 7f, -128;
/// `[bit [31:0]]` start -1 → `k=ffffffff`, then 0; `[longint]`
/// `@ffffffffffffffff` → -1.
#[test]
fn addresses_outside_the_key_type_are_refused() {
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@7fffffff 11 5a\n")],
        "the next address 2147483648 is not a value of the associative array's 32-bit signed key; stopped",
        &["k=7fffffff v=11", "num=1"],
    );
    stops(
        "bit [7:0] mem [bit [7:0]];",
        "bit [7:0]",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@1fe 5a a5 c3\n")],
        "address @1fe is not a value of the associative array's 8-bit unsigned key; stopped",
        &["num=0"],
    );
    stops(
        "bit [7:0] mem [byte];",
        "byte",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@7e 11 22 33\n")],
        "the next address 128 is not a value of the associative array's 8-bit signed key",
        &["k=7e v=11", "k=7f v=22", "num=2"],
    );
    stops(
        B32,
        "bit [31:0]",
        "$readmemh(\"p.hex\", mem, -1);",
        &[("p.hex", "11 22\n")],
        "the start address -1 is not a value of the associative array's 32-bit unsigned key; nothing loaded",
        &["num=0"],
    );
    stops(
        "bit [7:0] mem [longint];",
        "longint",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@100000000 5a @ffffffffffffffff 77\n")],
        "address @ffffffffffffffff is not a value of the associative array's 64-bit signed key",
        &["k=100000000 v=5a", "num=1"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 32'h80000000);",
        &[("p.hex", "11 22\n")],
        "the start address 2147483648 is not a value",
        &["num=0"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem, 8'bx);",
        &[("p.hex", "11 22\n")],
        "the start address of an associative-array load is x or z; nothing loaded",
        &["num=0"],
    );
    // `[time]` is 64-bit unsigned: no negative start, and nothing at or above 2^63,
    // which the signed key domain would hold as negative (ROADMAP §2 `[time]`).
    // verilator: `k=ffffffff v=11`, `k=100000000 v=22` (the 32-bit -1 zero-extended);
    // `k=7fffffffffffffff v=11`, `k=8000000000000000 v=5a`.
    stops(
        "bit [7:0] mem [time];",
        "time",
        "$readmemh(\"p.hex\", mem, -1);",
        &[("p.hex", "11 22\n")],
        "the start address -1 is not a value of the associative array's 64-bit unsigned key",
        &["num=0"],
    );
    stops(
        "bit [7:0] mem [time];",
        "time",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@7fffffffffffffff 11 5a\n")],
        "the next address 9223372036854775808 is not a value of the associative array's 64-bit unsigned key",
        &["k=7fffffffffffffff v=11", "num=1"],
    );
}

/// A word before any address into a SIGNED key with no start argument: refused
/// (module doc). verilator starts at 0 (`k=0 v=5a`, `k=1 v=a5`, `k=2 v=c3`).
#[test]
fn signed_key_without_a_start_address_is_refused() {
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "5a a5 c3\n")],
        "a word before any `@address`, into an associative array with a 32-bit signed key and no start address",
        &["num=0"],
    );
}

/// Words: wider than the element truncate to its low bits, narrower zero-extend;
/// an x / z digit is kept by a 4-state element (§21.4; verilator, 2-state, prints
/// `05 00 10`) and reads 0 in a 2-state one (§6.11.3; verilator `05 00 10`).
/// verilator: `k=0 v=ff`, `k=1 v=02`, `k=2 v=bc`; `v=0000000005`, `v=123456789a`,
/// `v=ffffffffff`; `int` elements `00000010 00000000 7fffffff ffffffff`; signed
/// elements `-1 127 -128`.
#[test]
fn word_widths_and_unknown_digits() {
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@0 1ff 2 abc\n")],
        &["k=0 v=ff", "k=1 v=02", "k=2 v=bc", "num=3"],
    );
    loads(
        "bit [39:0] mem [int];",
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@0 5 123456789a ffffffffffff\n")],
        &[
            "k=0 v=0000000005",
            "k=1 v=123456789a",
            "k=2 v=ffffffffff",
            "num=3",
        ],
    );
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@0 x5 zz 1x\n")],
        &["k=0 v=05", "k=1 v=00", "k=2 v=10", "num=3"],
    );
    loads(
        "logic [7:0] mem [int];",
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@0 x5 zz 1x\n")],
        &["k=0 v=x5", "k=1 v=zz", "k=2 v=1x", "num=3"],
    );
    loads(
        "int mem [int];",
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@3 1x z 7fffffff ffffffff\n")],
        &[
            "k=3 v=00000010",
            "k=4 v=00000000",
            "k=5 v=7fffffff",
            "k=6 v=ffffffff",
            "num=4",
        ],
    );
}

/// Comments, blank lines, upper-case digits, `_`, a zero-padded address, and
/// `$readmemb` (binary words, hex `@` addresses). verilator `k=3 v=11`, `k=4 v=22`,
/// `k=9 v=33`; `k=a v=a5`, `k=b v=c3`; `k=10 v=5a`, `k=11 v=a5`, `k=1f v=08`;
/// `k=80000010 v=5a`, `k=80000011 v=a5`.
#[test]
fn file_syntax_and_readmemb() {
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[(
            "p.hex",
            "// lead\n\n@3 /* blk\n x */ 11\n\n  22 // tail\n/* a */ @9\n33\n",
        )],
        &["k=3 v=11", "k=4 v=22", "k=9 v=33", "num=3"],
    );
    loads(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@A 5A_A5 Fe\n@0000000000000000000B c3\n")],
        &["k=a v=a5", "k=b v=c3", "num=2"],
    );
    loads(
        INT,
        "int",
        "$readmemb(\"p.txt\", mem);",
        &[("p.txt", "@10\n01011010 10100101\n@1f 1x0z\n")],
        &["k=10 v=5a", "k=11 v=a5", "k=1f v=08", "num=3"],
    );
    loads(
        B32,
        "bit [31:0]",
        "$readmemb(\"p.txt\", mem);",
        &[("p.txt", "@80000010\n01011010 10100101\n")],
        &["k=80000010 v=5a", "k=80000011 v=a5", "num=2"],
    );
}

/// A missing file warns and loads nothing (entries already there stay; verilator
/// `%Warning: nope.hex:0: $readmem file not found`, `k=3 v=11`); an empty file, or
/// one with only comments, loads nothing silently (verilator `num=0`). An address
/// or word with a character that is not a digit stops the load where it stands
/// (iverilog's fixed twin: `Invalid input character: g`, the words before it
/// loaded; verilator aborts the run).
#[test]
fn missing_empty_and_malformed_files() {
    stops(
        B32,
        "bit [31:0]",
        "mem[3] = 8'h11; $readmemh(\"nope.hex\", mem);",
        &[],
        "unable to open 'nope.hex' for reading",
        &["k=3 v=11", "num=1"],
    );
    loads(
        B32,
        "bit [31:0]",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "\n\n// only a comment\n")],
        &["num=0"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@1g 11 @4 22\n")],
        "bad address token '@1g'; stopped",
        &["num=0"],
    );
    stops(
        INT,
        "int",
        "$readmemh(\"p.hex\", mem);",
        &[("p.hex", "@4 11 g2 33\n")],
        "bad data word 'g2'; stopped",
        &["k=4 v=11", "num=1"],
    );
    stops(
        INT,
        "int",
        "$readmemb(\"p.txt\", mem);",
        &[("p.txt", "@3 111111111 01_10 2\n")],
        "bad data word '2'; stopped",
        &["k=3 v=ff", "k=4 v=06", "num=2"],
    );
}

/// Other spellings of the target: a static task's module array, a generate-scope
/// name, an upward hierarchical path with a start argument. verilator `k=20 v=5a`,
/// `k=21 v=a5`; `5a a5`; `m=a5 n=2`.
#[test]
fn target_spellings() {
    let (out, err, code) = run(
        "module top;
  bit [7:0] mem [bit [31:0]];
  task load(input string f); $readmemh(f, mem); endtask
  initial begin load(\"p.hex\"); $display(\"%h %h %0d\", mem[32], mem[33], mem.num()); $finish; end
endmodule
",
        &[("p.hex", "@20 5a a5\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["5a a5 2"]), "{err}");
    let (out, err, code) = run(
        "module top;
  if (1) begin : g
    bit [7:0] mem [int];
  end
  initial begin
    $readmemh(\"p.hex\", g.mem);
    $display(\"%h %h\", g.mem[16], g.mem[17]);
    $finish;
  end
endmodule
",
        &[("p.hex", "@10 5a a5\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["5a a5"]), "{err}");
    let (out, err, code) = run(
        "module memm;
  bit [7:0] mem [bit [31:0]];
  initial begin #1; $display(\"m=%h n=%0d\", mem[32'h80000001], mem.num()); end
endmodule
module top;
  memm lmem ();
  initial begin
    $readmemh(\"p.hex\", top.lmem.mem, 32'h80000000);
    #2 $finish;
  end
endmodule
",
        &[("p.hex", "5a a5\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["m=a5 n=2"]), "{err}");
}

/// Refused at elaboration: a `[string]` key, local and hierarchical (verilator:
/// `$readmemh address/key must be integral (IEEE 1800-2023 21.4.1)`), and
/// `$writememh` of an associative array, local and hierarchical (not part of this
/// row; PRE's messages, unchanged).
#[test]
fn string_key_and_writemem_stay_loud() {
    let refused = |src: &str, needle: &str| {
        let (_, err, code) = run(src, &[("p.hex", "@10 5a\n")]);
        assert_eq!(code, 1, "must be refused:\n{err}");
        assert!(
            err.contains("VITA-E3009") && err.contains(needle),
            "{needle}:\n{err}"
        );
    };
    refused(
        "module top; bit [7:0] mem [string]; initial $readmemh(\"p.hex\", mem); endmodule",
        "into an associative array with a `string` key",
    );
    refused(
        "module memm; bit [7:0] mem [string]; endmodule
module top; memm lmem (); initial $readmemh(\"p.hex\", lmem.mem); endmodule",
        "into an associative array with a `string` key",
    );
    refused(
        "module top; bit [7:0] mem [int]; initial begin mem[1] = 1; $writememh(\"o.hex\", mem); end endmodule",
        "a dynamic-storage handle has no whole-value surface",
    );
    refused(
        "module memm; bit [7:0] mem [int]; endmodule
module top; memm lmem (); initial $writememh(\"o.hex\", lmem.mem); endmodule",
        "hierarchical read of `lmem.mem` is unsupported",
    );
}

/// Round-1 lens F1: an ELEMENT select as the target is not a memory — PRE's W4023,
/// nothing loaded — whatever the index (constant, parameter, variable, negative,
/// hierarchical) and the key (keyword, typed, `[time]`, any width). Round 1 loaded
/// the whole file through `mem[5]`: the key type rode in the `word` slot, the shape a
/// constant-index element select lowers to. verilator 5.052 refuses every one:
/// `Unsupported: $readmemh into other than unpacked or associative array`.
#[test]
fn element_select_target_is_not_a_memory() {
    let not_a_memory = |decl: &str, pre: &str, target: &str| {
        let src = format!(
            "module top;
  {decl}
  {pre}
  initial begin $readmemh(\"p.hex\", {target}); $display(\"num=%0d\", mem.num()); $finish; end
endmodule
"
        );
        let (out, err, code) = run(&src, &[("p.hex", "@10 5a a5\n")]);
        assert_eq!(code, 0, "{target}:\n{err}");
        assert!(
            err.contains("VITA-W4023") && err.contains("$readmem target is not a memory"),
            "{target}:\n{err}"
        );
        assert_eq!(lines(&out), vec!["num=0"], "{target}:\n{err}");
    };
    not_a_memory(INT, "", "mem[5]");
    not_a_memory(INT, "", "mem[0]");
    not_a_memory(INT, "", "mem[-1]");
    not_a_memory(INT, "localparam int K = 3;", "mem[K]");
    not_a_memory(INT, "int k = 5;", "mem[k]");
    not_a_memory("int mem [int];", "", "mem[5]");
    not_a_memory("bit [7:0] mem [bit [7:0]];", "", "mem[8'd5]");
    not_a_memory(B32, "", "mem[5]");
    not_a_memory("bit [7:0] mem [longint];", "", "mem[64'd5]");
    not_a_memory("bit [7:0] mem [time];", "", "mem[5]");
    let (out, err, code) = run(
        "module memm;
  bit [7:0] mem [int];
  initial begin #1 $display(\"num=%0d\", mem.num()); end
endmodule
module top;
  memm lmem ();
  initial begin $readmemh(\"p.hex\", lmem.mem[5]); #2 $finish; end
endmodule
",
        &[("p.hex", "@10 5a\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["num=0"]), "{err}");
    assert!(err.contains("$readmem target is not a memory"), "{err}");
}

/// Round-1 lens F3: a `$readmem` word is an integral value, so an element type that
/// is not integral is refused by name (verilator 5.052: `Unsupported: $readmemh array
/// values must be integral`; PRE: E3009 `a dynamic-storage handle has no whole-value
/// surface` / `hierarchical read of `lmem.mem` is unsupported`). Round 1 loaded a
/// class-handle element array (`n=2`). Local, hierarchical, generate-scope,
/// block-local and typedef'd class element.
#[test]
fn non_integral_element_is_refused() {
    let refused = |src: &str| {
        let (_, err, code) = run(src, &[("p.hex", "@10 1 2\n")]);
        assert_eq!(code, 1, "must be refused:\n{err}");
        assert!(
            err.contains("VITA-E3009") && err.contains("whose element type is not integral"),
            "{err}"
        );
    };
    refused(
        "class C; int a; endclass
module top;
  C mem [int];
  initial begin $readmemh(\"p.hex\", mem); $display(\"n=%0d\", mem.num()); $finish; end
endmodule",
    );
    refused(
        "class C; int a; endclass
module memm; C mem [int]; endmodule
module top; memm lmem (); initial begin $readmemh(\"p.hex\", lmem.mem); $finish; end endmodule",
    );
    refused(
        "class C; int a; endclass
module top;
  if (1) begin : g C mem [int]; end
  initial begin $readmemh(\"p.hex\", g.mem); $finish; end
endmodule",
    );
    refused(
        "class C; int a; endclass
module top;
  initial begin : b C mem [int]; $readmemh(\"p.hex\", mem); $finish; end
endmodule",
    );
    refused(
        "class C; int a; endclass
module top;
  typedef C c_t;
  c_t mem [int];
  initial begin $readmemh(\"p.hex\", mem); $finish; end
endmodule",
    );
}

/// The integral element kinds load (verilator 5.052, each line).
#[test]
fn integral_element_kinds_load() {
    let loads_as = |decl: &str, show: &str, want: &str| {
        let src = format!(
            "module top;
  {decl}
  initial begin $readmemh(\"p.hex\", mem); $display({show}); $finish; end
endmodule
"
        );
        let (out, err, code) = run(&src, &[("p.hex", "@10 5a a5\n")]);
        assert_eq!((code, lines(&out)), (0, vec![want]), "{decl}:\n{err}");
    };
    let n_dec = "\"n=%0d v=%0d %0d\", mem.num(), mem[16], mem[17]";
    loads_as("int mem [int];", n_dec, "n=2 v=90 165");
    loads_as("integer mem [int];", n_dec, "n=2 v=90 165");
    loads_as("time mem [int];", n_dec, "n=2 v=90 165");
    loads_as("logic signed [7:0] mem [int];", n_dec, "n=2 v=90 -91");
    loads_as("typedef byte b_t; b_t mem [int];", n_dec, "n=2 v=90 -91");
    loads_as(
        "bit [1:0][3:0] mem [int];",
        "\"n=%0d v=%h %h\", mem.num(), mem[16], mem[17]",
        "n=2 v=5a a5",
    );
    loads_as(
        "typedef struct packed { bit [3:0] a; bit [3:0] b; } s_t; s_t mem [int];",
        "\"n=%0d a=%h b=%h\", mem.num(), mem[16].a, mem[17].b",
        "n=2 a=5 b=5",
    );
    // lens e6_el_enum: verilator `v=1 3`.
    let (out, err, code) = run(
        "module top;
  typedef enum bit [1:0] {A, B, C} e_t;
  e_t mem [int];
  initial begin $readmemh(\"p.hex\", mem); $display(\"v=%0d %0d\", mem[16], mem[17]); $finish; end
endmodule
",
        &[("p.hex", "@10 1 3\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["v=1 3"]), "{err}");
}

/// Round-1 lens F2: a `wait` over an associative array does not wake when its
/// elements are written (ROADMAP §2, on queues and dynamic arrays alike; the
/// element-write twins print only `end` on PRE), so a `$readmem*` into an array that
/// any `wait` condition reads is refused by name. Each reading: a method (`num`,
/// `size`, `exists`), a constant or variable element, inside a task body or a fork,
/// a called function (also one that loops), a hierarchical `wait` in the child over
/// the array the parent loads, a hierarchical element read, a virtual method an
/// override of which reads it. verilator 5.052 wakes the direct ones (`woke t=5`);
/// its function-call `wait`s never wake (`end`).
#[test]
fn an_array_a_wait_reads_is_refused() {
    let refused = |src: &str| {
        let (_, err, code) = run(src, &[("p.hex", "@10 5a a5\n")]);
        assert_eq!(code, 1, "must be refused:\n{src}\n{err}");
        assert!(
            err.contains("VITA-E3009") && err.contains("which a `wait` condition reads"),
            "{src}\n{err}"
        );
    };
    let local = |decls: &str, waiter: &str| {
        format!(
            "module top;
  bit [7:0] mem [int];
  {decls}
  initial begin #5 $readmemh(\"p.hex\", mem); end
  {waiter}
  initial begin #20 $display(\"end\"); $finish; end
endmodule
"
        )
    };
    let woke = "$display(\"woke t=%0t\", $time);";
    for cond in [
        "mem.num() == 2",
        "mem.size() == 2",
        "mem.exists(17)",
        "mem[17] == 8'ha5",
        "mem[k] != 0",
    ] {
        refused(&local(
            "int k = 16;",
            &format!("initial begin wait ({cond}); {woke} end"),
        ));
    }
    refused(&local(
        "task automatic wt(); wait (mem.num() == 2); $display(\"woke\"); endtask",
        "initial wt();",
    ));
    refused(&local(
        "",
        &format!("initial fork begin wait (mem.num() == 2); {woke} end join_none"),
    ));
    refused(&local(
        "function automatic int cnt(); return mem.num(); endfunction",
        &format!("initial begin wait (cnt() == 2); {woke} end"),
    ));
    refused(&local(
        "function automatic int cnt(); int n; n = 0; \
         for (int i = 0; i < 32; i++) if (mem.exists(i)) n++; return n; endfunction",
        &format!("initial begin wait (cnt() == 2); {woke} end"),
    ));
    refused(&local(
        "class B; virtual function bit [7:0] v(); return 0; endfunction endclass
  class D extends B; virtual function bit [7:0] v(); return top.mem[16]; endfunction endclass
  B b; D d;
  initial begin d = new; b = d; end",
        &format!("initial begin #1 wait (b.v() == 8'h5a); {woke} end"),
    ));
    // Lens r2 m23 pin (c5/m23_pin_mixed, c5/m23_pin_mixed_noq): a direct `wait` on
    // the array beside a virtual-call `wait` on something else, with and without an
    // unrelated class `Q` (whose vtable row is padded at `v`'s slot). verilator 5.052:
    // `woke t=5 n=2` for both.
    for q in ["class Q; int x; endclass", ""] {
        refused(&local(
            &format!(
                "bit [7:0] go;
  {q}
  class B; virtual function bit [7:0] v(); return 0; endfunction endclass
  class D extends B; virtual function bit [7:0] v(); return top.go; endfunction endclass
  B b; D d;
  initial begin d = new; b = d; end
  initial begin #1 wait (b.v() == 8'h5a); $display(\"v woke t=%0t\", $time); end"
            ),
            "initial begin wait (mem.num() == 2); $display(\"woke t=%0t n=%0d\", $time, mem.num()); end",
        ));
    }
    refused(
        "module memm;
  bit [7:0] mem [int];
  initial begin wait (mem.exists(16)); $display(\"woke\"); end
endmodule
module top;
  memm lmem ();
  initial begin #5 $readmemh(\"p.hex\", lmem.mem); end
endmodule
",
    );
    refused(
        "module memm; bit [7:0] mem [int]; endmodule
module top;
  memm lmem ();
  initial begin #5 $readmemh(\"p.hex\", lmem.mem); end
  initial begin wait (lmem.mem[16] == 8'h5a); $display(\"woke\"); end
endmodule
",
    );
}

/// The `wait` refusal is keyed on the array: a `wait` on something else — a flag, or
/// a variable an `always_comb` derives from the array (which does wake on the load) —
/// keeps the load. verilator 5.052: `woke t=5 n=2 v=5a`, `woke t=5 n=2`.
#[test]
fn a_wait_on_something_else_keeps_the_load() {
    let (out, err, code) = run(
        "module top;
  bit [7:0] mem [int];
  bit go;
  initial begin #5 $readmemh(\"p.hex\", mem); go = 1; end
  initial begin wait (go); $display(\"woke t=%0t n=%0d v=%h\", $time, mem.num(), mem[16]); end
  initial begin #20 $finish; end
endmodule
",
        &[("p.hex", "@10 5a a5\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["woke t=5 n=2 v=5a"]), "{err}");
    let (out, err, code) = run(
        "module top;
  bit [7:0] mem [int];
  int n;
  always_comb n = mem.num();
  initial begin #5 $readmemh(\"p.hex\", mem); end
  initial begin wait (n == 2); $display(\"woke t=%0t n=%0d\", $time, n); end
  initial begin #20 $finish; end
endmodule
",
        &[("p.hex", "@10 5a a5\n")],
    );
    assert_eq!((code, lines(&out)), (0, vec!["woke t=5 n=2"]), "{err}");
}

/// Lens r2 NOTE-1: a class with no method at a virtual slot has a padded vtable row
/// there (`finalize_vtables`), an absent method rather than an unknown callee, so a
/// `wait` through a virtual call on something else keeps the load beside it.
/// c4/m23_over_unrelated_class (an unrelated class `Q`), c4/m23_over_vslot_missing
/// (the base `B` lacks the derived `g`): `woke t=5 n=2 v=5a`, as their twins without
/// the padding (c4/m23_over_unrelated_ctl, c4/m23_over_vslot_ctl). c5/m23_over_two_waits
/// (a second array beside): `v woke t=5 n=2` — IEEE 1800-2017 §9.4.3, the `wait`
/// resumes when its condition becomes true, at `go = 8'h5a`. verilator 5.052 prints
/// only `end` for all three: it never re-evaluates a `wait` whose condition calls a
/// function.
#[test]
fn a_virtual_call_wait_beside_a_padded_vtable_keeps_the_load() {
    let cell = |decls: &str, classes: &str, wait: &str, display: &str| {
        format!(
            "module top;
  bit [7:0] mem [int];
  {decls}
  bit [7:0] go;
  {classes}
  initial begin d = new; {b} #5 $readmemh(\"p.hex\", mem); go = 8'h5a; end
  initial begin #1 wait ({wait} == 8'h5a); $display({display}); end
  initial begin #20 $display(\"end\"); $finish; end
endmodule
",
            b = if wait.starts_with("b.") { "b = d;" } else { "" },
        )
    };
    let full = "\"woke t=%0t n=%0d v=%h\", $time, mem.num(), mem[16]";
    let derived =
        "class D extends B; virtual function bit [7:0] v(); return top.go; endfunction endclass
  B b; D d;";
    let unrelated = cell(
        "",
        &format!(
            "class Q; int x; endclass
  class B; virtual function bit [7:0] v(); return 0; endfunction endclass
  {derived}"
        ),
        "b.v()",
        full,
    );
    let vslot_missing = cell(
        "",
        "class B; virtual function bit [7:0] v(); return 0; endfunction endclass
  class D extends B; virtual function bit [7:0] g(); return top.go; endfunction endclass
  D d;",
        "d.g()",
        full,
    );
    let two_waits = cell(
        "bit [7:0] other [int];",
        &format!(
            "class Q; int x; endclass
  class B; virtual function bit [7:0] v(); return 0; endfunction endclass
  {derived}"
        ),
        "b.v()",
        "\"v woke t=%0t n=%0d\", $time, mem.num()",
    );
    for (src, want) in [
        (&unrelated, "woke t=5 n=2 v=5a"),
        (&vslot_missing, "woke t=5 n=2 v=5a"),
        (&two_waits, "v woke t=5 n=2"),
    ] {
        for backend in [None, Some("interp"), Some("vm")] {
            let (out, err, code) = run_with(src, &[("p.hex", "@10 5a a5\n")], backend);
            assert_eq!(
                (code, lines(&out)),
                (0, vec![want, "end"]),
                "{backend:?}\n{src}\n{err}"
            );
        }
    }
}
