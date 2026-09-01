//! net-kind helpers — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Self-determined bit width of a net/var KIND + optional literal RANGE; atoms are
/// fixed-width, a vector kind folds a LITERAL `[msb:lsb]`. `None` for real/string/
/// handle (not bit arithmetic) or an unfoldable range.
pub(crate) fn ast_kind_range_width(
    kind: ast::NetVarKind,
    range: Option<&ast::Range>,
) -> Option<u32> {
    use ast::NetVarKind::*;
    match kind {
        Integer | Int => Some(32),
        Byte => Some(8),
        Shortint => Some(16),
        Longint | Time => Some(64), // `time` is a 64-bit integral (bit-vector) type
        Real | Realtime | Event | String | ClassHandle => None,
        _ => match range {
            None => Some(1),
            Some(r) => {
                let msb = ast_decimal_lit_i64(&r.msb)?;
                let lsb = ast_decimal_lit_i64(&r.lsb)?;
                u32::try_from(msb.abs_diff(lsb) + 1).ok()
            }
        },
    }
}

/// Is `kind` a BIT-VECTOR (integral) type — i.e. arithmetic evaluated at a context
/// width? `real`/`realtime`/`string`/`event`/class-handle are NOT (no context-width
/// extension applies), so a widening assignment to one is never mis-lowered this way.
/// May a subroutine formal of this declared type receive the §13.5.3 real→integral
/// narrowing at its bind?
///
/// One shape is refused: `input time signed`. `time` is 64-bit UNSIGNED (§6.11.2),
/// so `kind_signedness` forces the formal NET unsigned and DROPS an explicit
/// qualifier — and on the paths where the formal IS a net the body then reads it
/// unsigned no matter what the bind computed. Both oracles read
/// `input time signed k` as SIGNED (`-4`), so narrowing there would turn the
/// pre-slice answer (correct only because nothing narrowed at all) into a wrong
/// one on three paths at once, while the net-backed static-task path stayed wrong.
/// Refusing keeps every `time signed` cell EXACTLY at its pre-slice value, and a
/// plain `time` formal still gets the narrowing (measured: loud → 44, both oracles).
///
/// The root — a dropped signing qualifier on a `time` declaration — is ROADMAP §2's;
/// this is a decline, not a fix, and it is spelled once for all three binds.
pub(crate) fn formal_bind_may_narrow(kind: ast::NetVarKind, declared_signed: bool) -> bool {
    !(matches!(kind, ast::NetVarKind::Time) && declared_signed)
}

pub(crate) fn ast_kind_is_bit_vector(kind: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    !matches!(kind, Real | Realtime | Event | String | ClassHandle)
}

/// hdl-ast has 18 net/var kinds; sim-ir freezes only 4. Aliases collapse to the
/// closest 4-state kind; unsupported kinds still map to Wire so references
/// resolve (the call site emits `ElabUnsupported`).
pub(crate) fn map_net_kind_or_wire(k: ast::NetVarKind) -> ir::NetKind {
    use ast::NetVarKind::*;
    match k {
        Reg => ir::NetKind::Reg,
        Logic => ir::NetKind::Logic,
        Integer => ir::NetKind::Integer,
        // `real`/`realtime` → IEEE-754 f64 net (64-bit, signed, 2-state).
        Real | Realtime => ir::NetKind::Real,
        // `time` → 64-bit unsigned 4-state VARIABLE. The frozen NetKind has no
        // Time variant; Reg carries the same legality (procedural-assign ok,
        // user `assign` rejected) and 4-state all-X init. Width/signedness come
        // from range_to_dims (64, unsigned).
        Time => ir::NetKind::Reg,
        // named event → its 64-bit counter reg (v5 batch B desugar).
        Event => ir::NetKind::Reg,
        // SVPART 2-state types → Reg storage (procedural-assignable, user `assign`
        // rejected). Width/sign from range_to_dims, 2-state 0-init from default_init.
        Bit | Byte | Shortint | Int | Longint => ir::NetKind::Reg,
        // N7: a class handle is a 32-bit unsigned integer reg holding an object-id
        // (0 = null); the object itself lives in the engine `class_heap`. Reg
        // storage = procedural-assignable, user `assign` rejected.
        ClassHandle => ir::NetKind::Integer,
        // Wire + all net aliases (Tri/Uwire/Wand/...) behave as Wire in v1.
        _ => ir::NetKind::Wire,
    }
}

/// N1: net kind for a function/task frame-local DECLARATION (`body_decls` /
/// block-local `begin string s; …`). Unlike an input formal — which keeps the
/// 1-bit Wire slot and is filled by the call-site `str_params` mask
/// (`map_net_kind_or_wire`) — a frame-local `string` is WRITTEN in the body
/// (`s = $sformatf(...)`), so a Wire target would fail the procedural-assign
/// check (E3018, round-14 V1). It needs a real heap-backed `NetKind::String`
/// slot, exactly like an output formal. Every non-string type is unchanged.
pub(crate) fn frame_local_net_kind(k: ast::NetVarKind) -> ir::NetKind {
    if matches!(k, ast::NetVarKind::String) {
        ir::NetKind::String
    } else {
        map_net_kind_or_wire(k)
    }
}

pub(crate) fn net_is_variable(k: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    matches!(
        k,
        Reg | Logic | Integer | Time | Bit | Byte | Shortint | Int | Longint | ClassHandle
    )
}

/// SVPART: the X-free 2-state integer types (`bit`/`byte`/`shortint`/`int`/
/// `longint`). The engine coerces X/Z→0 on every write to these nets.
pub(crate) fn net_kind_is_two_state(k: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    matches!(k, Bit | Byte | Shortint | Int | Longint)
}

