//! §3 ⑤ ⓓ: `'{…}` assignment patterns, casts and package parameters of a
//! NESTED packed struct type (the second half of `nested_packed_struct.rs`): a
//! positional or keyed pattern whose element for a nested member is itself a
//! pattern (recursed per member), a plain value for the nested slot, a `default:`
//! fill (a non-fill non-zero `default:` for a nested slot is loud — whether it
//! applies whole or per leaf is not pinned), a struct-typed cast `cap_t'(e)`
//! (size + signing) also in a constant, a package `parameter cap_t` of such a
//! pattern, and the real ibex_cheriot_pkg page-1 types and constants.
//!
//! Every expected value is the census oracle line (verilator 5.050; iverilog
//! 13.0 rejects a `'{…}` with a nested struct; both agree where both ran).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_nest_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `DIGEST=` line, in emission order, joined by `|` (the census harness format).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn ctl_misc() {
    // ctl_cast_flat: both oracles
    digest(
        "ctl_cast_flat",
        r#"module tb;
  typedef struct packed { logic [1:0] cor; logic [2:0] perms; logic valid; } cap_t;
  typedef struct packed { logic [3:0] extra; logic [2:0] perms; logic [1:0] cor; logic [2:0] perms2; logic valid; } dcap_t;
  dcap_t d; cap_t c;
  initial begin
    d = '0; d.extra = 4'hf; d.perms2 = 3'b011; d.cor = 2'b01; d.valid = 1;
    c = cap_t'(d);
    $display("DIGEST=%h %h %h %0d %0d %h", d, c, c.perms, $bits(cap_t), $bits(dcap_t), cap_t'(3'b101));
    d = dcap_t'(c);
    $display("DIGEST=%h", d);
  end

  initial #5 $finish;
