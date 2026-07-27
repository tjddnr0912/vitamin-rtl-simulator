//! Elaboration CAPS and poison sentinels — split out of `lib.rs` (mechanical move;
//! module-size policy). Every value here is a fail-closed bound: exceeding one is a
//! loud reject, never a truncated design.

use super::*;

/// in hostile input cannot explode the block arena. Above the cap → `ElabUnsupported`.
pub(crate) const REPEAT_UNROLL_CAP: u32 = 1024;

/// Hard cap on generate-for unroll iterations. A malformed/hostile
/// `for(i=0;i<HUGE;i=i+1)` cannot explode the arena: above this we emit
/// `ElabUnsupported` and stop unrolling. Mirrors `REPEAT_UNROLL_CAP`'s intent
/// (generate bodies can each contribute many nets, so the cap is conservative).
pub(crate) const GENERATE_UNROLL_CAP: u32 = 4096;

/// Hard cap on generate nesting depth (nested for/if/case/block). Guards against
/// pathological recursion; deep-nesting beyond this is deferred per PR scope.
pub(crate) const GENERATE_DEPTH_CAP: u32 = 32;

/// ELAB-ERR-CAP: soft cap on emitted error diagnostics. A broken construct
/// unrolled across a large/nested generate would otherwise emit one error per
/// iteration (thousands of identical lines). After this many, emission stops
/// (with one suppression notice); `had_error` stays latched so the run is still
/// loud (exit 1). 200 ≫ any realistic multi-error report (parser caps at 50).
pub(crate) const MAX_ELAB_ERRORS: usize = 200;

/// Hard cap on a single net's declared bit width. Above this we reject the decl
/// with `ElabUnsupported` rather than `vec![0u64; huge]` (which would OOM) or
/// overflow the `+1` width arithmetic. 2^20 bits = 16 KiB of planes per net —
/// generous for real RTL, hostile-input-safe. (COVERAGE verdict HIGH.)
pub(crate) const MAX_NET_WIDTH: u64 = 1 << 20;

/// GEN-NET-CAP: hard cap on the TOTAL net/variable arena. `GENERATE_UNROLL_CAP`
/// bounds each loop individually, but nested generates multiply
/// (1000 × 1000 = 1M live nets ≈ 670 MiB), so the aggregate needs its own budget
/// (the `MAX_NET_WIDTH` idiom, applied to count instead of width). 2^17 nets
/// (≈85 MiB of planes) ≫ any realistic v1 design; above it `add_net` no-ops and
/// the run is loud (the pathological 1000×1000 is still rejected).
pub(crate) const MAX_TOTAL_NETS: usize = 1 << 17;
/// P2-6: unpacked-array element cap (16M elements; with the 1 MiB-bit width cap
/// the worst legal net is still bounded far below an OOM-kill allocation).
pub(crate) const MAX_ARRAY_LEN: u64 = 1 << 24;

/// Poison NetId returned on an unresolvable reference. `u32::MAX` (not 0) so an
/// accidentally-surviving placeholder edge is detectable, never a silent alias
/// of the first real net. The whole IR is discarded on error anyway (had_error),
/// but a poison sentinel makes any future error-recovery path fail loud.
/// (COVERAGE verdict MEDIUM.)
pub(crate) const POISON_NET: u32 = u32::MAX;

/// Family D (r17): placeholder FuncId for a DEFERRED hierarchical function call
/// (`u1.f(x)`), patched to the callee's real per-instance FuncId by
/// `resolve_deferred_hier_call`. Like `POISON_NET`, it must never survive to the
/// engine on a clean run — an unresolved call latches `had_error` (IR discarded).
pub(crate) const POISON_FID: u32 = u32::MAX;

/// Base of the sentinel net-id range used for a DEFERRED hierarchical WRITE
/// target (`tb.dut.x = …`): the real net does not exist when the lvalue is
/// lowered (the child instance's nets are created later), so the `LvalChunk`
/// gets `HIER_WRITE_SENTINEL_BASE + i` and `resolve_deferred_hier_write` patches
/// it to the real NetId (or `POISON_NET`) once every instance is elaborated.
/// Far above any real net id and below `POISON_NET`, so a surviving sentinel is
/// detectable and never aliases a real net.
pub(crate) const HIER_WRITE_SENTINEL_BASE: u32 = 0xFF00_0000;

/// Base of the sentinel net-id range used for a DEFERRED hierarchical ELEMENT/
/// bit-select WRITE (`tb.dut.mem[i] = …`, `m.l.v[3] <= …`): like the whole-net
/// write the target net is created later, but here resolution must also REBUILD
/// the chunk's word/offset/width/kind from the resolved net's shape. A distinct
/// range below `HIER_WRITE_SENTINEL_BASE` keeps the two deferral lanes disjoint.
pub(crate) const HIER_SEL_WRITE_SENTINEL_BASE: u32 = 0xFE00_0000;

/// A poison `LvalChunk` (sentinel net, no select) emitted after a loud error while
/// rebuilding a deferred hierarchical element/bit-select write — never aliases a
/// real net (`POISON_NET` is above every real net id).
pub(crate) fn poison_chunk() -> ir::LvalChunk {
    ir::LvalChunk {
        net: POISON_NET,
        word: None,
        offset: None,
        width: None,
        kind: ir::SelKind::Bit,
    }
}