/// True iff `k` is a VARIABLE kind (reg/logic/integer/real/time/event + the
/// 2-state integer types) as opposed to a net (wire) — a variable's declaration
/// initializer is a one-time value at time 0, not a continuous driver (§6.8).
pub(crate) fn netvar_kind_is_var(k: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    matches!(
        k,
        Reg | Logic
            | Integer
            | Real
            | Realtime
            | Time
            | Event
            | Bit
            | Byte
            | Shortint
            | Int
            | Longint
    )
}

/// Whether a kind is modeled in v1 without an `ElabUnsupported` note. Pure
/// aliases (Tri/Uwire) are accepted silently; resolution nets (wand/wor/...)
/// are flagged (still mapped to Wire so the arena stays valid).
pub(crate) fn net_kind_supported(k: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    matches!(
        k,
        Wire | Tri
            | Uwire
            | Wand
            | Wor
            | Reg
            | Logic
            | Integer
            | Real
            | Realtime
            | Time
            | Event
            | String
            | Bit
            | Byte
            | Shortint
            | Int
            | Longint
            | ClassHandle
    )
}

/// Time-0 default `init`: variables (reg/logic/integer) start all-X; nets start
/// all-Z. `(v,u)`: X=`01`, Z=`11`.
pub(crate) fn default_init(kind: ast::NetVarKind, width: u32) -> ir::BitPacked {
    // A real default = +0.0 = all-zero bits, never X (it is always 2-state).
    if matches!(kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime) {
        return ir::BitPacked {
            val: vec![0],
            unk: vec![0],
        };
    }
    // A named-event counter starts at ZERO, never X: `e = e + 1` on an all-X
    // start would stay X forever and no `@(e)` edge could ever fire.
    if matches!(kind, ast::NetVarKind::Event) {
        return ir::BitPacked {
            val: vec![0],
            unk: vec![0],
        };
    }
    // N7: a class handle defaults to `null` = object-id 0 (NOT X) — IEEE §8.4;
    // `h == null` must be TRUE for an uninitialized handle.
    if matches!(kind, ast::NetVarKind::ClassHandle) {
        return ir::BitPacked {
            val: vec![0],
            unk: vec![0],
        };
    }
    // SVPART: 2-state types are X-free — they default-initialise to 0, never X.
    if matches!(
        kind,
        ast::NetVarKind::Bit
            | ast::NetVarKind::Byte
            | ast::NetVarKind::Shortint
            | ast::NetVarKind::Int
            | ast::NetVarKind::Longint
    ) {
        let nwords = ((width as usize).div_ceil(64)).max(1);
        return ir::BitPacked {
            val: vec![0u64; nwords],
            unk: vec![0u64; nwords],
        };
    }
    let nwords = (((width as usize) + 63) / 64).max(1);
    let is_var = matches!(
        kind,
        ast::NetVarKind::Reg
            | ast::NetVarKind::Logic
            | ast::NetVarKind::Integer
            | ast::NetVarKind::Real
            | ast::NetVarKind::Realtime
            | ast::NetVarKind::Time
    );
    let mut val = vec![0u64; nwords];
    let mut unk = vec![0u64; nwords];
    for i in 0..(width as usize) {
        let w = i / 64;
        let off = i % 64;
        unk[w] |= 1u64 << off; // X and Z both have unk=1
        if !is_var {
            val[w] |= 1u64 << off; // Z has val=1; X has val=0
        }
    }
    ir::BitPacked { val, unk }
}

/// Resize a `BitPacked` from `from_w` to `to_w` bits. Truncates or zero-/sign-/
/// x-/z-extends per IEEE §3.5.1 (extend with the MSB *state*; sign-extend a `1`
/// only when `signed`). Used for net initializers.
pub(crate) fn resize_bits(
    src: &ir::BitPacked,
    from_w: u32,
    to_w: u32,
    signed: bool,
) -> ir::BitPacked {
    let nwords = (((to_w as usize) + 63) / 64).max(1);
    let mut val = vec![0u64; nwords];
    let mut unk = vec![0u64; nwords];
    let get = |plane: &[u64], i: usize| -> bool {
        plane
            .get(i / 64)
            .map(|w| (w >> (i % 64)) & 1 == 1)
            .unwrap_or(false)
    };
    // MSB state of the source (for extension).
    let msb_i = from_w.saturating_sub(1) as usize;
    let msb_v = get(&src.val, msb_i);
    let msb_u = get(&src.unk, msb_i);
    let (ext_v, ext_u) = match (msb_v, msb_u) {
        (false, true) => (false, true),   // X → x-extend
        (true, true) => (true, true),     // Z → z-extend
        (true, false) => (signed, false), // 1 → sign-extend only if signed
        _ => (false, false),              // 0 → zero-extend
    };
    for i in 0..(to_w as usize) {
        let (v, u) = if (i as u32) < from_w {
            (get(&src.val, i), get(&src.unk, i))
        } else {
            (ext_v, ext_u)
        };
        if v {
            val[i / 64] |= 1u64 << (i % 64);
        }
        if u {
            unk[i / 64] |= 1u64 << (i % 64);
        }
    }
    ir::BitPacked { val, unk }
}