endmodule
"#,
        "1e17 17 3 6 13 05|0017",
    );
    // ctl_default_x: verilator — flat control of ns_pat_default_x — PRE == POST
    digest(
        "ctl_default_x",
        r#"module tb;
  typedef struct packed { logic [1:0] cor; logic [3:0] perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{default: 'x};  $display("DIGEST=%b", c);
    c = '{cor: 2'b10, default: 'z}; $display("DIGEST=%b", c);
  end

  initial #5 $finish;
endmodule
"#,
        "xxxxxxx|10zzzzz",
    );
}

#[test]
fn ns_misc() {
    // ns_cast: both oracles
    digest(
        "ns_cast",
        r#"module tb;
  typedef struct packed { logic U0; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  typedef struct packed { logic [3:0] extra; perms_t perms; logic [1:0] cor; perms_t perms2; logic valid; } dcap_t;
  dcap_t d; cap_t c;
  initial begin
    d = '0; d.extra = 4'hf; d.perms2.q = 2'b11; d.cor = 2'b01; d.valid = 1;
    c = cap_t'(d);
    $display("DIGEST=%h %h %h %0d %0d %h", d, c, c.perms.q, $bits(cap_t), $bits(dcap_t), cap_t'(3'b101));
    d = dcap_t'(c);
    $display("DIGEST=%h", d);
  end

  initial #5 $finish;
endmodule
"#,
        "1e17 17 3 6 13 05|0017",
    );
    // ns_cast_signed: both oracles
    digest(
        "ns_cast_signed",
        r#"module tb;
  typedef struct packed signed { logic [1:0] cor; logic [2:0] q; } s_t;
  typedef struct packed { s_t s; logic v; } o_t;
  o_t o; s_t s; int i;
  initial begin
    s = s_t'(5'b10110); i = s_t'(5'b10110);
    o.s = s; o.v = 1;
    $display("DIGEST=%b %0d %0d %b", s, i, o.s, o);
  end

  initial #5 $finish;
endmodule
"#,
        "10110 -10 -10 101101",
    );
    // ns_cheriot_page1: verilator
    digest("ns_cheriot_page1", r#"// Copyright lowRISC contributors.
// Copyright Microsoft Corporation
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

// CHERIoT types, constants, and functions shared across Ibex.

package ibex_cheriot_pkg;

  // Capability width parameters (spec v1.0, chapter 7.13)
  //
  //                       31 30       25 24   22 21  18 17             9 8              0
  // +-----------+       +---+-----------+-------+------+----------------+----------------+
  // | valid tag |       | R |  cperms   | otype | cexp |    top (T)     |    base (B)    |
  // +-----------+       +---+-----------+-------+------+----------------+----------------+
  //      [1]             [1]      [6]      [3]    [4]         [9]               [9]
  //
  // Naming convention: C* prefix for compressed/stored form. No prefix for the expanded/working
  // form only used inside the core
  parameter int unsigned ADDR_W    = 32;
  parameter int unsigned CBOUND_W  = 9;   // 9-bit compressed bound (T or B)
  parameter int unsigned CEXP_W    = 4;   // 4-bit compressed exponent
  parameter int unsigned EXP_W     = 5;   // 5-bit expanded exponent
  parameter int unsigned OTYPE_W   = 3;   // 3-bit object type (sealing)
  parameter int unsigned CPERMS_W  = 6;   // 6-bit compressed permissions
  parameter int unsigned PERMS_W   = 12;  // 12-bit expanded permissions
  // Width of the compressed capability type (cap_t) as a flat vector used for ECC protection.
  parameter int unsigned REGCAP_W = 35;

  // Capability typedefs
  typedef logic [CBOUND_W-1:0] cbound_t;  // 9-bit compressed bound (T or B)
  typedef logic [CEXP_W-1:0]   cexp_t;    // 4-bit compressed exponent
  typedef logic [EXP_W-1:0]    exp_t;     // 5-bit expanded exponent
  typedef logic [OTYPE_W-1:0]  otype_t;   // 3-bit object type (sealing)
  typedef logic [CPERMS_W-1:0] cperms_t;  // 6-bit compressed permissions
  typedef logic [1:0]          cap_cor_t; // 2-bit correction: [1]=top_hi^addr_hi, [0]=addr_hi

  // Expanded 12-bit permissions (spec v1.0, chapter 7.13.1)
  typedef struct packed {
    logic U0;  // [11] user permission (software-defined)
    logic SE;  // [10] seal
    logic US;  // [9]  unseal
    logic EX;  // [8]  execute
    logic SR;  // [7]  access system registers
    logic MC;  // [6]  load/store capability
    logic LD;  // [5]  load
    logic SL;  // [4]  store local capability
    logic LM;  // [3]  load mutable
    logic SD;  // [2]  store
    logic LG;  // [1]  load global
    logic GL;  // [0]  global
  } perms_t;

  // Sealing types (spec v1.0, chapter 7.13.2)
  parameter otype_t OTYPE_UNSEALED        = 3'd0; // unsealed
  parameter otype_t OTYPE_SENTRY_II_FWD   = 3'd1; // interrupt-inheriting forward sentry
  parameter otype_t OTYPE_SENTRY_ID_FWD   = 3'd2; // interrupt disabling forward sentry
  parameter otype_t OTYPE_SENTRY_IE_FWD   = 3'd3; // interrupt enabling forward sentry
  parameter otype_t OTYPE_SENTRY_ID_BKWD  = 3'd4; // interrupt disabling backward sentry
  parameter otype_t OTYPE_SENTRY_IE_BKWD  = 3'd5; // interrupt enabling backward sentry

  // Exponent (spec v1.0, chapter 7.13.3)
  parameter cexp_t MAXCEXP = 4'd15; // compressed maximum exponent encoding
  parameter exp_t  MAXEXP  = 5'd24; // expanded maximum exponent

  // -----------------------------------------------------------------------------------------------
  // Capability types
  // -----------------------------------------------------------------------------------------------
  // The CHERIoT ISA defines the capability format (spec v1.0, chapter 7.13, Figure 7.2).
  // Two types are used in the RTL:
  //
  //  cap_t          - The primary compressed form defined by the spec and used everywhere a
  //                   capability is stored (register file, CSR registers, load/store ports,
  //                   ECC/lockstep vectors). Its lower 33 bits (cap[32:0]) are a 1:1 match with the
  //                   spec so writing to memory is a simple truncation and a direct cast.
  //                   Bits [34:33] store the correction factors that are not in the spec but kept
  //                   in the register file to save recomputation costs. See the
  //                   `cheriot_compute_corrections` function below.
  //
  //  decoded_cap_t  - The uncompressed capability type used inside the core. Created by
  //                   `cheriot_decode_cap` and `cheriot_encode_cap`. Its lower 35 bits are the same
  //                   as cap_t so encode is simpley a cast. The upper bits add the decompressed
  //                   bounds and permissions.
  // -----------------------------------------------------------------------------------------------

  // Compressed capability + correction factors
  typedef struct packed {
    cap_cor_t cap_cor; // [34:33] correction factors
    // Capability according to spec v1.0, chapter 7.13
    logic     valid;   // [32]    tag bit
    logic     rsvd;    // [31]    reserved R
    cperms_t  cperms;  // [30:25] compressed permissions
    otype_t   otype;   // [24:22] object type
    cexp_t    cexp;    // [21:18] 4-bit compressed exponent
    cbound_t  top;     // [17:9]  top mantissa T
    cbound_t  base;    // [8:0]   base mantissa B
  } cap_t;  // 2+1+1+6+3+4+9+9 = 35 bits = REGCAP_W

  // Uncompressed capability
  typedef struct packed {
    logic [ADDR_W:0]   top33;  // 33 bits absolute top
    logic [ADDR_W-1:0] base32; // 32 bits absolute base
    perms_t            perms;  // 12 bits expanded permissions
    // Identical layout to cap_t from here
    cap_cor_t cap_cor; // [34:33] correction factors
    logic     valid;   // [32]    tag bit
    logic     rsvd;    // [31]    reserved R
    cperms_t  cperms;  // [30:25] compressed permissions
    otype_t   otype;   // [24:22] object type
    cexp_t    cexp;    // [21:18] 4-bit compressed exponent
    cbound_t  top;     // [17:9]  top mantissa T
    cbound_t  base;    // [8:0]   base mantissa B
  } decoded_cap_t;  // 33+32+12+35 = 112 bits


  // Types for bound computation in CHERIoT EX Stage
  typedef struct packed {
    logic [32:0]    top33req; // requested top = addr + length (33-bit)
    exp_t           exp1;     // exponent candidate from length (no overflow)
    exp_t           exp2;     // exp1 + 1 (used when exp1 path overflows)
    logic [EXP_W:0] explen;   // MSB position of length[31:9] (6-bit, can be 32)
    logic [EXP_W:0] expb;     // trailing-zero count of base address (6-bit)
    logic           in_bound; // addr..addr+length lies within parent bounds
  } bound_req_t;

  // The decoded capability plus the alignment mask and representable length.
  typedef struct packed {
    decoded_cap_t cap;   // resulting capability
    logic [31:0]  maska; // alignment mask    (CRAM result)
    logic [31:0]  rlen;  // representable len (CRRL result)
  } bound_result_t;

  // Permission-clearing control for capability loads (CLC instruction).
  // Each field corresponds to one CLC clearing rule from the spec.
  typedef struct packed {
    logic CTAG;   // [2] clear tag (valid) bit (loading cap lacks MC)
    logic SD_LM;  // [1] clear SD and LM       (loading cap lacks LM)
    logic GL_LG;  // [0] clear GL and LG       (loading cap lacks LG)
  } cap_clrperm_t;

  // -----------------------------------------------------------------------------------------------
  // Root Capabilities and Constants
  // -----------------------------------------------------------------------------------------------
  parameter cap_t         NULL_CAP         = '{default: '0};
  parameter decoded_cap_t NULL_DECODED_CAP = '{default: '0};

  // Three CHERIoT root capabilities (spec v1.0, chapter 7.13.1)
  parameter logic [5:0] CPERMS_TX = 6'b101111;  // Tx (executable root)
  parameter logic [5:0] CPERMS_TM = 6'b111111;  // Tm (memory root)
  parameter logic [5:0] CPERMS_TS = 6'b100111;  // Tx (sealing root)

  // ROOT_DECODED_CAP_TX is used for the PCC. All other root capabilities are cap_t
  // Executable Root Capability
  parameter decoded_cap_t ROOT_DECODED_CAP_TX = '{
    top33:   33'h10000_0000,
    base32:  '0,
    perms:   12'h1eb,
    cap_cor: '0,
    valid:   1'b1,
    rsvd:    1'b0,
    cperms:  CPERMS_TX,
    otype:   OTYPE_UNSEALED,
    cexp:    MAXCEXP,
    top:     9'h100,
    base:    '0
  };
  parameter cap_t ROOT_CAP_TX = cap_t'(ROOT_DECODED_CAP_TX);

  // Memory Root Capability
  parameter cap_t ROOT_CAP_TM = '{
    cap_cor: '0,
    valid:   1'b1,
    rsvd:    1'b0,
    cperms:  CPERMS_TM,
    otype:   OTYPE_UNSEALED,
    cexp:    MAXCEXP,
    top:     9'h100,
    base:    '0
  };

  // Sealing Root Capability
  parameter cap_t ROOT_CAP_TS = '{
    cap_cor: '0,
    valid:   1'b1,
    rsvd:    1'b0,
    cperms:  CPERMS_TS,
    otype:   OTYPE_UNSEALED,
    cexp:    MAXCEXP,
    top:     9'h100,
    base:    '0
  };


  // Implicit permission masks for each compressed permission format (spec v1.0, chapter 7.13.1)
  parameter perms_t PERM_MRW_IMSK = '{LD:1, MC:1, SD:1, default:0}; // Memory cap-read-write
  parameter perms_t PERM_MRO_IMSK = '{LD:1, MC:1, default:0};       // Memory cap-read-only
  parameter perms_t PERM_MWO_IMSK = '{SD:1, MC:1, default:0};       // Memory cap-write-only
  parameter perms_t PERM_MDO_IMSK = '{default:0};                   // Memory data-only
  parameter perms_t PERM_EXE_IMSK = '{EX:1, MC:1, LD:1, default:0}; // Executable
  parameter perms_t PERM_SEA_IMSK = '{default:0};                   // Sealing
