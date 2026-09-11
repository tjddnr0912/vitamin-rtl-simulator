//! §2 Scoping (subroutine block-locals): frame-slot reservation for a subroutine's
//! `begin…end` block-locals — split out of `frames_reserve.rs`, which is over the
//! 1000-line policy already.
//!
//! The one thing this file exists to get right: a block-local's frame slot is keyed by
//! `self.fq(&decl.name.name)`, so two same-named sibling block-locals coalesced onto
//! ONE slot. They are reserved under the declaring block's `$blk$<lo>` segment instead
//! — the same segments the Logic-phase `Stmt::Block` arm (`stmt_main.rs`) lowers the
//! block body in, reproduced by `block_local_scope_prefix` from the span chain
//! `collect_block_local_decls_spanned` carries.

use super::*;

impl Elaborator<'_> {
    pub(crate) fn reserve_frame_block_locals(&mut self, body: &ast::Stmt, base_net: u32) -> u64 {
        let mut decls = Vec::new();
        crate::block_local::collect_block_local_decls_spanned(body, &mut Vec::new(), &mut decls);
        let mut auto_override = 0u64;
        for (chain, d) in &decls {
            // §2 Scoping (subroutine block-locals): a decl whose declaring block earned a
            // `$blk$` segment is reserved UNDER that segment, so two same-named sibling
            // block-locals stop coalescing onto one frame slot. A decl with no segment
            // takes the `None` arm, which is the pre-existing code verbatim.
            match self.block_local_scope_prefix(chain, d) {
                Some(seg) => {
                    let bits = self.with_scope(&seg, |s| s.reserve_block_local_decl(d, base_net));
                    auto_override |= bits;
                }
                None => auto_override |= self.reserve_block_local_decl(d, base_net),
            }
        }
        auto_override
    }

    /// One block-local declaration's frame slots, reserved under the CURRENT prefix.
    /// Returns the `automatic`-override bits for the slots it added.
    fn reserve_block_local_decl(&mut self, d: &ast::NetVarDecl, base_net: u32) -> u64 {
        let mut auto_override = 0u64;
        for decl in &d.names {
            let fq = self.fq(&decl.name.name);
            if let Some(&existing) = self.symbols.get(&fq) {
                // EXT2-H guard: a block-local UNPACKED ARRAY that shadows a
                // same-named outer scalar coalesces onto that (shape-blind) net —
                // mark the coalesced net so an element write `y[k]=v` stays loud,
                // not a silent scalar bit-write.
                if !decl.unpacked.is_empty() {
                    self.frame_array_local.insert(existing);
                }
                continue; // coalesce with an already-reserved formal/local
            }
            let slot = self.nets.len() as u32 - base_net;
            self.reserve_frame_local_decl(&decl.name.name, d, &decl.unpacked);
            if d.lifetime == Some(true) && slot < 64 {
                auto_override |= 1u64 << slot;
            }
        }
        auto_override
    }
}