impl Elaborator<'_> {
    /// Per-NetId fully-qualified name table for the VCD writer, built by inverting
    /// the FQ-name → NetId `symbols` map (`"top.dut.q"`). A net with no symbol entry
    /// (anonymous/implicit) falls back to `n{id}`. BTreeMap iteration is sorted, so
    /// a net mapped by several aliases keeps the lexicographically smallest FQ name
    /// (its canonical declaration path). Order-independent of arena order → 3-OS
    /// stable. Computed before `finish()` (which moves `self.nets`/`self.symbols`).
    pub(crate) fn net_name_table(&self) -> Vec<String> {
        let mut names = vec![String::new(); self.nets.len()];
        for (fq, &id) in &self.symbols {
            if let Some(slot) = names.get_mut(id as usize) {
                if slot.is_empty() {
                    *slot = fq.clone();
                }
            }
        }
        for (i, n) in names.iter_mut().enumerate() {
            if n.is_empty() {
                *n = format!("n{i}");
            }
        }
        names
    }

    // ── parameter binding (defaults + overrides; FQ-keyed) ──────────
    /// Bind a module's params for the current instance scope: each declared
    /// param's default (const-eval'd IN ORDER so a later param sees earlier ones),
    /// then overlay the instantiation overrides (positional by index, named by
    /// name). Localparams are NOT overridable. Params are keyed by FQ name so two
    /// instances with different `WIDTH` coexist. Returns the prior FQ→value
    /// entries so siblings/ancestors are restored on exit.
    ///
    /// The instantiation overrides are ALREADY resolved in the PARENT scope (Fix 1
    /// / Finding M1), so a `child #(.W(PARENT_W))` override carries the parent's
    /// `PARENT_W` value — no longer folds to 0 in the child scope.
    /// Coerce a folded parameter value to its DECLARED type's (width, signedness),
    /// matching IEEE 1800 §6.20 param typing: a ranged or integer-typed param
    /// truncates to its width and sign-extends when signed (so `parameter byte B =
    /// 200` is -56, `parameter signed [7:0] = 8'hA5` is -91); an UNSIZED param (no
    /// range, untyped) keeps its full value (the common width-defining `parameter W
    /// = 8`). Real-typed params are not integer-coerced. `int`/`integer` are signed
    /// by default (unsigned only with an explicit `unsigned`); `time` keeps its value.
    /// The DECLARED `(width, signed)` of a parameter, when determinate: an
    /// explicit `[msb:lsb]` range (foldable bounds) or `integer`/`int` (32-bit).
    /// `None` for an untyped/unsized param (width inferred from its value) or a
    /// `real`/`time` (no fixed packed width here). Single source of truth shared
    /// by value coercion and the typed-param read-width (`param_meta`).
    /// The DECLARED WIDTH of an enum's base type (`enum logic [3:0]` → `4`), so
    /// a label materializes at its real self-width inside a concat/replication
    /// (`{4'h5, STATE}`) instead of the value-inferred 32 bits. An implicit-base
    /// enum (`enum {A,B}` → 32-bit `int`) or an unfoldable bound returns `None`
    /// (value-inferred width, unchanged). Signedness is decided PER LABEL by its
    /// value (see the call sites): the enum VARIABLE's whole-value signedness is
    /// now captured into `TypeInfo.signed` (§4.5.153), but the AST enum node carries
    /// only the base RANGE (not its sign), so this LABEL-width path derives sign PER
    /// LABEL by its value — mirroring the value-inferred sign the local-param path
    /// already used (`const_param_expr`: unsigned for `v ≥ 0`, signed for `v < 0`).
    /// That keeps a negative label's arithmetic correct (`A=-2` stays -2), narrowing
    /// only the WIDTH. Deriving sign from the base instead would silently flip a
    /// negative label to a large unsigned value (`A=-2` → 14).
    pub(crate) fn enum_base_width(&self, base: &Option<ast::Range>) -> Option<u32> {
        let r = base.as_ref()?;
        match (
            self.const_eval_in_scope(&r.msb),
            self.const_eval_in_scope(&r.lsb),
        ) {
            (Some(m), Some(l)) => Some(m.abs_diff(l) as u32 + 1),
            _ => None,
        }
    }

    /// §6.19: an enum label whose value does not fit its base type is an ERROR, and
    /// this is the ELABORATE twin of the check in `hdl-parser/src/typedefs.rs`.
    ///
    /// ⚠️⚠️ The parser's copy can only run when BOTH base bounds are bare literals —
    /// it folds with `const_lit`, which knows nothing about parameters. So
    /// `typedef enum logic [W-1:0] { EA = 32'hAB34 } e_t;` with `-GW=8` was accepted at
    /// exit 0 where iverilog (*"value that is too large"*) and verilator
    /// (`ENUMITEMWIDTH`) both REJECT the design. By elaborate the bound is folded, so
    /// the same question can finally be asked.
    ///
    /// ⚠️ It does NOT try to skip the shapes the parser already policed, and that is a
    /// correction: the first version mirrored the parser's fold with a local
    /// `is_literal`, and the mirror drifted immediately — `const_lit` folds `2+1` but
    /// not `(3)`, and it requires a DECIMAL literal, so `[(3):0]` and `[8'd7:0]` fell
    /// between the two checks and stayed fail-open, which is the exact hole this
    /// function exists to close. There is nothing to skip: a parser error HALTS the
    /// pipeline, so a base the parser rejects never reaches elaborate, and one mistake
    /// still prints once. (`hdl-parser` is not a dependency of `elaborate`, so sharing
    /// the predicate rather than mirroring it was not available.)
    ///
    /// The bounds test itself is the parser's, verbatim in intent: at width 64 the
    /// distinction is PROVENANCE rather than magnitude — an explicitly written negative
    /// in an unsigned 64-bit base is an error, while an AUTO-INCREMENTED label that
    /// wraps past `64'sh7FFF…` is a legal `logic [63:0]` pattern iverilog accepts.
    pub(crate) fn check_enum_label_fits(
        &mut self,
        base: &Option<ast::Range>,
        enum_signed: bool,
        label: &str,
        v: i64,
        explicit: bool,
    ) {
        if base.is_none() {
            return; // a base-less enum is `int`, which the parser sizes itself
        }
        // ⚠️ `*w < 64`, and 64 is not an off-by-one. The label VALUE is an `i64`, so at
        // 64 bits the union range below is `[-2^63, 2^64-1]` — every i64 is inside it and
        // the check cannot fire. Above 64 it is worse than inert: `1i128 << w` WRAPS at
        // 128 and overflows past that, which made a legal `enum logic [W-1:0]` with
        // W=128 report `the widest range is [-170…728, 0]` and reject a label of 5 that
        // iverilog, verilator and vita's own previous build all accept — and panic in a
        // debug build from w=127 up. Two review lenses found it independently.
        let Some(w) = self.enum_base_width(base).filter(|w| *w < 64) else {
            return;
        };
        let _ = explicit;
        // ⚠️⚠️ The bound is the UNION of the signed and unsigned readings, which is
        // WEAKER than the parser's, and deliberately so: the two folds disagree about
        // what a based literal MEANS. `const_lit` reads `8'shFF` as the pattern 255;
        // `const_eval_in_scope` reads it as −1. The parser can afford the narrow range
        // because it sees the literal it folded; by elaborate the provenance is gone,
        // and a `logic [W-1:0]` base with `A = 8'shFF` is a legal design that must not
        // be rejected (`enum_sized_label.rs` pins it at 255). So this only reports a
        // value that fits under NEITHER reading — which is still every cell row 12 was
        // about, because those overflow the width outright.
        //
        // ⚠️ The consequence is an asymmetry worth knowing: a negative label on an
        // UNSIGNED base is loud when the base bounds are literals (the parser) and
        // accepted when they are not (here). Closing it needs the sign PROVENANCE the
        // constant domain does not carry — the same wall ROADMAP §2 row 14 stops at.
        let vi = v as i128;
        let (lo, hi): (i128, i128) = (-(1i128 << (w - 1)), (1i128 << w) - 1);
        let _ = enum_signed;
        if vi < lo || vi > hi {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "enum label `{label}` = {v} does not fit its {w}-bit base type (§6.19); \
                     no reading of a {w}-bit value reaches it (the widest range is \
                     [{lo}, {hi}])"
                ),
            );
        }
    }

    /// The enum base's DECLARED `(lo, width, ascending)` — the enum-label twin of
    /// [`Self::param_decl_range_opt`], and the reason a label can join the constant
    /// domain's select fold at all.
    ///
    /// ⚠️ A label's width is a DECLARED fact, exactly like a parameter's: it comes from
    /// the enum's base type, never from the label's value. That is the whole
    /// provenance question `param_range` exists to answer, and without an entry there
    /// `logic [EA[7:0]-1:0] v;` declared **one bit** at exit 0 where both oracles
    /// declare 52 — while the RUNTIME read of the same `EA[7:0]` was already right in
    /// all three tools.
    ///
    /// ⚠️⚠️ **THE SECOND PREREQUISITE IS THE CONSUMER'S, NOT §6.19's.** §4.5.373 showed
    /// that a declared width is only usable if the stored value is CANONICAL at it, and
    /// the first draft of this function claimed §6.19 supplied that: a label outside its
    /// base range is a loud `E2002` (`enum_label_range.rs`). The adversarial review
    /// measured the claim and it is FALSE. That check lives in the PARSER
    /// (`hdl-parser/src/typedefs.rs`) and is fail-open twice over — it gives up on the
    /// whole enum after the first label it cannot fold itself, and it skips entirely
    /// when a bound is not a bare literal — while this fold runs on
    /// `const_eval_in_scope`, which is strictly stronger (parameters, package
    /// constants, constant functions). `module top #(parameter P = 4); typedef enum
    /// logic [P-1:0] { A = 300 } e_t;` stores 300 against a recorded width of 4, at
    /// exit 0. Both adversarial lenses reached this independently, from opposite ends:
    /// the parser's check needs a BARE LITERAL bound, so a parameterised base — set in
    /// the header, by an instance override, or by `-G` — never reaches it, and both
    /// oracles REJECT those designs while vita exits 0.
    ///
    /// ⚠️ And even where the check does fire the value is canonical only MODULO SIGN:
    /// `enum logic [7:0] { EA = -8'sd2 }` — an unsigned base, i.e. exactly a base this
    /// function records — stores `-2` where both oracles read 254, while `param_meta`
    /// marks the label signed through its `|| v < 0` clause. Pre-existing and tracked
    /// separately (ROADMAP §2); it matters here as the second reason the premise cannot
    /// carry the argument.
    ///
    /// What actually makes this sound is that **every consumer narrows to the recorded
    /// width before using the value**: `select_base_at_declared` masks
    /// (`const_select.rs`) and `narrow_param_bits` resizes 64 → w (`const_wide.rs`).
    /// §6.19 is a second line of defence, not the argument. Do not add a consumer that
    /// reads `params` at this width WITHOUT narrowing — and note that the one operation
    /// which would AMPLIFY rather than mask is a width above 64, which is why the cap
    /// below is not decoration.
    ///
    /// A base-less `enum {…}` is `int` (§6.19), so it reports a 32-bit zero-LSB range —
    /// the same substitution [`Self::enum_base_width`]'s callers already make.
    ///
    /// ⚠️⚠️ A NON-ZERO declared LSB and an ASCENDING base DECLINE, and not for the
    /// usual representational reason: **the oracles split on them.** With
    /// `typedef enum logic [39:8] { EA = 32'hAB34 }`, `EA[15:8]` is **171** in iverilog
    /// (which reads the label as a plain value of the base's WIDTH, indexed from 0) and
    /// **52** in verilator (which honours the declared LSB, as both do for a NET of that
    /// type). Recording the range would install verilator's reading as vita's answer on
    /// an axis where there is no agreement to appeal to; declining leaves the cell
    /// exactly where it was. A zero-LSB base — every spelling anyone writes, including
    /// `int`, `byte` and the base-less form — is the case the two agree on, and the
    /// offset normalization is the identity there, so what the entry actually buys is
    /// the DECLARED WIDTH the select fold needs.
    ///
    /// A NEGATIVE declared bound declines as well — subsumed by the `lo != 0` test
    /// below, since a negative low bound is not zero. (An earlier draft wrote the sign
    /// test separately and credited it; the review showed it could never be the
    /// deciding one.)
    ///
    /// ⚠️ The `1 ..= 64` cap is load-bearing and is the only guard against the
    /// amplifying direction. `narrow_param_bits` EXTENDS the stored i64 to the recorded
    /// width (sign-extending a label of a signed enum), so a width above 64 would turn
    /// a value the const domain cannot represent into invented bits, and a width of
    /// ZERO — reachable in a release build, where `m.abs_diff(l) + 1` wraps for a bound
    /// near `u32::MAX` instead of panicking — would reach `fold_self_bits` with a
    /// zero-width operand where it used to decline. Its narrow twin
    /// `select_base_at_declared` already refuses both; this refuses them at the source,
    /// for every consumer at once.
    pub(crate) fn enum_base_range(&self, base: &Option<ast::Range>) -> Option<DeclRange> {
        let Some(r) = base.as_ref() else {
            return Some((0, 32, false));
        };
        let m = self.const_eval_in_scope(&r.msb)?;
        let l = self.const_eval_in_scope(&r.lsb)?;
        // Descending, zero-LSB, and no wider than the i64 constant domain can carry.
        if m.min(l) != 0 || m < l {
            return None;
        }
        let w = m.abs_diff(l) as u32 + 1;
        if w == 0 || w > 64 {
            return None;
        }
        Some((0, w, false))
    }

    /// Gap B (round-5): register a function/task's body-local `typedef enum` labels
    /// as integer constants under the CURRENT scope (`self.cur_prefix`), returning a
    /// save-list for `restore_params` to unwind afterwards. Mirrors the module-scope
    /// enum-label loop (3c) exactly: an explicit `LABEL = expr` const-folds (and
    /// resets the running counter to `expr+1`); an implicit label takes the counter.
    /// The caller scopes the registration to the body lowering so the labels are
    /// visible to `A`/`B` reads inside the body but do NOT leak to the module: the
    /// FRAME path registers under the `$func$<name>` segment (innermost-wins via
    /// `walk_scopes_key`), the INLINE path under the caller prefix bounded by the
    /// reduction. Empty `body_enums` (the common case) returns an empty save-list →
    /// byte-identical.
    ///
    /// A label whose name also names a `begin/end` BLOCK-LOCAL in the same body is
    /// loud-rejected: a body label registers as an enclosing-scope const, and vita
    /// resolves an enclosing const OVER an inner block-local net (a pre-existing
    /// resolution-order limitation, opposite of IEEE §6.21), so the label would
    /// silently shadow the block-local meant to shadow IT. Rejecting keeps this
    /// correct-or-loud rather than mis-resolving (the general resolution order is a
    /// documented follow-on).
    #[allow(clippy::type_complexity)] // (params save, param_meta save) — mirrors push_pkg_consts_scoped
    pub(crate) fn push_body_enum_labels(
        &mut self,
        body_enums: &[ast::TypedefDecl],
        body: &ast::Stmt,
    ) -> (
        Vec<(String, Option<i64>)>,
        Vec<(String, Option<(u32, bool)>)>,
    ) {
        let mut saved = Vec::new();
        let mut saved_meta = Vec::new();
        if body_enums.is_empty() {
            return (saved, saved_meta); // common case — no gather, byte-identical
        }
        // Names declared in `begin/end` blocks of this body (nested inner scopes) —
        // a label sharing one of these would mis-shadow it (see doc above).
        let mut block_locals = Vec::new();
        collect_block_local_decls(body, &mut block_locals);
        let block_local_names: std::collections::BTreeSet<&str> = block_locals
            .iter()
            .flat_map(|d| d.names.iter())
            .map(|n| n.name.name.as_str())
            .collect();
        for td in body_enums {
            #[allow(irrefutable_let_patterns)]
            if let ast::TypedefKind::Enum {
                base,
                signed,
                labels,
            } = &td.kind
            {
                // §4.5.158: give body-local labels the enum's declared width+sign in
                // `param_meta` (twin of the module/package paths) so a positive label of
                // a signed enum compares signed inside a function/task; base-less = int(32).
                // ⚠️ DECLARED sign only — see the module-scope twin in `instance.rs` for why
                // the old `|| v < 0` is gone.
                let base_w = self
                    .enum_base_width(base)
                    .or_else(|| base.is_none().then_some(32u32));
                let base_range = self.enum_base_range(base);
                let mut next: i64 = 0;
                for lab in labels {
                    if block_local_names.contains(lab.name.name.as_str()) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "body-local enum label `{}` shares its name with a \
                                 `begin/end` block-local in the same function/task; v1 \
                                 resolves the enclosing enum label OVER the inner \
                                 block-local (opposite of IEEE §6.21 lexical scope) — \
                                 rename one",
                                lab.name.name
                            ),
                        );
                    }
                    let v = match &lab.value {
                        Some(e) => self.const_eval_in_scope(e).unwrap_or_else(|| {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "enum label `{}` value is not a foldable constant",
                                    lab.name.name
                                ),
                            );
                            0
                        }),
                        None => next,
                    };
                    // §6.19 on the value as WRITTEN — see `instance.rs`.
                    self.check_enum_label_fits(
                        base,
                        *signed,
                        &lab.name.name,
                        v,
                        lab.value.is_some(),
                    );
                    // ⭐ Canonical at the declared width — see `instance.rs`.
                    let v = match base_w {
                        Some(w) => Self::const_mask(v, w, *signed),
                        None => v,
                    };
                    let key = self.fq(&lab.name.name);
                    // V33-3: a body-local label is a label too (diagnostic-only map —
                    // see `enum_label_types`; deliberately NOT in the save-list, since
                    // the message that reads it is written long after the unwind).
                    self.enum_label_types
                        .insert(key.clone(), td.name.name.clone());
                    if let Some(w) = base_w {
                        saved_meta.push((
                            key.clone(),
                            self.param_meta.insert(key.clone(), (w, *signed)),
                        ));
                    }
                    let prev = self.bind_param_value(key.clone(), v);
                    self.bind_param_range(&key, base_range);
                    saved.push((key, prev));
                    next = v.wrapping_add(1);
                }
            }
        }
        (saved, saved_meta)
    }

    /// Register a net by name → NetId (declaration-order append). A duplicate
    /// name is a hard error: we keep the FIRST binding, emit `ElabUnsupported`
    /// (closest v1 code; doc-15 reserves `E-ELAB-DUP-DECL` for the eventual
    /// dedicated slot), and do NOT push the orphan net — so `net_count` and the
    /// golden hash are not perturbed by an unreferenceable duplicate.
    /// (LOWERING + COVERAGE verdicts: duplicate-net silent acceptance.)
    /// IEEE 1364-2005 §3.5 — declare `name` as an implicit scalar net, or answer
    /// `false` if the policy forbids it.
    ///
    /// §3.5 says an undeclared identifier becomes a net of the current
    /// `` `default_nettype `` when it appears in ONE OF TWO POSITIONS ONLY: the terminal
    /// list of a gate or module instance, and the LHS of a continuous assignment.
    /// Anywhere else — an ordinary rhs, a procedural lvalue — it stays an error, and
    /// that boundary is iverilog-pinned, not inferred: all three of `assign y = TYPO;`,
    /// `initial TYPO = 1;` and any position under `` `default_nettype none `` are hard
    /// errors there too. So this is deliberately NOT wired into `resolve_net`'s generic
    /// unresolved arm; the two callers that own a §3.5 position ask for it by name.
    ///
    /// vita refused to do this at all until R28 (doc-15 recorded the refusal as
    /// policy: "오타가 조용히 wire가 되는 사고 클래스가 원천 차단"). The refusal is
    /// conservative but it is also non-conforming, and it is unfixable from the user
    /// side when the construct sits in a foundry-supplied cell library or IP model.
    /// The safety it was buying is bought instead by `W2003`, which doc-15 reserved for
    /// exactly this and which `-Werror=W-PARSE-IMPLICIT-NET` restores to a hard error.
    ///
    /// The net is SCALAR (1 bit) — that is what §3.5 mandates, and it is why a wider
    /// continuous assignment to one silently loses its top bits in every simulator.
    /// `check_implicit_net_width` makes that case loud rather than leaving it to the
    /// reader (the reporter found a live 12→1 bit truncation this way).
    pub(crate) fn declare_implicit_net(&mut self, name: &str) -> bool {
        if self.cur_nettype_none {
            return false;
        }
        let fq = self.fq(name);
        let fq2 = fq.clone();
        self.warn_code(
            MsgCode::ParseImplicitNet,
            &format!(
                "implicit net `{fq}` inferred as a 1-bit wire (IEEE 1364-2005 §3.5); \
                 declare it explicitly, or use ``default_nettype none`` to make this \
                 an error"
            ),
        );
        self.implicit_nets.insert(fq2);
        self.add_net(
            name,
            ir::NetVar {
                kind: ir::NetKind::Wire,
                width: 1,
                msb: 0,
                lsb: 0,
                signed: false,
                array_len: 1,
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Wire, 1),
            },
        );
        true
    }

    /// Is `name` unresolved in this scope — i.e. would using it here be an E3010?
    /// Asked with `lookup_net_scoped`, the same resolver `resolve_net` uses.
    pub(crate) fn net_is_undeclared(&self, name: &str) -> bool {
        self.lookup_net_scoped(name).is_none() && self.lookup_scoped(name).is_none()
    }

    pub(crate) fn add_net(&mut self, name: &str, net: ir::NetVar) {
        let key = self.fq(name);
        // A2b-prereq S3: when a local decl shadows a wildcard alias, the alias
        // entry is removed only AFTER the budget check below passes — an early
        // return must leave the maps consistent.
        let mut shadow_wildcard_alias = false;
        if self.symbols.contains_key(&key) {
            // A2b-prereq: a name bound by a package-variable IMPORT is not a
            // declaration. A LOCAL declaration shadows a WILDCARD import
            // (iverilog-pinned: `import p::*` + `int cnt` ⇒ local wins) — drop
            // the alias and fall through to create the real net (the insert
            // below replaces the symbols entry). Colliding with an EXPLICIT
            // import stays loud (iverilog-pinned: "already been imported").
            match self.pkg_var_aliases.get(&key) {
                // Documented leniency (diff F3): IEEE §26.3 makes a local
                // declaration AFTER a use of the wildcard binding an error;
                // vita lowers by pass (nets before process bodies), so textual
                // use-before-decl is not observable here — the local uniformly
                // wins the whole scope (self-consistent, never a torn state).
                Some((_, false)) => {
                    shadow_wildcard_alias = true;
                }
                Some((pkg, true)) => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "`{name}` has already been imported into this scope \
                             from package `{pkg}` (an explicit import conflicts \
                             with a local declaration)"
                        ),
                    );
                    return;
                }
                None => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("net/variable `{key}` redeclared (duplicate declaration)"),
                    );
                    return;
                }
            }
        }
        // GEN-NET-CAP: bound the aggregate net arena. Past the cap, no-op (the
        // arena stops growing) and report once — `had_error` makes the run loud.
        if self.nets.len() >= MAX_TOTAL_NETS {
            if !self.net_budget_blown {
                self.net_budget_blown = true;
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "total net/variable count exceeds the v1 cap ({MAX_TOTAL_NETS}); \
                         the design is too large or a generate loop is pathological"
                    ),
                );
            }
            return;
        }
        if shadow_wildcard_alias {
            self.pkg_var_aliases.remove(&key);
        }
        let id = self.nets.len() as u32;
        self.nets.push(net);
        self.symbols.insert(key, id);
    }

    /// v7 `$bits` prescan (see `bits_prescan`): record one body decl's widths.
    /// Every fold failure is a SILENT skip — the `$bits` call site is the loud
    /// one; the real net lowering later re-folds with full diagnostics.
    pub(crate) fn prescan_net_bits(&mut self, d: &ast::NetVarDecl) {
        let fold_range = |me: &Self, r: &ast::Range| -> Option<u64> {
            match (
                me.const_eval_in_scope(&r.msb),
                me.const_eval_in_scope(&r.lsb),
            ) {
                (Some(m), Some(l)) if m >= 0 && l >= 0 => Some(m.abs_diff(l) + 1),
                _ => None,
            }
        };
        if matches!(d.kind, ast::NetVarKind::String) {
            return; // dynamic length — $bits on a string stays loud
        }
        let elem: u64 = match d.kind {
            // §4.5.155: fixed-width integer atoms (IEEE §6.11.1) carry no range, so
            // the `$bits` prescan must report the KIND width — mirroring `range_to_dims`
            // — not the rangeless `None => 1` default below (which made
            // `$bits(byte_arr[i])` = 1 instead of 8, even though the net storage /
            // `%b` / arithmetic already size the element correctly). Only the static
            // prescan path was stale (the scalar §4.5.154 fix rides `range_to_dims`).
            ast::NetVarKind::Byte => 8,
            ast::NetVarKind::Shortint => 16,
            ast::NetVarKind::Int | ast::NetVarKind::Integer => 32,
            ast::NetVarKind::Longint => 64,
            ast::NetVarKind::Real
            | ast::NetVarKind::Realtime
            | ast::NetVarKind::Time
            | ast::NetVarKind::Event => 64,
            _ => {
                let mut w = match &d.range {
                    None => 1u64,
                    Some(r) => match fold_range(self, r) {
                        Some(w) => w,
                        None => return,
                    },
                };
                for r in &d.packed {
                    match fold_range(self, r) {
                        Some(pw) => w = w.saturating_mul(pw),
                        None => return,
                    }
                }
                w
            }
        };
        'names: for n in &d.names {
            let mut dims: Vec<u64> = Vec::new();
            for dim in &n.unpacked {
                match dim {
                    ast::Dim::Range(r) => match fold_range(self, r) {
                        Some(len) => dims.push(len),
                        None => continue 'names,
                    },
                    ast::Dim::Size(e) => match self.const_eval_in_scope(e) {
                        Some(s) if s > 0 => dims.push(s as u64),
                        _ => continue 'names,
                    },
                    // dyn/queue/assoc — no static bit size
                    _ => continue 'names,
                }
            }
            self.bits_prescan.insert(n.name.name.clone(), (elem, dims));
        }
    }
}