endpackage
module tb;
  import ibex_cheriot_pkg::*;
  cap_t c; decoded_cap_t d; bound_req_t br; bound_result_t bres; cap_clrperm_t cp;
  initial begin
    $display("DIGEST=%0d %0d %0d %0d %0d %0d", $bits(cap_t), $bits(decoded_cap_t), $bits(bound_req_t), $bits(bound_result_t), $bits(cap_clrperm_t), $bits(perms_t));
    $display("DIGEST=%h %h %h %h %h %h", NULL_CAP, NULL_DECODED_CAP, ROOT_DECODED_CAP_TX, ROOT_CAP_TX, ROOT_CAP_TM, ROOT_CAP_TS);
    $display("DIGEST=%h %h %h %h %h %h", PERM_MRW_IMSK, PERM_MRO_IMSK, PERM_MWO_IMSK, PERM_MDO_IMSK, PERM_EXE_IMSK, PERM_SEA_IMSK);
    c = ROOT_CAP_TM; d = ROOT_DECODED_CAP_TX;
    $display("DIGEST=%b %h %h %h %h %h %b %b", c.valid, c.cperms, c.otype, c.cexp, c.top, c.base, d.perms.EX, d.perms.SR);
    $display("DIGEST=%h %h %h %h %b", d.top33, d.base32, d.perms, d.cperms, d.cap_cor);
    bres = '0; bres.cap = d; bres.maska = 32'hffff_0000;
    $display("DIGEST=%h %b %h %h", bres.cap.perms, bres.cap.perms.MC, bres.cap.top, bres.maska);
    bres.cap.perms.LD = 1'b0; bres.cap.cperms = CPERMS_TS;
    $display("DIGEST=%h %h %h", bres.cap.perms, bres.cap.cperms, bres.cap);
    c = cap_t'(d);
    $display("DIGEST=%h %b", c, c == ROOT_CAP_TX);
    br = '0; br.exp1 = MAXEXP; br.explen = 6'd32;
    $display("DIGEST=%h %0d %0d", br, br.exp1, br.explen);
    #1 $finish;
  end
