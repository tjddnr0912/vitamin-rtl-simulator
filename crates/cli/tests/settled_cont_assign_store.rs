//! The shapes that decide which arm of `NativeKernel::write_settled` a continuous
//! assign's settled value takes (§4.5.352).
//!
//! `write_settled` stores flat — straight to `NetArena::write_chunk_word` — when the
//! destination is a proven plain whole-net scalar whose value fits one arena word, and
//! routes through the general funnel otherwise. Both arms must produce the same bytes;
//! the fast one exists only because it skips work that was already proved unnecessary.
//!
//! ⚠️ WHY THE WIDE CASES ARE HERE. `plain_scalar_dest_of`'s "scalar" means WHOLE-NET, not
//! one bit — a 128-bit net satisfies it (`build_plain_scalar` has no width ceiling, and
//! §4.5.351 measured 128-bit destinations taking the sibling fast arm in the NBA region).
//! The only thing that keeps a wide destination off `store_plain_word` — whose contract is
//! "a destination that occupies a single arena word" — is `write_settled`'s
//! `value.width <= 64` guard, which holds because a whole-net lvalue makes
//! `value.width >= net width`. A mutation battery deleting each of `value.width <= 64`,
//! `s.words == 1`, `!s.is_real` and `s.width > 0` in turn found ALL FOUR to be equivalent
//! mutants — they survive the whole suite AND they survive this file (measured, not
//! assumed) — because each is implied by an earlier guard: the width pair by
//! `value.width >= net width`, the other two by `build_plain_scalar`, which already
//! requires `!is_real && width > 0`. They stay because the guard is spelled the same in
//! all three flat-store regions and a redundancy that fails CLOSED is cheaper than a
//! divergence. What these tests pin is therefore the IMPLICATION, not the redundancy: a
//! future change that widens the flat store, drops a `build_plain_scalar` clause, or
//! narrows the value's context width fails HERE rather than silently writing one word of
//! a wide net.
//!
//! ORACLE: iverilog 13.0 (every value below is vvp-pinned).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vita_scas_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success(), "expected success:\n{all}");
    // The run trailer ("simulation ended (…) at time N") is the CLI's, not the design's.
    let mut kept = String::new();
    for l in String::from_utf8_lossy(&out.stdout).lines() {
        if !l.starts_with("simulation ended") {
            kept.push_str(l);
            kept.push('\n');
        }
    }
    kept
}

// ───────────────────── the fast arm: plain whole-net, one word ─────────────────────

#[test]
fn plain_scalar_destinations_settle_to_the_oracle_values() {
    // 1, 8, 32, 63, 64 bits — the whole span the one-word store owns, including the
    // exact boundary. Signed and unsigned, and one net fed by another so the fixpoint
    // has to run a second pass (the `changed` bool the fast arm now returns).
    let o = run("module t;\n\
         reg a; wire y1; assign y1 = ~a;\n\
         reg [7:0] b; wire [7:0] y8; assign y8 = b + 8'd1;\n\
         reg signed [31:0] c; wire signed [31:0] y32; assign y32 = c >>> 4;\n\
         reg [62:0] d; wire [62:0] y63; assign y63 = d ^ {63{1'b1}};\n\
         reg [63:0] e; wire [63:0] y64; assign y64 = e + 64'd1;\n\
         wire [63:0] chain; assign chain = y64 ^ 64'hFFFF_FFFF_FFFF_FFFF;\n\
         initial begin\n\
           a = 1'b0; b = 8'hFF; c = -32'sd64; d = 63'h1234_5678; e = 64'hFFFF_FFFF_FFFF_FFFF;\n\
           #1 $display(\"%b %h %0d %h %h %h\", y1, y8, y32, y63, y64, chain);\n\
           a = 1'b1; b = 8'h7F; c = 32'sd64; d = 63'd0; e = 64'd0;\n\
           #1 $display(\"%b %h %0d %h %h %h\", y1, y8, y32, y63, y64, chain);\n\
         end\n\
         endmodule\n");
    // vvp: same two lines.
    assert_eq!(
        o,
        "1 00 -4 7fffffffedcba987 0000000000000000 ffffffffffffffff\n\
         0 80 4 7fffffffffffffff 0000000000000001 fffffffffffffffe\n"
    );
}

// ───────────── the else arm: shapes the fast arm must NOT take ─────────────