/// The bare single-segment name an lvalue targets, if that is all it is. A select, a
/// concatenation, or a hierarchical path is NOT a §3.5 implicit-net position — §3.5
/// declares a SCALAR, and `assign x[3] = …` on an undeclared `x` is an error in
/// iverilog too (there is no width to select from).
pub(crate) fn bare_lvalue_name(lv: &ast::Lvalue) -> Option<&str> {
    match lv {
        ast::Lvalue::Ident(p) => match p.segments.as_slice() {
            [seg] => Some(seg.name.as_str()),
            _ => None,
        },
        _ => None,
    }
}

/// Collect the bare single-segment identifiers an expression reads. Used only for a
/// GATE-desugared continuous assign, where every terminal is a §3.5 position.
pub(crate) fn collect_bare_idents(e: &ast::Expr, out: &mut Vec<String>) {
    if let ast::ExprKind::Ident(p) = &e.kind {
        if let [seg] = p.segments.as_slice() {
            out.push(seg.name.clone());
            return;
        }
    }
    // Only the shapes `gate_desugar` builds need arms here (Binary for the multi-input
    // reductions, Unary for buf/not, Ternary + IntLit for bufif/notif, Paren for
    // grouping). Being CONSERVATIVE is the safe direction: a missed arm means a name
    // stays undeclared and gets the loud E3010, never a silent net.
    match &e.kind {
        ast::ExprKind::Unary { operand, .. } => collect_bare_idents(operand, out),
        ast::ExprKind::Paren { inner } => collect_bare_idents(inner, out),
        ast::ExprKind::Binary { lhs, rhs, .. } => {
            collect_bare_idents(lhs, out);
            collect_bare_idents(rhs, out);
        }
        ast::ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            collect_bare_idents(cond, out);
            collect_bare_idents(then_e, out);
            collect_bare_idents(else_e, out);
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// IEEE 1364-2005 §3.5 — declare every implicit net this module body implies, in one
    /// pass, before anything is lowered.
    ///
    /// §3.5 names exactly two positions, and this is the only place that decides what
    /// counts as one:
    ///
    /// 1. the terminal list of a GATE or MODULE instance, and
    /// 2. the LHS of a continuous assignment.
    ///
    /// Gate terminals arrive here as a `ContAssign` with `from_gate` set (the parser
    /// desugars `not (Ax, AN);` to `assign Ax = ~AN;`), so for those BOTH sides count —
    /// the read terminals are terminals too. For a user `assign` only the LHS counts:
    /// an undeclared name in an ordinary rhs is an error in iverilog as well, and
    /// admitting it here would restore the typo-becomes-a-wire hazard that the previous
    /// blanket refusal existed to prevent.
    ///
    /// Runs over the module body only. A name that appears solely inside a generate
    /// body stays loud — narrower than the standard, and on the safe side of the ladder.
    pub(crate) fn declare_implicit_nets(&mut self, body: &[ast::ModuleItem]) {
        // V34-6: an INTERFACE INSTANCE name is a declaration, not a §3.5 position.
        // `simple_if bus(); child c(bus);` used to warn `implicit net t.bus inferred as
        // a 1-bit wire` and create a phantom 1-bit wire, because the port-actual arm
        // below sees a bare ident and `net_is_undeclared("bus")` is TRUE — the iface
        // flatten registers symbols for the MEMBERS (`t.bus.d`) and never for the bare
        // instance name. Three things were wrong with that warning: `bus` is declared
        // one line above; its advice is unfollowable (an interface instance cannot be
        // declared as a net, and `default_nettype none` made the warning VANISH rather
        // than become the promised error, since `declare_implicit_net` returns early);
        // and it fired only when the instance was passed as an actual, so N sharing
        // modules meant N fake warnings. verilator (the only oracle here — iverilog 13
        // cannot parse an interface PORT at all, `syntax error` on `module child
        // (simple_if s)`) accepts this design silently and prints d=5a.
        //
        // The set is built from the SAME body this pass already walks, so the pass stays
        // order-free (see instance.rs:658-664 for why that matters). Fixing it by
        // reordering the flatten instead would not work: the flatten never registers a
        // symbol under the bare instance name.
        let iface_insts: BTreeSet<&str> = body
            .iter()
            .filter_map(|it| match it {
                ast::ModuleItem::Instance(mi)
                    if self.ifaces.contains_key(mi.module_name.name.as_str()) =>
                {
                    Some(mi.instances.iter().map(|i| i.name.name.as_str()))
                }
                _ => None,
            })
            .flatten()
            .collect();
        let mut want: Vec<String> = Vec::new();
        for item in body {
            match item {
                ast::ModuleItem::ContAssign(ca) => {
                    for (lv, rhs) in &ca.assigns {
                        if let Some(n) = bare_lvalue_name(lv) {
                            want.push(n.to_string());
                        }
                        if ca.from_gate {
                            collect_bare_idents(rhs, &mut want);
                        }
                    }
                }
                ast::ModuleItem::Instance(mi) => {
                    for inst in &mi.instances {
                        let actuals: Vec<&ast::Expr> = match &inst.conns {
                            // `.name` shorthand is NOT a §3.5 position — IEEE 1800
                            // §23.3.2.2 requires the same-named object to be DECLARED,
                            // and iverilog rejects `.a` with no `a` while accepting the
                            // explicit `.a(a)`. The parser desugars both to the same
                            // `.name(name)` shape, so the flag is the only thing left
                            // that tells them apart.
                            ast::PortConnList::Named(v, _) => v
                                .iter()
                                .filter(|c| !c.implicit_name)
                                .filter_map(|c| c.value.as_ref())
                                .collect(),
                            ast::PortConnList::Positional(v) => v.iter().flatten().collect(),
                        };
                        for a in actuals {
                            if let ast::ExprKind::Ident(p) = &a.kind {
                                if let [seg] = p.segments.as_slice() {
                                    want.push(seg.name.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        for n in want {
            if iface_insts.contains(n.as_str()) {
                continue;
            }
            if self.net_is_undeclared(&n) {
                self.declare_implicit_net(&n);
            }
        }
        self.warn_implicit_net_truncation(body);
    }

    /// The 8-D hazard, made loud: a §3.5 net is SCALAR, so `assign IMPL = <12-bit>;`
    /// keeps bit 0 and discards eleven bits — in every simulator, silently, because the
    /// code is legal. The reporter found a live instance where a port declaration sat
    /// behind an inactive ``ifdef` while its `assign` did not, and the only reason it
    /// was harmless was that nothing read the net.
    ///
    /// This is the one place where matching iverilog's VALUE is not enough. The value
    /// stays truncated (differential wins), and the fact is stated with the widths.
    fn warn_implicit_net_truncation(&mut self, body: &[ast::ModuleItem]) {
        for item in body {
            let ast::ModuleItem::ContAssign(ca) = item else {
                continue;
            };
            if ca.from_gate {
                continue; // a gate output is scalar by construction
            }
            for (lv, rhs) in &ca.assigns {
                let Some(n) = bare_lvalue_name(lv) else {
                    continue;
                };
                if !self.implicit_nets.contains(&self.fq(n)) {
                    continue;
                }
                let Some(w) = self.rhs_decl_width(rhs) else {
                    continue;
                };
                if w > 1 {
                    let fq = self.fq(n);
                    self.warn_code(
                        MsgCode::ElabFeatureLimit,
                        &format!(
                            "`{fq}` is an IMPLICIT net, so it is 1 bit wide (IEEE \
                             1364-2005 §3.5) — this assignment drives it with {w} bits \
                             and the top {} are discarded. Declare it with the width you \
                             meant.",
                            w - 1
                        ),
                    );
                }
            }
        }
    }

    /// The DECLARED width of a bare-identifier rhs, or `None` for anything else. Only
    /// the unambiguous shape is answered — the point is to catch `assign IMPL = bus;`,
    /// not to re-implement context-determined width rules.
    fn rhs_decl_width(&self, e: &ast::Expr) -> Option<u32> {
        let ast::ExprKind::Ident(p) = &e.kind else {
            return None;
        };
        let [seg] = p.segments.as_slice() else {
            return None;
        };
        let id = self.lookup_net_scoped(&seg.name)?;
        self.nets.get(id as usize).map(|n| n.width)
    }
}