endmodule
"#, "35 112 56 176 3 12|000000000 0000000000000000000000000000 80000000000000000f595e3e0000 15e3e0000 17e3e0000 14e3e0000|064 060 044 000 160 000|1 3f 0 f 100 000 1 1|100000000 00000000 1eb 2f 00|1eb 1 100 ffff0000|1cb 27 80000000000000000e594e3e0000|15e3e0000 1|00000000601000 24 32");
}

#[test]
fn ns_pat() {
    // ns_pat_bad_key: verilator
    loud(
        "ns_pat_bad_key",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{cor: 2'b10, perms: '{U0: 1, SE: 0, zz: 2'b11}, valid: 1};
    $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected an assignment-pattern key naming a member of this struct, fou",
    );
    // ns_pat_count_mismatch: verilator
    loud(
        "ns_pat_count_mismatch",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{cor: 2'b10, perms: '{U0: 1, q: 2'b11}, valid: 1};
    $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected every packed-struct member given by name or by `default:` (IE",
    );
    // ns_pat_decl_init: verilator
    digest(
        "ns_pat_decl_init",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c = '{cor: 2'b11, perms: '{U0: 0, SE: 1, q: 2'b10}, valid: 1};
  cap_t d = '{default: '0};
  initial begin
    $display("DIGEST=%h %h", c, d);
  end

  initial #5 $finish;
endmodule
"#,
        "6d 00",
    );
    // ns_pat_default_fill: verilator
    digest(
        "ns_pat_default_fill",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{default: '0};  $display("DIGEST=%h", c);
    c = '{default: '1};  $display("DIGEST=%h", c);
    c = '{default: 0};   $display("DIGEST=%h", c);
    c = '{cor: 2'b10, default: '0}; $display("DIGEST=%h", c);
    c = '{perms: '{q: 2'b10, default: '1}, default: '0}; $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "00|7f|00|40|1c",
    );
    // ns_pat_default_one: verilator
    loud(
        "ns_pat_default_one",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{default: 1};  $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected a fill (`'0`/`'1`) or 0 as the `default:` of a pattern whose ",
    );
    // ns_pat_default_two: verilator
    loud(
        "ns_pat_default_two",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{default: 2'b10};  $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected a fill (`'0`/`'1`) or 0 as the `default:` of a pattern whose ",
    );
    // ns_pat_default_x: verilator — fills stay x/z (LRM §11.6); verilator is 2-state here, PRE flat control identical
    digest(
        "ns_pat_default_x",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{default: 'x};  $display("DIGEST=%b", c);
    c = '{cor: 2'b10, default: 'z}; $display("DIGEST=%b", c);
  end

  initial #5 $finish;
endmodule
"#,
        "xxxxxxx|10zzzzz",
    );
    // ns_pat_keyed: verilator
    digest(
        "ns_pat_keyed",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{cor: 2'b10, perms: '{U0: 1, SE: 0, q: 2'b11}, valid: 1};
    $display("DIGEST=%h", c);
    c = '{valid: 0, perms: '{q: 2'b01, default: 1}, cor: 2'b11};
    $display("DIGEST=%h", c);
    c = '{cor: 2'b01, perms: 4'b0110, valid: 0};
    $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "57|7a|2c",
    );
    // ns_pat_localparam: verilator
    digest(
        "ns_pat_localparam",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  localparam cap_t ROOT = '{cor: 2'b10, perms: '{U0: 1, SE: 1, q: 2'b01}, valid: 1};
  localparam cap_t NULLC = '{default: '0};
  localparam perms_t PM = '{SE: 1, default: 0};
  cap_t c;
  initial begin
    c = ROOT;
    $display("DIGEST=%h %h %h %b %h %0d", ROOT, NULLC, PM, c.perms.SE, ROOT.perms, ROOT.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "5b 00 4 1 d 1",
    );
    // ns_pat_pkg_param: verilator
    digest(
        "ns_pat_pkg_param",
        r#"package p;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  parameter cap_t ROOT = '{cor: 2'b10, perms: '{U0: 1, SE: 1, q: 2'b01}, valid: 1};
  parameter cap_t NULLC = '{default: '0};
  parameter cap_t ONE = '{default: '1};
  parameter perms_t PM = '{SE: 1, default: 0};
  parameter cap_t FLAT = '{cor: 2'b01, perms: 4'b1010, valid: 0};
  parameter cap_t CST = cap_t'(7'h55);
endpackage
module tb;
  import p::*;
  cap_t c;
  initial begin
    c = ROOT;
    $display("DIGEST=%h %h %h %h %h %h %b %h", ROOT, NULLC, ONE, PM, FLAT, CST, c.perms.SE, p::ROOT);
  end

  initial #5 $finish;
endmodule
"#,
        "5b 00 7f 4 34 55 1 5b",
    );
    // ns_pat_pkg_param_scoped: verilator
    digest(
        "ns_pat_pkg_param_scoped",
        r#"package p;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  parameter cap_t ROOT = '{cor: 2'b10, perms: '{U0: 1, SE: 1, q: 2'b01}, valid: 1};
endpackage
module tb;
  p::cap_t c;
  initial begin
    c = p::ROOT;
    $display("DIGEST=%h %h %b", c, p::ROOT, c.perms.q);
  end

  initial #5 $finish;
endmodule
"#,
        "5b 5b 01",
    );
    // ns_pat_positional: both oracles
    digest(
        "ns_pat_positional",
        r#"module tb;
  typedef struct packed { logic U0; logic SE; logic [1:0] q; } perms_t;
  typedef struct packed { logic [1:0] cor; perms_t perms; logic valid; } cap_t;
  cap_t c;
  initial begin
    c = '{2'b10, '{1'b1, 1'b0, 2'b11}, 1'b1};
    $display("DIGEST=%h", c);
    c = '{2'b01, 4'b0110, 1'b0};
    $display("DIGEST=%h", c);
  end

  initial #5 $finish;
endmodule
"#,
        "57|2c",
    );
}
