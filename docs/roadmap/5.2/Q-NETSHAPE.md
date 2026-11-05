(§5.2 Q-NETSHAPE · rank — F · source Q-DECLTYPE plan (W/qdecltype/plan/PLAN.txt P1 S3)) (one net-shape record in elaborate, written only by add_net):
* slot: after the first batch's boundary verify (Q-MEMBSRC's carrier, 46, 24, 31), beside Q-SIDETABLES, which goes first is the owner's call (owner, 2026-11-05: Q-DECLTYPE's S3 as its own row)
* scope: one `BTreeMap<u32, NetShape>` written only by `add_net(name, NetVar, NetShape)`, output byte-identical to the base (the `.velab` compared with its composite input hash, 1-based bytes 42-73, zeroed)
* evidence: elaborate keeps a net's shape in separate per-net maps written after `add_net` returns (net_decl_range, net_decl_neg_lsb, net_decl_asc_lsb, packed_dims, array_dims, intro_kind, unpacked_array_nets, typedef_elem_arrays, the two-state set ; 55 `add_net` sites in 26 files, Q-DECLTYPE grounding G2)
* evidence: the residual twin `elaborate/src/netdecl.rs:745-766` of `array_geom.rs:419-435` (`record_declared_bounds_for`) is rewritten by this row, not alone
* note: premise, unmeasured, deciding the row: every shape-map write targets the NetId its own `add_net` just returned, including writes through helpers (`array_geom.rs:903-904`, `frames_reserve.rs:256` / `:263`) and onto merged nets (a non-ANSI port with a later declaration, a block-local coalesce)
* note: its sidecar projection (`api.rs:187` net_decl_ranges, `:203` two_state_nets ; `cli/src/staged.rs:706` / `:709`) is the seam Q-SIDETABLES restructures: one census serves both
* note: filling a hole (§0 11's h1 h2 h4 bounds) moves values and is not this row
* note: a net declaration's `dtype.kind` `TypeKind::Kind(k)` (Q-DECLTYPE's S12, §4.5.635) is PRE's lossy kind and, under a type parameter, `shape_param`'s default kind, so the first reader of it as the declared kind measures that first (S12 soundness lens S-F8)
* source: W/qdecltype/plan/PLAN.txt P1 S3, W/qdecltype/GROUND.txt G2