#[test]
fn wide_plain_destinations_take_the_routed_arm_and_stay_whole() {
    // 65, 100 and 128 bits. `plain_scalar_dest_of` ADMITS all three (whole-net, not
    // heap/frame/class/real/2-state) — only `value.width <= 64` sends them to the funnel.
    // If the flat store ever reached them it would write the low word and leave the rest,
    // so the top bit of each is what fails first.
    let o = run("module t;\n\
         reg [64:0] a; wire [64:0] u; assign u = a ^ 65'h1_0000_0000_0000_0000;\n\
         reg [99:0] b; wire [99:0] v; assign v = b + 100'd3;\n\
         reg [127:0] c; wire [127:0] w; assign w = c | 128'd1;\n\
         initial begin\n\
           a = 65'h1_0000_0000_0000_0001;\n\
           b = 100'h5_5555_5555_5555_5555_5555;\n\
           c = 128'hDEAD_BEEF_0000_0000_1234_5678_9ABC_DEF0;\n\
           #1 $display(\"%h %h %h\", u, v, w);\n\
           a[64] = 1'b0; b[99] = 1'b1; c[127] = 1'b0;\n\
           #1 $display(\"%h %h %h\", u, v, w);\n\
         end\n\
         endmodule\n");
    // vvp: same two lines. The second line is the one that moves a bit ABOVE word 0.
    assert_eq!(
        o,
        "00000000000000001 0000555555555555555555558 deadbeef00000000123456789abcdef1\n\
         10000000000000001 8000555555555555555555558 5eadbeef00000000123456789abcdef1\n"
    );
}

#[test]
fn non_plain_destinations_take_the_routed_arm() {
    // A part-select LHS, a concat LHS, an unpacked-array-element LHS and a 2-state net —
    // every one of them a shape `plain_scalar_dest_of` rejects, so the funnel must still
    // slice them. A fast arm that admitted any of these would write the wrong bits.
    let o = run("module t;\n\
         reg [7:0] s; wire [7:0] p; assign p[5:2] = s[3:0];\n\
         reg [3:0] hi, lo; wire [7:0] cc; assign {cc[7:4], cc[3:0]} = {hi, lo};\n\
         reg [7:0] m [0:1]; wire [7:0] am; assign am = m[1];\n\
         reg [7:0] q; bit [7:0] two; always_comb two = q | 8'h0F;\n\
         initial begin\n\
           s = 8'hA5; hi = 4'h3; lo = 4'hC; m[1] = 8'h99; q = 8'hx0;\n\
           #1 $display(\"%b %h %h %b\", p, cc, am, two);\n\
           s = 8'h5A; hi = 4'hF; m[1] = 8'h11; q = 8'hA5;\n\
           #1 $display(\"%b %h %h %b\", p, cc, am, two);\n\
         end\n\
         endmodule\n");
    // vvp agrees on every column, `bit` included (§6.11.3: a 2-state net holds no x, so
    // `8'hx0 | 8'h0F` reads 0000_1111 and not xxxx_1111).
    assert_eq!(
        o,
        "zz0101zz 3c 99 00001111\n\
         zz1010zz fc 11 10101111\n"
    );
}

// ───────────────────── the delayed set the pre-filter narrows ─────────────────────

#[test]
fn delayed_assigns_at_every_index_position_still_fire_in_declaration_order() {
    // `schedule_delayed_cas` iterates a PRECOMPUTED index list now. The set is sparse and
    // non-contiguous here on purpose — delayed assigns sit at the first, middle and last
    // declaration positions with undelayed ones interleaved — so a filter that lost an
    // index, reordered the survivors, or shifted the mapping between `ci` and the
    // per-assign tables (`last_ca`, `ca_gen`) shows up as a missing or late value.
    let o = run("module t;\n\
         reg x; wire d0, u1, d2, u3, d4;\n\
         assign #2 d0 = x;\n\
         assign    u1 = x;\n\
         assign #4 d2 = ~x;\n\
         assign    u3 = ~x;\n\
         assign #6 d4 = x;\n\
         initial begin\n\
           x = 1'b0; #10 x = 1'b1;\n\
           #1 $display(\"%0t %b%b%b%b%b\", $time, d0, u1, d2, u3, d4);\n\
           #2 $display(\"%0t %b%b%b%b%b\", $time, d0, u1, d2, u3, d4);\n\
           #2 $display(\"%0t %b%b%b%b%b\", $time, d0, u1, d2, u3, d4);\n\
           #2 $display(\"%0t %b%b%b%b%b\", $time, d0, u1, d2, u3, d4);\n\
         end\n\
         endmodule\n");
    // Each delayed net flips exactly `d` after the t=10 change (12, 14, 16). BOTH
    // oracles agree cell-for-cell (iverilog 13.0 and verilator 5.050 --binary --timing);
    // the author's hand-predicted first line was wrong and the oracles corrected it.
    assert_eq!(
        o,
        "11 01100\n\
         13 11100\n\
         15 11000\n\
         17 11001\n"
    );
}

#[test]
fn a_design_with_no_delayed_assign_still_settles_its_plain_ones() {
    // The case the pre-filter reduces to an EMPTY iteration set — picorv32's shape, and
    // the one worth 7.2% of that run. Nothing about the settle may change.
    let o = run("module t;\n\
         reg [7:0] a, b; wire [7:0] s, x, y, z;\n\
         assign s = a + b; assign x = s ^ 8'h0F; assign y = x & a; assign z = y | b;\n\
         initial begin\n\
           a = 8'h3C; b = 8'hC3; #1 $display(\"%h %h %h %h\", s, x, y, z);\n\
           a = 8'hFF;           #1 $display(\"%h %h %h %h\", s, x, y, z);\n\
         end\n\
         endmodule\n");
    assert_eq!(o, "ff f0 30 f3\nc2 cd cd cf\n");
}
