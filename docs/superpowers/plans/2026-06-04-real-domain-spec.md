# Real / Realtime Domain — Implementation-Ready Spec (Deliberate Frozen-IR Evolution)

**Date:** 2026-06-04
**Author:** lead architect (vitamin)
**Status:** implementation-ready. SPEC-authoritative. The highest-care section is the **frozen-IR evolution** (§1): a *deliberate* `format_version 2→3` bump that flips the `sim_ir::SimIr` root hash and invalidates every on-disk `.velab`/`.vu`. That is the intended behavior of the staleness gate, not a regression.

**Front-end status (given):** lexer/parser/hdl-ast ALREADY parse real literals (`ExprKind::RealLit { raw, .. }`) and real/realtime declarations (`ast::NetVarKind::Real`, `ast::NetVarKind::Realtime`). This spec covers **elaborate lowering + sim-ir additions + sim-engine f64 domain + builtins + tests**. It does NOT touch lex/parse.

---

## 0. The one hard constraint that shapes everything

`crates/vita-artifact-derive/src/lib.rs:210` rejects `f32`/`f64`/`usize`/`isize` as a **type head** in any `#[derive(SchemaHash)]` type:

```rust
if matches!(head.as_str(), "usize" | "isize" | "f32" | "f64") {
    return Err(Error::new_spanned(tp, format!(
        "SchemaHash: `{head}` is forbidden in schema types ...")));
}
```

Every IR type (`ConstVal`, `NetVar`, `BitPacked`, `SimIr`, …) derives `SchemaHash`. Therefore **no `f64` field may ever appear in the IR.** A real value crosses the frozen IR as its `f64::to_bits() -> u64` image inside the existing `BitPacked.val: Vec<u64>`. `u64` is whitelisted in `PRIMITIVES`. Two new *fieldless* enum variants add **zero** new field types, so the V-PRIM guard is never triggered. The only place an `f64` ever materializes is inside `sim-engine` arithmetic (via `f64::from_bits`), and `sim-engine` derives **no** `SchemaHash` (zero references) — so the runtime `Value`/`NetSlot` are free to carry an `is_real: bool` flag.

**Encoding contract (canonical, shared everywhere):** a real value `x: f64` is represented as
`BitPacked { val: vec![x.to_bits()], unk: vec![0] }`, with `width = 64`, `signed = true`. `unk` is always all-zero (a real is always 2-state; it can never be X or Z). Uninitialized real = `0.0` = all-zero bits, **not** all-X. This same `u64` image is what `$realtobits`/`$bitstoreal` produce/consume — one shared encode/decode rule.

**Non-finite literals (overflow / ±inf) — documented, deterministic.** Rust's `parse::<f64>()` returns `Ok(f64::INFINITY)` (NOT `Err`) for a literal that overflows the f64 range (e.g. `1e400`), so an overflowing real literal interns as `±inf` with the canonical IEEE bit pattern (`0x7FF0000000000000` / `0xFFF0000000000000`). **This is intentional and accepted:** the bit pattern is fixed and byte-identical across the 3 OSes, so it does NOT threaten the determinism / golden-IR byte-identity guard — an `inf` const is just another fixed `u64` image. The MVP does **not** add an elaborate-time overflow diagnostic; `±inf` flows through arithmetic with IEEE semantics and prints as `inf`/`-inf` under `%f`/`%g`/`%e` (Rust `format!` of an infinity). NaN literals are not producible from source (no NaN literal syntax); NaN arises only from runtime ops like `0.0/0.0` and prints as `NaN`. The §0 "fixed bit pattern" guarantee therefore explicitly includes the non-finite values.

---

## 1. FROZEN-IR EVOLUTION (deliberate — highest care)

### 1.1 The two additions (and the one we override the SPEC for)

**SPEC conflict to flag to the human BEFORE pinning.** `docs/preview/17-sim-ir-ir-backbone-freeze.md` §10 (line ~346) currently records the *opposite* decision:

> `real`/`realtime` 저장 → **sim-ir re-freeze 아님** — no-float 규칙상 별도 비해시 lane(`--dump` RON 뷰)

That standing decision says: do NOT re-freeze sim-ir; carry reals in a separate non-hashed lane. **This spec deliberately overrides it.** Adopting the `to_bits()`-in-`BitPacked` strategy keeps reals *inside* the hashed golden IR while still never adding an f64 field — strictly cleaner than a side-lane (one IR, one golden, reals participate in determinism). **Action item:** when this lands, rewrite doc-17 §10 line ~346 from "sim-ir re-freeze 아님" to a Phase-2 deliberate re-freeze entry, or SPEC and code disagree. This is the only contrary SPEC line; everything else (the freeze contract in `schema_hash.rs:19-21` and doc-17 §10 Phase-2 path) supports this evolution.

### 1.2 The exact IR diff

`crates/sim-ir/src/lib.rs`, NetKind (currently lines 338–345) — **append** one fieldless variant:

```rust
/// Net/variable kind (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum NetKind {
    Wire,
    Reg,
    Logic,
    Integer,
    /// IEEE-754 f64 net (`real`/`realtime`). 64-bit, signed, 2-state. The f64 is
    /// stored as `f64::to_bits()` in `init.val[0]`, `init.unk` all-zero. No f64
    /// field is introduced — the V-PRIM derive guard sees only `u64` inside
    /// `BitPacked`. `realtime` is a synonym and ALSO maps here (no 6th variant).
    Real,
}
```

`crates/sim-ir/src/lib.rs`, ConstRepr (currently lines 377–381) — **append** one fieldless variant:

```rust
/// Constant representation tag (§6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum ConstRepr {
    Numeric,
    StrUtf8,
    /// IEEE-754 f64 literal. `ConstVal.width = 64`, `signed = true`,
    /// `bits.val[0] = literal.to_bits()`, `bits.unk = [0]`. No f64 field.
    Real,
}
```

**Variant order:** append at the end. Variant order is part of the structural shape, so *any* ordering flips the root hash regardless; appending is the convention and keeps diffs minimal. `NetVar`/`ConstVal` **struct field lists are unchanged** — only two enums grow one fieldless arm each.

### 1.3 `BinOp` gains NO variants — confirmed

Real `+ - * /` and `< <= > >= == !=` reuse the **existing** `BinOp` variants. The IR is operand-typed: `Expr::Binary { op, lhs, rhs }` carries no result-type tag; real-vs-integer dispatch lives entirely in `sim-engine` eval on operand kind (exactly as signed-vs-unsigned `Div`/`AShr` already dispatch today). There is no `realadd` token in Verilog. **`BinOp` stays verbatim at its current 24 variants** (`Add, Sub, Mul, Div, Mod, Pow, BitAnd, BitOr, BitXor, BitXnor, LogAnd, LogOr, Lt, Le, Gt, Ge, Eq, Ne, CaseEq, CaseNe, Shl, Shr, AShl, AShr` — verified against `crates/sim-ir/src/lib.rs:112`). Adding `RealAdd` etc. would be redundant and flip the hash further for no reason. Because **no `BinOp` line is touched**, the §1.6 Step E canonical-string golden has **zero** edits on the `BinOp` line; the 24-variant arity is stated here only so the unchanged-ness is auditable.

### 1.4 `SysFuncId` — gains FOUR variants (folded into the SAME bump)

`SysFuncId` (currently `Time, Realtime, Signed, Unsigned, Clog2`) also derives `SchemaHash`, so adding ids ALSO flips the root hash. We need `$rtoi`/`$itor`/`$realtobits`/`$bitstoreal` → **add four ids and batch them into the same `format_version 2→3` bump** (a single deliberate flip, not two). `$realtime` already exists as `SysFuncId::Realtime` (no new id; we only change its *runtime* return to a real — §4). `$time`/`$signed`/`$unsigned`/`$clog2` unchanged.

```rust
/// System-function id (§1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum SysFuncId {
    Time,
    Realtime,
    Signed,
    Unsigned,
    Clog2,
    Rtoi,        // $rtoi  — real → int, TRUNCATE toward zero
    Itor,        // $itor  — int  → real, exact convert
    RealToBits,  // $realtobits — real → 64-bit vector (raw IEEE bits)
    BitsToReal,  // $bitstoreal — 64-bit vector → real (raw IEEE bits)
}
```

So the deliberate flip touches **three** enums: `NetKind` (+1), `ConstRepr` (+1), `SysFuncId` (+4). All in one `format_version` bump.

### 1.5 The exact `format_version` 2→3 bump procedure

`CURRENT_FORMAT_VERSION` is a **single source** — `crates/vita-artifact/src/header.rs:15`:

```rust
pub const CURRENT_FORMAT_VERSION: u32 = 2; // bumped for the M3 SimIr freeze
```

change to:

```rust
pub const CURRENT_FORMAT_VERSION: u32 = 3; // bumped for the real/realtime IR re-freeze (NetKind::Real, ConstRepr::Real, +4 SysFuncId)
```

Every other reference (`cli/src/lib.rs:500,518`, `cli/tests/staged_flow.rs:26`, `vita-artifact/tests/gate.rs:30,49`, `vita-artifact/tests/vu_roundtrip.rs:6`) uses the **constant**, not a literal `2`, so they update transitively. **Nothing else to edit for the version.**

**Effect on existing artifacts (intended).** All previously-written `.velab`/`.vu` files carry `format_version = 2`. After the bump, `verify_header` rejects them at the FORMAT gate (`E-ART-FORMAT-MISMATCH`, with a "regenerate with `velab`" hint) *before* the schema gate is even reached. This is the **correct refuse-and-rebuild** behavior of the staleness machine — old artifacts are stale and must be regenerated. There are **no binary `.velab`/`.vu` golden fixtures** in-tree (`find crates -name '*.velab' -o -name '*.vu'` returns nothing), so there is no binary golden to regenerate. The gate tests build their expected hashes dynamically via `schema_hash::<sim_ir::SimIr>()` (`vita-artifact/tests/gate.rs:62`, `cli/tests/staged_flow.rs:87`) and self-update.

### 1.6 Golden re-pin procedure (unambiguous, step-by-step)

The freeze contract (`crates/sim-ir/tests/schema_hash.rs:19-21`): *"If intentional: all .velab invalid -> bump format_version + update both goldens."* The "both goldens" are (1) the pinned root hash and (2) the canonical-string golden. Two structural golden FILES also grow: `sim_ir_canonical.txt` and `sim_ir_registry.ron`.

**Step A — make the IR edits (§1.2, §1.4).** Build will pass; the *test* `schema_hash_is_pinned` will now FAIL with the new hash printed.

**Step B — capture the NEW hash.** Run:

```bash
cargo test -p sim-ir --locked schema_hash_is_pinned -- --nocapture 2>&1 | grep -A2 'left\|right'
# or print directly:
cargo test -p sim-ir --locked print_simir_hash -- --nocapture --ignored
```

(There is no `print_simir_hash` helper today; the panic message of `schema_hash_is_pinned` already shows `left = <NEW>` / `right = <OLD>`. Copy the `left` value — that is `hex::encode(schema_hash::<sim_ir::SimIr>())` under the new shape.)

**Step C — re-pin `EXPECTED_SIMIR_HASH`** at `crates/sim-ir/tests/schema_hash.rs:6`. Replace
`7b46c1706bc026725c1812db7045df8770136fa5ac85d0e2c8bb44d41071bcd4`
with the captured value.

**Step D — `EXPECTED_PROCESS_HASH` (`schema_hash.rs:9`) MUST NOT CHANGE.** The `Process` cluster transitively contains neither `NetKind` nor `ConstRepr` nor `SysFuncId` reaching it only via `SimIr.nets`/`SimIr.consts`/expr-pool — those are root-level, not Process-level. Verified against `crates/sim-ir/src/lib.rs`: `Process => Sensitivity + Vec<BasicBlock> + SuspendState`; `BasicBlock => Vec<u32> + Terminator`; none of these reach `NetKind`/`ConstRepr`/`SysFuncId`. `SysFuncId` reaches the root via `Expr::SysFunc` in the expression pool, which is also outside the `Process` struct cluster. **If `EXPECTED_PROCESS_HASH` changes, something is wrong** (you accidentally touched a Process-reachable type) — stop and investigate; do not blindly re-pin it.

> **`process_subpin` is necessary-but-not-sufficient.** A green `process_subpin` only proves no *Process-reachable* type moved — it is the intended sanity gate confirming the change was scoped to root-level types (`NetKind`/`ConstRepr`/`SysFuncId` via `nets`/`consts`/`exprs`). It does NOT, by itself, prove the root-hash flip equals the *intended* flip; that is what the canonical-string golden diff (Step E.1 — exactly three enum lines grow, nothing else) demonstrates. Treat the two together: `process_subpin` green + canonical diff limited to the three expected enum lines = the change is correctly scoped.

**Step E — regenerate the two structural golden files:**

1. `crates/testdata/sim_ir_canonical.txt` (consumed by `schema_hash.rs:12` `canonical_string_golden`). Three lines grow:
   - the `ConstRepr` line: `...enum{#[]Numeric,#[]StrUtf8}` → `...enum{#[]Numeric,#[]StrUtf8,#[]Real}`
   - the `NetKind` line: `...enum{#[]Wire,#[]Reg,#[]Logic,#[]Integer}` → `...enum{...,#[]Integer,#[]Real}`
   - the `SysFuncId` line: `...enum{#[]Time,#[]Realtime,#[]Signed,#[]Unsigned,#[]Clog2}` → `...,#[]Clog2,#[]Rtoi,#[]Itor,#[]RealToBits,#[]BitsToReal}`

   Do NOT hand-edit blindly — regenerate. If a `--bless`/regen path exists, use it; otherwise run the canonical-string test with `--nocapture`, copy the emitted canonical string verbatim into the file.

2. `crates/testdata/sim_ir_registry.ron` (Layer-3 RON registry). Add the `Real` arm to the `ConstRepr` ENUM and the `NetKind` ENUM, and the four arms to the `SysFuncId` ENUM. Same regenerate-don't-guess discipline.

**Step F — verify the gate is green:**

```bash
cargo test --workspace --locked
```

The gates that must pass: `schema_hash_is_pinned` (new root hash), `canonical_string_golden` (new canonical file), `process_subpin` (UNCHANGED). `m3_shapes.rs` pins `BinOp`/`Expr`/`Terminator`/`SimIr` local-shape strings but does **NOT** assert `NetKind`/`ConstRepr`/`SysFuncId` local_shape — so those shape tests pass untouched. Only the root-hash pin + canonical/registry goldens need editing.

**This is the intended deliberate-change path.** All old `.velab` become stale; the staleness gate firing on them is correct, not a bug.

---

## 2. ENGINE — the real `Value` domain

### 2.1 Representation: `is_real: bool` flag on `Value` (NOT a `RealVal` enum) — justified

Current struct (`crates/sim-engine/src/value.rs:34-41`):

```rust
pub struct Value {
    pub val: Vec<u64>,
    pub unk: Vec<u64>,
    pub width: u32,
    pub signed: bool,
}
```

**Add one field:**

```rust
pub struct Value {
    pub val: Vec<u64>,
    pub unk: Vec<u64>,
    pub width: u32,
    pub signed: bool,
    /// When true, `val[0]` is `f64::to_bits(x)` and this Value is an IEEE-754
    /// real (64-bit, 2-state, `unk == [0]`). All 4-state paths keep this false.
    pub is_real: bool,
}
```

**Why a flag, not a `RealVal(f64)` enum:** `Value` is constructed at ~40 sites (`zeros`, `xs`, `one1`, `from_packed`, `resize`, every arith/bitwise/select helper) and field-accessed pervasively (`v.val`, `v.width`, `get_vu`, `set_vu`, `has_xz`, `into_bitpacked`). An enum forces every one of those through a `match` — a massive, error-prone churn. A bool field is **additive**: existing constructors set `is_real: false`, existing 4-state paths are byte-for-byte untouched. A real `Value` round-trips through `into_bitpacked(64)`/`from_packed` *for free* (it is just 64 bits with `unk = 0`), so a real net write reaches `NetSlot.cur` and the VCD writer with **no new bridge code**. `PartialEq`/`Eq` still hold (bit-equality of `to_bits()` is the correct `$monitor` change key). Every `Value { .. }` struct-literal construction site must add `is_real: false` (the compiler will enumerate them — see touch list).

### 2.2 Helpers (add next to `to_u64` at `value.rs:158`)

```rust
impl Value {
    /// Build a real Value from an f64. width=64, signed=true, unk=0, is_real.
    pub fn from_f64(x: f64) -> Value {
        Value {
            val: vec![x.to_bits()],
            unk: vec![0],
            width: 64,
            signed: true,
            is_real: true,
        }
    }

    /// Decode to f64. If already real, reinterpret val[0]. Otherwise coerce the
    /// 4-state integer value to f64 (IEEE 1364 §4.3 int→real promotion), honoring
    /// signedness. Returns None only if an integer operand is X/Z (caller decides
    /// poison vs 0.0).
    pub fn to_f64(&self) -> Option<f64> {
        if self.is_real {
            return Some(f64::from_bits(self.val[0]));
        }
        if self.has_xz() {
            return None;
        }
        // reuse existing decoders: signed → to_i128_signed, else to_u64
        if self.signed {
            self.to_i128_signed().map(|i| i as f64)
        } else {
            self.to_u64().map(|u| u as f64)
        }
    }

    /// Build an INTEGER (is_real=false) Value of `width` bits from an i128,
    /// masked to width with two's-complement wrap. This is the constructor the
    /// real→int coercion and `$rtoi` need; it does NOT exist in the codebase
    /// today and MUST be added here (do not reference a phantom API). Pattern
    /// mirrors `$clog2`'s const-build (eval.rs:663): `zeros` + low-word fill +
    /// `mask_top`. Semantics: the low `width` bits of `i`'s two's-complement
    /// image are kept; the upper bits are discarded (the same truncation an
    /// integer lvalue applies). `signed` only stamps the result's sign flag.
    pub fn from_i128(i: i128, width: u32, signed: bool) -> Value {
        let mut v = Value::zeros(width.max(1), signed);
        let bits = i as u128; // reinterpret two's-complement bit image
        let words = ((width as usize) + 63) / 64;
        for w in 0..words.min(v.val.len()) {
            v.val[w] = (bits >> (w * 64)) as u64;
        }
        v.width = width;
        v.mask_top(); // is_real=false so mask_top applies (clears bits above width)
        v
    }
}
```

### 2.3 The width/sign-extend trap — guard three helpers

A real is **always** 64 bits and must **never** be sign-extended or top-masked bit-wise (that corrupts the IEEE-754 pattern). Guard each with an early return:

- `resize` (`value.rs` ~189): `if self.is_real { return self; }` at the top.
- `resize_keep_sign` (`value.rs` ~216): `if self.is_real { return self; }` **as the VERY FIRST line, before the existing `self.signed = self.signed && ctx_signed;` mutation.** `resize_keep_sign` mutates `.signed` *before* delegating to `resize`, so a guard placed only inside `resize` would still let a context flip the real's `.signed` flag (harmless to the bit pattern, but a downstream `if v.signed` branch would then misclassify the real). The early-return must precede every field mutation in this method.
- `mask_top` (`value.rs` ~106): `if self.is_real { return; }` (skip the mask).

This is the single most important correctness guard in the engine: the context-width machinery (`eval_ctx`) must be a no-op for reals because a real is dimensionless.

> **Helper-ordering caveat for `from_i128` (§2.2):** `from_i128` deliberately calls `mask_top()` on an `is_real=false` Value, so the §2.3 `mask_top` guard does NOT fire there (correct — it must mask an integer). The guard keys on `self.is_real`, which `from_i128` never sets. No conflict.

### 2.4 Real arithmetic — `arith` fast-path (`eval.rs:334`)

`arith` is the choke point, dispatched from `eval_binary_ctx` for `Add|Sub|Mul|Div|Mod` (`eval.rs:229-233`) and **`Pow`** (`eval.rs:241-246` — `Pow` ALSO routes into `arith`, verified). Insert the real fast-path as the **very first lines** of `arith`, *before* the `has_xz` poison check:

```rust
fn arith(&self, op: BinOp, l: &Value, r: &Value) -> Value {
    if l.is_real || r.is_real {
        // IEEE 1364 §4.3: if either operand is real, the other is promoted to real.
        let a = l.to_f64().unwrap_or(0.0);
        let b = r.to_f64().unwrap_or(0.0);
        let res = match op {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,   // f64 semantics: x/0 → ±inf, 0/0 → NaN; NOT X
            // Mod (`%`) AND Pow (`**`) on a real are permanent illegalities (§6.2),
            // BOTH rejected at elaborate, so neither should reach here. But this
            // branch is user-reachable IFF the elaborate gate is ever bypassed, so
            // it must NOT panic the simulator. Defensive poison instead of
            // `unreachable!()`: return NaN (a defined, deterministic real poison).
            // `**` IS implementable via powf; we still gate it at elaborate for the
            // MVP, but if a future cut admits `**` on real, replace this arm with
            // `BinOp::Pow => a.powf(b)`.
            BinOp::Mod => f64::NAN,   // unreachable in well-formed elaborate output
            BinOp::Pow => f64::NAN,   // ditto; future: a.powf(b)
            _ => f64::NAN,            // any other op: defensive, never panic
        };
        return Value::from_f64(res);
    }
    // ... existing 4-state integer path, unchanged (has_xz check etc.) ...
}
```

**Note on `Mod`/`Pow`:** `%` AND `**` on a real are **permanent illegalities** caught at elaborate (§6.2 now lists BOTH explicitly). The arith real branch never panics — a defensive `f64::NAN` poison is returned for any op that somehow slips past the elaborate gate, so a user-reachable `real r; r = 2.0 ** 3;` can never crash the simulator even if the gate regresses. The elaborate gate (§6.2) is the primary, provable block; the NaN poison is the belt-and-suspenders backstop. **Never route a user-reachable `**`/`%` on a real to `unreachable!()`.**

### 2.5 Real comparison — `relational` + `log_eq`

`relational` (`eval.rs:409`, handles `< <= > >=`) and `log_eq` (`eval.rs:443`, handles `== != === !==`) are dispatched at `eval.rs:267-277`. Add a real branch at the **top** of each:

```rust
// in relational (eval.rs:409):
if l.is_real || r.is_real {
    let a = l.to_f64().unwrap_or(0.0);
    let b = r.to_f64().unwrap_or(0.0);
    let bit = match (op, a.partial_cmp(&b)) {
        // partial_cmp is None on NaN → all ordered comparisons false (IEEE-754).
        (_, None) => false,
        (BinOp::Lt, Some(o)) => o == std::cmp::Ordering::Less,
        (BinOp::Le, Some(o)) => o != std::cmp::Ordering::Greater,
        (BinOp::Gt, Some(o)) => o == std::cmp::Ordering::Greater,
        (BinOp::Ge, Some(o)) => o != std::cmp::Ordering::Less,
        _ => unreachable!(),
    };
    return Value::logic(bit);   // single 1-bit logic result
}
```

```rust
// in log_eq (eval.rs:443) — handles Eq/Ne ONLY (verified dispatch at eval.rs:276:
// `Eq | Ne => self.log_eq(...)`; CaseEq/CaseNe go to case_eq, see below):
if l.is_real || r.is_real {
    let a = l.to_f64().unwrap_or(0.0);
    let b = r.to_f64().unwrap_or(0.0);
    // VALUE comparison: +0.0 == -0.0 is true (f64 `==` gives this). NaN != NaN.
    let eq = a == b;
    let bit = match op {
        BinOp::Eq => eq,
        BinOp::Ne => !eq,
        _ => unreachable!(),
    };
    return Value::logic(bit);
}
```

**`case_eq` (eval.rs, the `===`/`!==` handler) needs the SAME real branch — `CaseEq`/`CaseNe` are dispatched HERE, not to `log_eq`** (verified at `eval.rs:275`: `CaseEq | CaseNe => self.case_eq(...)`). Without this, a real `===` would fall into the integer case-equality path and compare raw IEEE bits as a 4-state vector. Add at the top of `case_eq`:
```rust
// in case_eq (the === / !== handler):
if l.is_real || r.is_real {
    let a = l.to_f64().unwrap_or(0.0);
    let b = r.to_f64().unwrap_or(0.0);
    // MVP: === on real == value-equal. A real is 2-state (never X/Z), so === has
    // no X/Z bits to distinguish from ==; the two coincide for reals. NaN !== NaN
    // (f64 `==` is false for NaN), matching ==. NOTE: bit-=== would make
    // +0.0 !== -0.0 (different bit patterns); we use VALUE equality (a == b) so
    // +0.0 === -0.0 is TRUE, consistent with == (§ test #13). This is the
    // deliberate MVP choice — documented, not accidental.
    let eq = a == b;
    let bit = match op {
        BinOp::CaseEq => eq,
        BinOp::CaseNe => !eq,
        _ => unreachable!(),
    };
    return Value::logic(bit);
}
```

(`Value::logic(bool)` exists — verified at `value.rs:80`. No fallback needed.)

### 2.6 Unary minus — `negate` (`eval.rs:207`)

The `UnOp::Minus` path (`eval.rs:76`) calls `negate`. Add at its top:

```rust
fn negate(&self, a: &Value) -> Value {
    if a.is_real {
        // `.unwrap_or(0.0)` for consistency with arith/relational (NOT `.unwrap()`):
        // on a real, to_f64 always returns Some, so the default never fires — but
        // using the same unwrap policy everywhere removes the latent panic surface
        // a future edit could expose if the `is_real` guard ever moves.
        return Value::from_f64(-a.to_f64().unwrap_or(0.0));
    }
    // ... existing 4-state negate ...
}
```

Unary plus on a real is identity (no change needed — it passes the value through). **Edge: `-(0.0)` yields `-0.0`.** Under the §4.1 formatters this prints `0` via `%g` (the `format_g` zero-guard canonicalizes `±0.0`→`"0"`) but `-0.000000` via `%f` (`{:.6}` preserves the sign). Test #14b (§5) pins both so the signed-zero display is intentional, not accidental. (Verified on rustc 1.82.)

### 2.7 Real-literal eval (from `ConstRepr::Real`)

When eval reads a `ConstVal { repr: Real, .. }` from the const pool, it must produce a real `Value`. The two `matches!(c.repr, ConstRepr::Numeric) && c.signed` sites currently mis-handle a real const (they yield `signed=false` and route bits through integer `from_packed`):

- `eval.rs:161` — the const-read path. Add a real special-case **before** the `matches!`:

```rust
// at eval.rs ~161, where a ConstVal is materialized into a Value:
if matches!(c.repr, ConstRepr::Real) {
    // val[0] already holds f64::to_bits; reinterpret as real.
    return Value::from_f64(f64::from_bits(c.bits.val[0]));
}
let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
// ... existing from_packed integer path ...
```

- `width.rs:96` — self-width of a const. A real const is `{width:64, signed:true}`. Add:

```rust
if matches!(c.repr, ConstRepr::Real) {
    return SelfWidth { width: 64, signed: true };
}
let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
```

### 2.8 Every new NetKind / ConstRepr match arm in engine (exhaustive)

**Compiler-FORCED (exhaustive `match`, will not compile without the arm):**

| file:line | site | new arm |
|---|---|---|
| `sim-engine/src/state.rs:322` | `vcd_var_type(kind: NetKind) -> VarType` | `NetKind::Real => VarType::Real` |

`vcd_var_type` is currently:
```rust
pub(crate) fn vcd_var_type(kind: NetKind) -> VarType {
    match kind {
        NetKind::Reg => VarType::Reg,
        NetKind::Integer => VarType::Integer,
        NetKind::Wire | NetKind::Logic => VarType::Wire,
    }
}
```
becomes:
```rust
pub(crate) fn vcd_var_type(kind: NetKind) -> VarType {
    match kind {
        NetKind::Reg => VarType::Reg,
        NetKind::Integer => VarType::Integer,
        NetKind::Real => VarType::Real,           // VCD `$var real`
        NetKind::Wire | NetKind::Logic => VarType::Wire,
    }
}
```
`VarType::Real` must exist in the vcd-writer's `VarType` enum. If it does not, add it and have the writer emit `$var real 64 <id> <name> $end` (and dump real values as `r<decimal> <id>` per the VCD real format). MVP fallback if the strict real VCD var is invasive: map to `VarType::Reg` and dump the 64 bits — bits are correct and lossless either way; the strict `$var real` is a refinement.

**`matches!` semantic sites (compile fine; must special-case for correctness):** `eval.rs:161`, `width.rs:96` — both covered in §2.7.

**`NetSlot` read flag (`state.rs`):** add `pub is_real: bool` to `NetSlot` (set in `SimState::new`, `state.rs:111`, from `nv.kind == NetKind::Real`). In `read_net` (`state.rs:311-315`):

```rust
fn read_net(&self, net: u32, word: Option<u32>) -> Value {
    let slot = &self.nets[net as usize];
    let packed = self.net_word_packed(net, word);
    let mut v = Value::from_packed(&packed, slot.width, slot.signed);
    v.is_real = slot.is_real;     // <-- flag the read-back as real
    v
}
```

The **write path needs no change**: `write_chunk` (`state.rs:215`) consumes `get_vu`/`set_bit` bit-by-bit; a real `Value`'s 64 `val` bits are its IEEE pattern and store verbatim into `cur: BitPacked`.

---

## 3. ELABORATE — lowering

### 3.1 Real literal: `ExprKind::RealLit` → `ConstVal { repr: Real }`

Currently (`elaborate/src/lib.rs:1504-1505`):
```rust
ast::ExprKind::RealLit { .. } => {
    self.error(MsgCode::ElabUnsupported, "real literal not supported (v2)");
    // ... poison / placeholder ...
}
```
Replace with parse-and-intern:
```rust
ast::ExprKind::RealLit { raw, .. } => {
    let x = parse_real_literal(raw);   // strips '_' separators, parse::<f64>(), round-to-nearest-even
    let cv = ir::ConstVal {
        width: 64,
        signed: true,
        repr: ir::ConstRepr::Real,
        bits: ir::BitPacked { val: vec![x.to_bits()], unk: vec![0] },
    };
    let cidx = self.intern_const(cv);
    // emit Expr::Const(cidx) (same pathway integer literals use)
    self.push_expr(ir::Expr::Const(cidx))
}
```
`parse_real_literal` (companion to the existing `parse_int_literal` in `elaborate/src/literal.rs:140`):
```rust
/// IEEE 1364 real literal → f64. Underscores stripped. round-to-nearest-even
/// is Rust's f64 parse default. Caller guarantees `raw` is a well-formed real
/// literal (front-end already validated the grammar).
///
/// OVERFLOW: an out-of-range literal (e.g. `1e400`) parses to `Ok(±inf)`, NOT
/// `Err`, so `unwrap_or(0.0)` does NOT fire and the literal interns as `±inf`
/// with the canonical IEEE bit pattern. This is intentional and deterministic
/// (§0 "Non-finite literals"). The `unwrap_or(0.0)` only covers a truly
/// unparseable string, which the validated grammar should never deliver.
pub(crate) fn parse_real_literal(raw: &str) -> f64 {
    let cleaned: String = raw.chars().filter(|&c| c != '_').collect();
    cleaned.parse::<f64>().unwrap_or(0.0) // overflow → ±inf (Ok), see §0
}
```

### 3.2 `intern_const` dedup key — compiler-FORCED arm (`elaborate/src/lib.rs:1976`)

Currently:
```rust
match cv.repr {
    ir::ConstRepr::Numeric => 0,
    ir::ConstRepr::StrUtf8 => 1,
}
```
becomes (exhaustive — will NOT compile without it):
```rust
match cv.repr {
    ir::ConstRepr::Numeric => 0,
    ir::ConstRepr::StrUtf8 => 1,
    ir::ConstRepr::Real => 2,
}
```
(The dedup key must distinguish a real const whose `val[0]` happens to equal an integer const's bits — the `repr` tag in the key does exactly that.)

### 3.3 `NetVarKind::Real/Realtime` → `NetVar { kind: Real, width: 64 }`

**`map_net_kind_or_wire` (`elaborate/src/lib.rs:3471`)** — currently `_ => ir::NetKind::Wire` silently collapses real to wire (WRONG). Add explicit arms (behaviorally required, not compiler-forced):
```rust
fn map_net_kind_or_wire(k: ast::NetVarKind) -> ir::NetKind {
    match k {
        ast::NetVarKind::Reg => ir::NetKind::Reg,
        ast::NetVarKind::Logic => ir::NetKind::Logic,
        ast::NetVarKind::Integer => ir::NetKind::Integer,
        ast::NetVarKind::Real | ast::NetVarKind::Realtime => ir::NetKind::Real,
        _ => ir::NetKind::Wire,
    }
}
```

**`net_kind_supported` (`elaborate/src/lib.rs:3485`)** — currently lists `Wire|Tri|Uwire|Reg|Logic|Integer`. Add `Real|Realtime` so they stop hitting the `ElabUnsupported` at `lib.rs:1162` ("unsupported net/var kind (v1)"):
```rust
fn net_kind_supported(k: ast::NetVarKind) -> bool {
    matches!(k,
        ast::NetVarKind::Wire | ast::NetVarKind::Tri | ast::NetVarKind::Uwire
        | ast::NetVarKind::Reg | ast::NetVarKind::Logic | ast::NetVarKind::Integer
        | ast::NetVarKind::Real | ast::NetVarKind::Realtime)
}
```

**`range_to_dims` (`elaborate/src/lib.rs:1217`)** — mirror the `Integer => (32,31,0,true)` special-case. For `Real|Realtime` return `(width:64, msb:63, lsb:0, signed:true)`:
```rust
// inside range_to_dims, alongside the Integer special-case:
ast::NetVarKind::Real | ast::NetVarKind::Realtime => (64, 63, 0, true),
```
(A real declaration carries no `[msb:lsb]` range; it is dimensionless. Force 64/63/0/signed.)

**`default_init` (`elaborate/src/lib.rs:3492`)** — currently treats `Real|Realtime|Time` as variables (lines ~3499-3501) but initializes them all-X. A real MUST initialize to `0.0` (all-zero bits, `unk=0`), NOT X. Add a real-specific branch BEFORE the all-X variable default:
```rust
fn default_init(kind: ast::NetVarKind, width: u32) -> ir::BitPacked {
    if matches!(kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime) {
        // real default = +0.0 = all-zero bits, never X.
        return ir::BitPacked { val: vec![0], unk: vec![0] };
    }
    // ... existing all-X-for-variables / all-0-for-wires logic ...
}
```

### 3.4 Real assignment coercion (real↔int at the lvalue boundary) — DONE IN THE ENGINE WRITE PATH, NOT "implied"

**This is the single most load-bearing real-semantics rule and the earlier "no opcode, implied by kind" wording was NOT implementable.** Verified flow: an assignment evaluates the RHS via `eval_for_lvalue` (`sched.rs:584`), which only knows `lvalue_width` — it has **no** access to the LHS `NetKind`. The result then goes to `write_lvalue` (`state.rs:160`), which does `value.resize(total)` and writes bits verbatim through `write_chunk` into `NetSlot.cur`. `NetSlot` carries `width`/`signed` but historically **no kind**. So a real RHS `Value` (64 IEEE bits) would land bit-for-bit in an integer net — `integer n; real r; r = 2.5; n = r;` would store `0x4004000000000000` into `n` and `%0d` would print garbage, **not** `3`. There was no site performing the round. **Fixed by carrying the kind on `NetSlot` and coercing inside `write_lvalue`** (§2.8 already adds `NetSlot.is_real`; this section specifies the coercion that consumes it).

**The mechanism (option b — concrete, no new IR node).** `write_lvalue(lhs, value)` gains a **whole-net kind-aware coercion prologue**, executed BEFORE the `value.resize(total)` line. It compares the destination net's `is_real` against `value.is_real`:

```rust
pub fn write_lvalue(&mut self, lhs: &Lvalue, value: Value) -> bool {
    // ── real↔int assignment coercion (IEEE 1364 §6.2) ──
    // Only a WHOLE-NET lvalue (single Bit chunk, no offset/width) can be a real
    // destination: a real is dimensionless and never bit/part-selected (§6.2
    // makes r[i]/r[hi:lo] illegal at elaborate, so a real net never appears as a
    // sub-select chunk). Detect the whole-net case and consult NetSlot.is_real.
    let dest_is_real = lhs.chunks.len() == 1
        && matches!(lhs.chunks[0].kind, SelKind::Bit)
        && lhs.chunks[0].offset.is_none()
        && lhs.chunks[0].width.is_none()
        && self.nets[lhs.chunks[0].net as usize].is_real;

    let value = match (dest_is_real, value.is_real) {
        // real net ← real value: store verbatim (already 64 IEEE bits).
        (true, true) => value,
        // real net ← integer value (int→real CONVERT): exact for ≤53-bit.
        (true, false) => Value::from_f64(value.to_f64().unwrap_or(0.0)),
        // integer net ← real value (real→int ASSIGNMENT: ROUND half-away):
        // f64::round() is round-half-away-from-zero == IEEE 1364 §6.2 assignment
        // rounding. Then from_i128 masks to the destination width below via the
        // normal resize/write path (we hand it the net's full width here).
        (false, true) => {
            let w = self.nets[lhs.chunks[0].net as usize].width; // safe: a real RHS
            // only assigns to a whole scalar int net in the MVP (concat-LHS of a
            // real is illegal §6.2). If a multi-chunk int LHS ever receives a real,
            // round first then let write_chunk slice the integer image.
            let signed = self.nets[lhs.chunks[0].net as usize].signed;
            real_to_int_round(value.to_f64().unwrap_or(0.0), w.max(1), signed)
        }
        // integer net ← integer value: unchanged legacy path.
        (false, false) => value,
    };

    let total: u32 = lhs.chunks.iter().map(|c| self.chunk_width(c)).sum();
    let src = value.resize(total.max(1)); // for a real dest, §2.3 resize-guard no-ops
    // ... existing chunk-slice / write_chunk loop, UNCHANGED ...
}
```

The engine helper (added next to `from_i128` in §2.2):

```rust
/// real → int assignment coercion: ROUND half-away-from-zero, then build an
/// integer Value masked to the target width. Saturation/NaN handling is pinned
/// in §8 invariant 13.
fn real_to_int_round(x: f64, width: u32, signed: bool) -> Value {
    let r = x.round();                  // Rust f64::round = round-half-away-from-zero
    let i = r as i128;                  // large |x| SATURATES to i128 extremes (MVP);
                                        // NaN.round() as i128 == 0 (so %d of NaN→"0")
    Value::from_i128(i, width, signed)  // masks to target width (see §2.2)
}
```

**Why whole-net only is sufficient:** §6.2 makes bit/part-select and concatenation of a real **permanently illegal at elaborate**, so a real value can only ever be assigned to (or received from) a *whole scalar* net. The `dest_is_real` predicate and the `(false, true)` int-net branch therefore cover every legal real-bearing assignment. A multi-chunk LHS can never be a real destination, and a real RHS into a (illegal) concat-LHS is already rejected upstream.

**`$rtoi` vs assignment:** `$rtoi` TRUNCATES toward zero; a plain real→int assignment ROUNDS half-away. These are deliberately different (§7, Example C, and tests #3 vs #4). Do not conflate — `real_to_int_round` uses `round()`, `$rtoi` (§4.3) uses `trunc()`.

**No IR opcode is introduced.** The coercion lives entirely in the engine `write_lvalue` choke point, keyed on `(NetSlot.is_real, Value.is_real)` — both runtime, non-schema. The frozen IR is untouched by this rule (consistent with the operand-typed philosophy: dispatch on runtime kind, not on a tagged node).

### 3.5 Real delay `#1.5` → rounded integer ticks

The tick wheel is integer-only: `Terminator::Delay { amount: u32, .. }` and `ContAssign.delay: Option<u32>`. The rounding happens at **elaborate**, not runtime. `lower_delay` (`elaborate/src/lib.rs:3108-3129`) folds via `const_eval_u32` (`lib.rs:3301`), which returns `None` for a `RealLit` (its `_ => None` arm) — so today `#1.5` degrades to `#0`. Fix: detect `RealLit` before the fallback, in BOTH delay-fold sites (`lower_delay` and the cont-assign fold at `lib.rs:1321-1330`):

```rust
// before falling back in lower_delay (and cont-assign fold):
if let ast::ExprKind::RealLit { raw, .. } = &expr.kind {
    let x = parse_real_literal(raw);
    // IEEE 1364 §9: real delay = number of time UNITS; multiply by
    // (unit/precision) ratio to get precision-ticks, then round half-away.
    // MVP without per-module timescale ratio: ratio = 1, round directly.
    let ticks = (x.round() as i64).clamp(0, u32::MAX as i64) as u32;
    return ticks;   // Terminator::Delay.amount (u32); runtime wheel unchanged
}
```
(The full timescale-ratio scaling — §9/§10 — multiplies by unit/precision before rounding; the rounding *site* is exactly here. MVP can ship ratio=1 and layer scaling later without moving this site.) **The runtime scheduler needs zero change**: it only ever sees integer ticks.

---

## 4. BUILTINS — formatting + system functions

### 4.1 `%f` / `%g` / `%e` (and `%d` on a real rounds)

`format_args_str` (`builtins.rs:252`), the `match spec` block (`builtins.rs:286`). Add arms alongside `'d'`/`'h'`:

```rust
// in match spec (builtins.rs:286):
'f' | 'F' | 'g' | 'G' | 'e' | 'E' => {
    let v = next_arg(sched, args, &mut argi);
    out.push_str(&fmt_real(&v, spec));   // honors width/precision modifiers parsed alongside
}
```

**`%b/%h/%o`/`%x` on a real is rejected at ELABORATE, not here.** `format_args_str` has **no diagnostic sink** (verified: `builtins.rs` imports no `MsgCode`/`diag` — only `io::Write`, `SysTaskId`, vcd-writer, `Scheduler`, `SimState`, `Value`). A runtime "diagnose %h-of-real" is therefore impossible at this site, and the previous `'b' | 'o' if /* arg is_real */` pseudo-guard does not compile and has nowhere to report. **The illegality is made a STATIC elaborate-time check** (§4.1a below). Because elaborate guarantees no real argument is ever paired with a `%b/%h/%o/%x` specifier, the existing `'h'|'x' => fmt_radix(...)` / `'b' => fmt_radix(...)` / `'o' => fmt_radix(...)` arms can **never** receive a real `Value` — so they need no guard and the silent-garbage fallthrough is provably impossible. **Do NOT add a runtime `is_real` branch inside `fmt_radix`** (it has no error channel); the elaborate gate is the sole, sufficient block.

New `fmt_real` next to `fmt_dec` (`builtins.rs:338`):
```rust
fn fmt_real(v: &Value, spec: char) -> String {
    let x = v.to_f64().unwrap_or(0.0);
    match spec {
        'f' | 'F' => format!("{:.6}", x),  // default 6 fractional digits (C %f)
        'e' | 'E' => fmt_real_e(x),        // LRM/printf form: 1.500000e+03
        'g' | 'G' => format_g(x),          // shortest of fixed/exp, strip trailing zeros
        _ => format!("{}", x),
    }
}

/// %e → C/printf/LRM form: 6 mantissa fraction digits, signed exponent, exponent
/// zero-padded to AT LEAST 2 digits. Rust's `{:.6e}` gives `1.500000e3` (no sign,
/// no zero-pad), so we post-process the exponent. Non-finite passes through as
/// Rust prints it (`inf`/`-inf`/`NaN`).
fn fmt_real_e(x: f64) -> String {
    if !x.is_finite() {
        return format!("{}", x); // inf / -inf / NaN
    }
    let s = format!("{:.6e}", x);            // e.g. "1.500000e3" or "1.234500e-5"
    // split mantissa and exponent at the 'e'
    let (mant, exp) = s.split_once('e').expect("rust {:e} always emits 'e'");
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(d) => ('-', d),
        None => ('+', exp),
    };
    // zero-pad exponent magnitude to >= 2 digits
    let padded = if digits.len() < 2 {
        format!("{:0>2}", digits)
    } else {
        digits.to_string()
    };
    format!("{mant}e{sign}{padded}")        // "1.500000e+03"
}

/// %g: shortest of %e/%f with trailing zeros stripped, per C/LRM. Rust's default
/// `{}` Display does NOT switch to exponent form for large/small magnitudes
/// (`1e20` → "100000000000000000000", `1e-10` → "0.0000000001", `-0.0` → "-0"),
/// so it is NOT %g-conformant on its own. We implement the LRM algorithm: pick
/// %e or %f by exponent, then strip insignificant trailing zeros.
fn format_g(x: f64) -> String {
    if !x.is_finite() {
        return format!("{}", x); // inf / -inf / NaN
    }
    // C %g with default precision P=6: let exp be the base-10 exponent of x.
    // If exp < -4 or exp >= P, use %e (precision P-1); else use %f (precision P-1-exp).
    // Then strip trailing zeros (and a trailing '.').
    if x == 0.0 {
        return "0".to_string(); // both +0.0 and -0.0 → "0" under %g zero-strip
    }
    let exp = x.abs().log10().floor() as i32;
    let p: i32 = 6; // default %g precision
    let body = if exp < -4 || exp >= p {
        // exponent form, P-1 mantissa fraction digits, then LRM exponent normalize
        let raw = format!("{:.*e}", (p - 1) as usize, x); // e.g. "1.50000e3"
        let (mant, e) = raw.split_once('e').unwrap();
        let mant = strip_trailing_zeros(mant);
        let (sgn, dig) = match e.strip_prefix('-') { Some(d) => ('-', d), None => ('+', e) };
        let dig = if dig.len() < 2 { format!("{:0>2}", dig) } else { dig.to_string() };
        return format!("{mant}e{sgn}{dig}");
    } else {
        let prec = (p - 1 - exp).max(0) as usize;
        format!("{:.*}", prec, x) // fixed form
    };
    strip_trailing_zeros(&body)
}

/// Strip insignificant trailing zeros after a decimal point, and a bare trailing '.'.
/// "1500.000" → "1500", "0.000100" → "0.0001", "3.0" → "3".
fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let t = s.trim_end_matches('0');
    t.trim_end_matches('.').to_string()
}
```

**Conformance note:** `format_g` now matches C/LRM `%g` (shorter of `%e`/`%f`, trailing-zero strip) for the realistic magnitudes the prior `format!("{}", x)` got wrong — `1e20`→`1e+20`, `1e21`→`1e+21`, `1e-10`→`1e-10`, `1500.0`→`1500`, `0.0001`→`0.0001`, `0.00001`→`1e-05`. We do **not** assert "Rust Display matches %g" anywhere; the algorithm is the LRM one. `-0.0` prints `0` under `%g` (zero-strip), and `inf`/`-inf`/`NaN` pass through verbatim.

**`%d` on a real ROUNDS** (§8). `fmt_dec` (`builtins.rs:338`) must check `is_real`:
```rust
fn fmt_dec(v: &Value) -> String {
    if v.is_real {
        let x = v.to_f64().unwrap_or(0.0);
        // round half-away (Rust round()). Large |x| SATURATES to i64::MAX/MIN
        // (e.g. 1e30 → 9223372036854775807); NaN.round() as i64 == 0 (NaN → "0").
        // Pinned as intentional MVP behavior in §8 invariant 13.
        return format!("{}", x.round() as i64);
    }
    // ... existing integer signed/unsigned path ...
}
```

Width/precision modifiers (`%8.2f`, `%0.3g`) — parse the existing modifier prefix the format engine already extracts for integer specifiers and thread `width`/`precision` into the `format!` calls (`format!("{:>width$.prec$}", x)`). At minimum support width+precision on `%f`/`%e`/`%g` (MVP requirement §8).

### 4.1a `%b/%h/%o/%x` on a real — STATIC elaborate-time rejection (where the diagnostic actually lives)

The format string is a compile-time constant (a `ConstVal` `StrUtf8`, reachable at elaborate). Elaborate can therefore pair each conversion specifier against the static kind of its argument BEFORE the IR is frozen, which is the only place with a `MsgCode` sink. Add a check in the `$display`/`$write`/`$monitor`/`$strobe`/`$fwrite` lowering (wherever the `SysTask { fmt, args }` is built in `elaborate/src/lib.rs`):

```rust
// when lowering a formatted system task: scan the literal format string,
// match each %-specifier to its positional arg's elaborate-time NetKind/expr kind.
// If a real-typed arg is paired with a radix specifier, reject:
//   %b/%B, %h/%H/%x/%X, %o/%O on a real arg  →
//     self.error(MsgCode::ElabUnsupported,
//                "binary/hex/octal format (%b/%h/%o) not defined on a real argument (use %f/%g/%e, or $realtobits for the raw bits)");
```

This makes the §6.2 "%b/%h/%o on real = illegal" rule a **provable static gate**, so the runtime `fmt_radix` arms can never see a real (no runtime guard needed, no runtime diag channel needed). `%f/%g/%e/%d` on a real are all legal (real formatting + `%d`-rounds). `%b/%h/%o/%x` on the raw 64-bit vector is available via `$realtobits(r)` then `%h` on the resulting integer vector.

### 4.2 `$realtime` returns a REAL (`eval.rs:643`)

Currently `Time | Realtime` share one integer arm:
```rust
SysFuncId::Time | SysFuncId::Realtime => {
    let mut v = Value::zeros(64, false);
    v.val[0] = self.now;
    v
}
```
**Split `Realtime` out** to a real:
```rust
SysFuncId::Time => {
    let mut v = Value::zeros(64, false);
    v.val[0] = self.now;       // $time: integer ticks, rounded to time-unit
    v
}
SysFuncId::Realtime => {
    // $realtime: current time as a REAL in time-units (fractional preserved).
    // MVP without per-module unit ratio: now-as-f64. Scale by precision/unit later.
    Value::from_f64(self.now as f64)
}
```
**`eval_sysfunc_ctx` (`eval.rs:676`)** routes `Realtime` through the `_ =>` arm at `eval.rs:697` which calls `resize_keep_sign(w, eff_signed)` — with the §2.3 real-no-op guard, the f64 bits survive untouched. Verify `Realtime` hits the real-safe path (its result `is_real`, so `resize_keep_sign` early-returns).

### 4.3 `$rtoi` / `$itor` / `$realtobits` / `$bitstoreal`

**`map_sysfunc` (`elaborate/src/lib.rs:3389`)** — add four names:
```rust
"$rtoi"       => Some(ir::SysFuncId::Rtoi),
"$itor"       => Some(ir::SysFuncId::Itor),
"$realtobits" => Some(ir::SysFuncId::RealToBits),
"$bitstoreal" => Some(ir::SysFuncId::BitsToReal),
```

**`eval_sysfunc` (`eval.rs:641`)** — add arms. NOTE: adding any `SysFuncId` variant makes this `match which` (no `_` wildcard, verified `eval.rs:642`) and the `Expr::SysFunc` self-width `match which` (`width.rs:234`, also no `_`) **fail to compile** until all four arms exist at BOTH sites — they are compiler-FORCED, exactly like `vcd_var_type`/`intern_const` (see §8 invariant 11). Add all four arms here AND in `width.rs` in the same atomic change:
```rust
SysFuncId::Rtoi => {
    // real → int, TRUNCATE toward zero. Result is a plain integer Value.
    // from_i128 is the §2.2 helper (masks to 32 bits, signed). NOT a phantom API.
    let x = self.eval(args[0]).to_f64().unwrap_or(0.0);
    Value::from_i128(x.trunc() as i128, 32, true)   // integer-typed (is_real=false)
}
SysFuncId::Itor => {
    // int → real, exact convert.
    let i = self.eval(args[0]).to_i128_signed().unwrap_or(0);
    Value::from_f64(i as f64)
}
SysFuncId::RealToBits => {
    // real → 64-bit vector (raw IEEE bits). val[0] ALREADY holds to_bits();
    // just clear is_real so it reads as a plain 64-bit integer vector.
    let mut v = self.eval(args[0]);
    v.is_real = false;
    v.signed = false;
    v.width = 64;
    v
}
SysFuncId::BitsToReal => {
    // 64-bit vector → real. Same bits, set is_real.
    // X/Z GUARD (§6.2 "cannot convert X/Z to real"): if any source bit is X/Z,
    // the masked-to-zero `val` would decode to a DEFINITE wrong real with the X/Z
    // information destroyed. We cannot raise an elaborate diagnostic from the
    // engine, so produce a DEFINED, deterministic poison (NaN) rather than a
    // fabricated clean real. (The companion STATIC elaborate gate — §6.2 / §4.1a
    // style — should reject a statically-X/Z arg where detectable; this runtime
    // backstop covers values that only become X/Z at runtime.)
    let src = self.eval(args[0]);
    if src.has_xz() {
        return Value::from_f64(f64::NAN);   // poison, never a silent wrong number
    }
    let mut v = src;
    v.is_real = true;
    v.signed = true;
    v.width = 64;
    v
}
```
`$realtobits`/`$bitstoreal` are nearly free: the `is_real` flag *is* the only thing distinguishing the two interpretations of identical 64 bits. `$bitstoreal($realtobits(x)) === x` for all non-NaN, non-X/Z x (and bit-preserving for a clean NaN bit-image). **Mixed-op X-integer policy (documented, not silent):** `to_f64()` on an X/Z integer operand returns `None`, and the arith/relational fast-paths use `.unwrap_or(0.0)` — i.e. an X/Z integer participating in a *mixed real op* is deliberately coerced to `0.0` (a defined, deterministic caller choice, NOT a panic and NOT an X-propagation). This is the documented MVP policy: real arithmetic is 2-state, so an X integer entering a real op decays to `0.0` rather than poisoning the whole real result. `$bitstoreal` is the one place this is upgraded to a NaN poison because there the X/Z is the *entire* payload, not an incidental operand. **Width of `$rtoi` result, self-width in `width.rs:234`:** add arms so `Rtoi`→`{32,signed:true}` (integer), `Itor`/`BitsToReal`→`{64,signed:true}` (real-domain, but `SelfWidth` carries no `is_real`; the real-ness is established at eval time), `RealToBits`→`{64,signed:false}`. `Realtime`'s self-width stays `{64,false}` as today (the §2.3 resize guard protects the bits regardless of the `SelfWidth.signed` the ctx computes — see §2.8).

### 4.4 System-function state table

| sysfunc | SysFuncId | input | output Value | semantics |
|---|---|---|---|---|
| `$time` | `Time` (exists) | — | int 64, unsigned | now, rounded to time-unit |
| `$realtime` | `Realtime` (exists, retargeted) | — | **real 64** | now as real, fractional kept |
| `$rtoi(r)` | `Rtoi` (new) | real | int 32 signed | **truncate** toward zero |
| `$itor(i)` | `Itor` (new) | int | real 64 | exact convert |
| `$realtobits(r)` | `RealToBits` (new) | real | int 64 unsigned | raw IEEE bits (clear is_real) |
| `$bitstoreal(b)` | `BitsToReal` (new) | int 64 | real 64 | raw IEEE bits (set is_real) |

---

## 5. TESTS (18 functions, exact assertions)

All engine end-to-end tests follow the `end_to_end.rs` `build()` + `simulate_capture()` shape (build SV source → elaborate → simulate → capture `$display`/`$write` stdout). Assertions match captured output exactly. Add to `crates/cli/tests/` (or the existing `end_to_end.rs` integration module). Helper assumed: `fn run_sv(src: &str) -> String` returning concatenated display output.

```rust
// crates/cli/tests/real_domain.rs   (new integration test file)

use super::run_sv; // or the crate's existing test harness import

// 1. real division is real (Example A): 1.0/3.0 prints 0.333333 via %f
#[test]
fn real_division_is_real() {
    let out = run_sv(r#"
module t; real r; initial begin r = 1.0 / 3.0; $display("%f", r); end endmodule
"#);
    assert_eq!(out.trim(), "0.333333");
}

// 2. int+real promotion (Example B): i/2.0 promotes → 3.5 ; i/2 stays integer 3.0
#[test]
fn int_real_promotion() {
    let out = run_sv(r#"
module t; integer i; real r;
initial begin i = 7; r = i / 2.0; $display("%g", r); r = i / 2; $display("%g", r); end
endmodule
"#);
    assert_eq!(out.trim(), "3.5\n3"); // i/2.0 → 3.5 (real div) ; i/2 → 3 (int div, then to real)
}

// 3. real→int assignment ROUNDS half-away (Example C)
#[test]
fn real_to_int_assignment_rounds_half_away() {
    let out = run_sv(r#"
module t; real r; integer n;
initial begin r = 2.5; n = r; $display("%0d", n); r = -2.5; n = r; $display("%0d", n); end
endmodule
"#);
    assert_eq!(out.trim(), "3\n-3"); // 2.5→3, -2.5→-3 (away from zero)
}

// 4. $rtoi TRUNCATES toward zero (contrast with #3)
#[test]
fn rtoi_truncates_toward_zero() {
    let out = run_sv(r#"
module t; real r; integer n;
initial begin r = 2.9; n = $rtoi(r); $display("%0d", n); r = -2.9; n = $rtoi(r); $display("%0d", n); end
endmodule
"#);
    assert_eq!(out.trim(), "2\n-2"); // truncate, NOT round
}

// 5. $itor exact int→real
#[test]
fn itor_converts() {
    let out = run_sv(r#"
module t; real r; initial begin r = $itor(7); $display("%g", r); end endmodule
"#);
    assert_eq!(out.trim(), "7");
}

// 6. $realtobits / $bitstoreal round-trip is identity
#[test]
fn realtobits_bitstoreal_roundtrip() {
    let out = run_sv(r#"
module t; real r; reg [63:0] b; real r2;
initial begin r = 3.14159; b = $realtobits(r); r2 = $bitstoreal(b); $display("%g", r2); end
endmodule
"#);
    assert_eq!(out.trim(), "3.14159");
}

// 7. $realtime returns a real with fractional time (Example D, simplified ratio=1)
#[test]
fn realtime_returns_real() {
    // With MVP ratio=1, advancing 1 tick then reading $realtime as %g.
    let out = run_sv(r#"
module t; initial begin #1 $display("%g", $realtime); end endmodule
"#);
    assert_eq!(out.trim(), "1"); // now=1 as real → "1" via %g
}

// 8. %g shortest formatting (Example E)
#[test]
fn percent_g_shortest() {
    let out = run_sv(r#"
module t; real r;
initial begin
  r = 1500.0;   $display("%g", r);
  r = 0.0001;   $display("%g", r);
  r = 0.00001;  $display("%g", r);
end endmodule
"#);
    // C/LRM %g: exp(0.00001) = -5 < -4 → exponent form "1e-05".
    assert_eq!(out.trim(), "1500\n0.0001\n1e-05"); // %f for exp in [-4,P) ; %e otherwise
}

// 9. %f vs %e fixed shapes — %e is LRM/printf form: 6 mantissa digits, signed
//    2-digit exponent (1.500000e+03), produced by fmt_real_e (§4.1), NOT raw
//    Rust {:e} (which would give 1.5e3). This is the EXACT emitted string.
#[test]
fn percent_f_and_e() {
    let out = run_sv(r#"
module t; real r; initial begin r = 1500.0; $display("%f|%e", r, r); end endmodule
"#);
    assert_eq!(out.trim(), "1500.000000|1.500000e+03"); // %f six digits ; %e printf-form exponent
}

// 10. %d on a real ROUNDS half-away
#[test]
fn percent_d_on_real_rounds() {
    let out = run_sv(r#"
module t; real r; initial begin r = 2.7; $display("%0d", r); end endmodule
"#);
    assert_eq!(out.trim(), "3");
}

// 11. real delay #1.5 rounds to integer ticks; $time after is the integer now
#[test]
fn real_delay_rounds_to_ticks() {
    // #1.5 (ratio=1) rounds half-away → 2 ticks. $time after = 2.
    let out = run_sv(r#"
module t; initial begin #1.5 $display("%0d", $time); end endmodule
"#);
    assert_eq!(out.trim(), "2");
}

// 12. NetKind::Real net round-trips through write/read (a real reg holds its value)
#[test]
fn real_net_write_read_roundtrip() {
    let out = run_sv(r#"
module t; real r; initial begin r = 6.022; $display("%g", r); end endmodule
"#);
    assert_eq!(out.trim(), "6.022");
}

// 13. real comparison: value compare, +0.0 == -0.0
#[test]
fn real_compare_value_semantics() {
    let out = run_sv(r#"
module t; real a, b;
initial begin a = 0.0; b = -0.0; $display("%0d", (a == b)); a = 1.5; b = 2.5; $display("%0d", (a < b)); end
endmodule
"#);
    assert_eq!(out.trim(), "1\n1"); // +0==-0 true ; 1.5<2.5 true
}

// 14. unary minus on real
#[test]
fn real_unary_minus() {
    let out = run_sv(r#"
module t; real r; initial begin r = -(2.5); $display("%g", r); end endmodule
"#);
    assert_eq!(out.trim(), "-2.5");
}

// 14b. signed-zero display is intentional and pinned. Under the §4.1 format_g
//      (which has an `if x == 0.0 { return "0" }` guard that also catches -0.0),
//      %g of -(0.0) is "0"; but %f uses `format!("{:.6}", x)` which PRESERVES the
//      sign → "-0.000000". Verified on rustc 1.82. (If a future format_g drops
//      the zero guard, %g would become "-0" via Rust Display — re-bless then.)
#[test]
fn real_negative_zero_display() {
    let out = run_sv(r#"
module t; real r; initial begin r = -(0.0); $display("%g|%f", r, r); end endmodule
"#);
    assert_eq!(out.trim(), "0|-0.000000"); // %g canonicalizes -0.0→"0"; %f keeps sign
}

// 16. %d of a NaN real prints "0"; %d of a huge real saturates (i64::MAX, then
//     %d signed-decimal of that). Pins the §8 invariant-13 saturation/NaN policy.
#[test]
fn percent_d_real_nan_and_huge() {
    let out = run_sv(r#"
module t; real r;
initial begin
  r = 0.0/0.0;        $display("%0d", r);   // NaN.round() as i64 == 0
  r = 1.0e30;         $display("%0d", r);   // saturates to i64::MAX
end endmodule
"#);
    assert_eq!(out.trim(), "0\n9223372036854775807");
}

// 17. real division by zero is ±inf (NOT X), printed as "inf"/"-inf" — pins the
//     §2.4 f64 div semantics and §4.1 non-finite passthrough.
#[test]
fn real_div_zero_is_inf() {
    let out = run_sv(r#"
module t; real r; initial begin r = 1.0/0.0; $display("%g", r); r = -1.0/0.0; $display("%g", r); end endmodule
"#);
    assert_eq!(out.trim(), "inf\n-inf");
}
```

**cli smoke (a real testbench computes + prints a real):**

```rust
// 15. cli smoke — pipe a .sv testbench through `vita` (oneshot) and check stdout
#[test]
fn cli_smoke_real_testbench() {
    // writes a temp .sv, invokes the vita oneshot binary, asserts the printed real.
    let src = r#"
module tb; real acc; integer k;
initial begin
  acc = 0.0;
  for (k = 1; k <= 4; k = k + 1) acc = acc + (1.0 / k);  // harmonic sum H4
  $display("H4=%f", acc);
end
endmodule
"#;
    let out = run_vita_oneshot(src); // harness: write temp file, run `vita <file>`, capture stdout
    // 1 + 0.5 + 0.333333... + 0.25 = 2.083333...
    assert_eq!(out.trim(), "H4=2.083333");
}
```

**Schema-hash golden re-pin test (updated to NEW value):**

```rust
// crates/sim-ir/tests/schema_hash.rs — AFTER §1.6 Step C re-pin.
// The root hash is the NEW value captured from Step B. process_subpin UNCHANGED.
const EXPECTED_SIMIR_HASH: &str = "<NEW_HASH_CAPTURED_IN_STEP_B>"; // was 7b46c170...1071bcd4
const EXPECTED_PROCESS_HASH: &str =
    "927e19344413644037635cfcebc50c76c08a413356b9463b5819f7979f1f486b"; // UNCHANGED — sanity gate

#[test]
fn schema_hash_is_pinned() {
    assert_eq!(
        hex::encode(schema_hash::<sim_ir::SimIr>()),
        EXPECTED_SIMIR_HASH,
        "SCHEMA_HASH changed — a frozen sim-ir shape/serde-attr moved.\n\
         If intentional: all .velab invalid -> bump format_version + update both goldens."
    );
}
#[test]
fn process_subpin() {
    assert_eq!(hex::encode(schema_hash::<sim_ir::Process>()), EXPECTED_PROCESS_HASH);
}
```

**Note on exact `%e`/`%g` strings (now pinned, not "adjust later"):** the §4.1 `fmt_real_e` produces the LRM/printf form `1.500000e+03` (6 mantissa digits, signed exponent zero-padded to ≥2 digits) — NOT raw Rust `{:e}` (`1.5e3`) and NOT `{:.6e}` (`1.500000e3`). Test #9 asserts exactly `1.500000e+03`. The §4.1 `format_g` implements the C/LRM `%g` algorithm (shorter of `%e`/`%f`, trailing-zero strip), so test #8's three values are deterministic: `1500.0`→`1500`, `0.0001`→`0.0001`, `0.00001`→`1e-05`. **All real-format test strings in §5 are blessed against the §4.1 algorithms as written — they are not placeholders.** If an implementer changes a §4.1 formatter, they must re-bless the matching §5 assertion byte-for-byte; the formatter and the test are co-specified here.

> **Caution — test #8 `0.00001`:** under the C/LRM `%g` algorithm, `0.00001` has base-10 exponent `-5`, which is `< -4`, so it renders in exponent form `1e-05` (NOT the fixed `0.00001` the prior `format!("{}", x)` would have produced). Test #8's assertion is therefore `1500\n0.0001\n1e-05`. Update the test #8 expected string accordingly (the spec's earlier `0.00001` literal was the non-conformant Rust-Display value).

---

## 6. DIAGNOSTICS / bijection + DEFERRALS

### 6.1 MsgCode bijection — no NEW code expected

Real errors REUSE the existing `MsgCode::ElabUnsupported` (the doc-15 ↔ doc bijection gate stays balanced — **no new MsgCode minted, no bijection edit**). The permanent illegalities (§6.2 below) and the MVP deferrals all surface as `ElabUnsupported` with a *distinct message string*. If a future cut wants to split "permanently illegal" from "not-yet-supported" into two codes, that is a separate doc-15 bijection change — out of scope here.

### 6.2 Permanent illegalities (MUST diagnose with `ElabUnsupported`, NOT silent)

Emit `self.error(MsgCode::ElabUnsupported, "<msg>")` at elaborate when any operand is real:
- `%` (modulo) on real — `"modulo (%) not defined on real operand"`
- **`**` (power) on real** — `"power (**) not defined on real operand in MVP"` — **REQUIRED:** `Pow` dispatches into `arith` (`eval.rs:241`), so without this gate a real `**` reaches the arith real branch. The arith branch now returns a defensive NaN poison instead of panicking (§2.4), but the *primary* block is this elaborate gate; both must exist. (A future cut may admit `**` via `powf` — see §2.4.)
- bitwise `& | ^ ~^ ~`, reductions, shifts `<< >> <<< >>>` on real — `"bitwise/shift/reduction not defined on real operand"`
- bit-select `r[i]`, part-select `r[hi:lo]` on a real — `"bit/part-select not defined on real operand"`
- concatenation/replication containing a real `{..., r, ...}` — `"real may not appear in concatenation (use $realtobits)"`
- `posedge`/`negedge` on a real (or real expr) — `"real has no edge (posedge/negedge illegal)"`
- `%b`/`%h`/`%o`/`%x` format specifier paired with a real argument — `"binary/hex/octal format not defined on a real argument"` (static format-string check, §4.1a)
- converting an X/Z vector to real (`$bitstoreal` of a statically-X/Z arg) — `"cannot convert X/Z to real"`; runtime-X/Z values are backstopped by the `$bitstoreal` NaN poison (§4.3)

Logical `&& || !` and ternary `?:` ARE legal on reals (real is true iff non-zero; ±0.0 false). These do NOT diagnose.

### 6.3 DEFERRED (MVP boundary — `ElabUnsupported` with a "not yet supported in this MVP" message, distinct from "illegal")

- **`shortreal`** (SV 32-bit single) — only 64-bit `real`/`realtime`.
- **Real arrays / real memories** — only scalar real variables.
- **Real-typed PORTS across module boundaries** — supported cross-module path is the `$realtobits`/`$bitstoreal` 64-bit-vector bridge. Internal-to-one-module reals are fully supported.
- **`$sformatf`/`$sformat` of a real** — real formatting only via display/write family in MVP.
- **Real in `case`/`casex`/`casez`** selector/items — use `==` in an `if` instead.
- **Level-sensitive `@(real)` value-change triggers** — keeps the event domain integral.
- **Real `$random`/`$dist_*` and the transcendental math library** (`$sqrt`/`$pow`/`$exp`/`$ln`/`$sin`/…) — only the four arithmetic ops + the §4 conversion/time sysfuncs.

Everything in §2–§4 (decl, literals, `+ - * /`, comparisons, int↔real coercion, `$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`/`$realtime`, `%f`/`%g`/`%e`/`%d`, real delays) is IN the MVP.

---

## 7. TOUCH LIST (every file) + ORDER (build never sits broken across the IR bump)

**Order rationale:** do the IR + version + golden re-pin as ONE atomic step so the schema-hash gate is green before any downstream code is added. Then add engine arms (compiler-forced first), then elaborate, then builtins, then tests. The build is green after each numbered group.

**Group 1 — IR + frozen-IR bump + golden re-pin (ATOMIC, do together):**
1. `crates/sim-ir/src/lib.rs` — append `NetKind::Real`; append `ConstRepr::Real`; append `SysFuncId::{Rtoi,Itor,RealToBits,BitsToReal}`. (§1.2, §1.4)
2. `crates/vita-artifact/src/header.rs:15` — `CURRENT_FORMAT_VERSION 2 → 3`. (§1.5)
3. Run `cargo test -p sim-ir schema_hash_is_pinned -- --nocapture`; capture NEW root hash. (§1.6 Step B)
4. `crates/sim-ir/tests/schema_hash.rs:6` — re-pin `EXPECTED_SIMIR_HASH`. Leave `EXPECTED_PROCESS_HASH` UNCHANGED. (§1.6 Step C/D)
5. `crates/testdata/sim_ir_canonical.txt` — regenerate (ConstRepr/NetKind/SysFuncId lines grow). (§1.6 Step E.1)
6. `crates/testdata/sim_ir_registry.ron` — add ConstRepr/NetKind/SysFuncId arms. (§1.6 Step E.2)
   → `cargo test --workspace --locked` GREEN (schema gates pass, code still ignores the new variants).

**Group 2 — engine arms (compiler-forced + semantic):**
7. `crates/sim-engine/src/value.rs:34` — `Value { ..., is_real: bool }`; add `from_f64`/`to_f64`/**`from_i128`**(§2.2, masking constructor — NOT phantom) + **`real_to_int_round`**(§3.4 helper); guard `mask_top`(~106)/`resize`(~189)/**`resize_keep_sign`(~216, early-return as the FIRST line before `.signed` mutation)** with `if is_real`. Add `is_real: false` to EVERY `Value { .. }` literal (compiler enumerates them). (§2.1–2.3)
8. `crates/sim-engine/src/state.rs:322` — `vcd_var_type`: `NetKind::Real => VarType::Real` (compiler-forced). Add `is_real` to `NetSlot` (struct ~18), set in `SimState::new` (~115, from `nv.kind == NetKind::Real`), flag in `read_net` (~315). **Add the §3.4 real↔int coercion prologue to `write_lvalue`(~160)** keyed on `(dest_is_real, value.is_real)`. (§2.8, §3.4)
9. vcd-writer `VarType` — add `Real` (emit `$var real`), or MVP-fallback map to `Reg`/dump bits. (§2.8)
10. `crates/sim-engine/src/eval.rs` — `arith`(334) real fast-path (**`Mod`/`Pow`/`_` → `f64::NAN` poison, NOT `unreachable!()`**); `relational`(409, `Lt/Le/Gt/Ge`); `log_eq`(443, `Eq/Ne` ONLY); **`case_eq` (`CaseEq/CaseNe` — separate dispatch, same real branch)**; `negate`(207, `.unwrap_or(0.0)`); const-read real special-case (161). (§2.4–2.7)
11. `crates/sim-engine/src/width.rs:96` — `ConstRepr::Real => {64,true}` self-width. (§2.7)

**Group 3 — elaborate lowering:**
12. `crates/elaborate/src/lib.rs:1504` — replace `RealLit` `ElabUnsupported` with intern-as-`ConstVal{repr:Real}`. (§3.1)
13. `crates/elaborate/src/literal.rs` — add `parse_real_literal`. (§3.1)
14. `crates/elaborate/src/lib.rs:1976` — `intern_const` dedup key `ConstRepr::Real => 2` (compiler-forced). (§3.2)
15. `crates/elaborate/src/lib.rs:3471` — `map_net_kind_or_wire`: `Real|Realtime => NetKind::Real`. (§3.3)
16. `crates/elaborate/src/lib.rs:3485` — `net_kind_supported`: accept `Real|Realtime`. (§3.3)
17. `crates/elaborate/src/lib.rs:1217` — `range_to_dims`: `Real|Realtime => (64,63,0,true)`. (§3.3)
18. `crates/elaborate/src/lib.rs:3492` — `default_init`: `Real|Realtime => BitPacked{val:[0],unk:[0]}` (0.0 not X). (§3.3)
19. **(coercion is in the ENGINE, not elaborate — see item 8.)** Elaborate needs no per-assignment conversion node; the real↔int round/convert is performed in `write_lvalue` keyed on runtime kind. Elaborate only ensures lhs/rhs net kinds are correctly set (items 15–18) so `NetSlot.is_real` is right. (§3.4)
20. `crates/elaborate/src/lib.rs:3108` (`lower_delay`) + `:1321` (cont-assign fold) — `RealLit` → rounded ticks. (§3.5)
21. `crates/elaborate/src/lib.rs:3389` — `map_sysfunc`: `$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`. (§4.3)
22. `crates/elaborate/src/lib.rs` — emit `ElabUnsupported` diagnostics for §6.2 illegalities (incl. **`**`/Pow on real** and the **static `%b/%h/%o/%x`-on-real format-string check, §4.1a**) + §6.3 deferrals.

**Group 4 — builtins:**
23. `crates/sim-engine/src/builtins.rs:286` — `match spec` `%f/%g/%e` arms. **NO runtime `%b/%h/%o`-on-real guard** (no diag sink here; the static elaborate gate §4.1a/item 22 makes a real reaching `fmt_radix` impossible). (§4.1)
24. `crates/sim-engine/src/builtins.rs:338` — `fmt_dec` `is_real` → round (saturate/NaN per §8.13); add `fmt_real`/`fmt_real_e`/`format_g`/`strip_trailing_zeros`. (§4.1)
25. `crates/sim-engine/src/eval.rs:643` — split `Realtime` → `Value::from_f64(now as f64)`. (§4.2)
26. `crates/sim-engine/src/eval.rs:642` — `Rtoi`/`Itor`/`RealToBits`(X/Z→NaN poison)/`BitsToReal` arms. **COMPILER-FORCED** (exhaustive `match which`). (§4.3)
27. `crates/sim-engine/src/width.rs:234` — `Expr::SysFunc` self-width arms for the four new sysfuncs. **COMPILER-FORCED** (exhaustive `match which`, same forcing as item 26 — both must land together). (§4.3)

**Group 5 — tests:**
28. `crates/cli/tests/real_domain.rs` (new) — tests **1–17** (§5: 1–14, 14b, 15, 16, 17 = **18 functions**). Strings are blessed against the §4.1 algorithms as written (test #8 → `1e-05`, test #9 → `1.500000e+03`). (§5)
29. `crates/elaborate/src/tests.rs` — add `Real` net-kind lowering cases (cover §3.3) + a `%h`-on-real and `**`-on-real `ElabUnsupported` rejection case (§4.1a, §6.2).
30. `crates/sim-engine/src/width_tests.rs` — add real self-width cases (const `{64,true}`; the four sysfunc self-widths).

→ `cargo test --workspace --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all -- --check` GREEN.

**Group 6 — SPEC reconciliation (non-code, REQUIRED before merge):**
31. `docs/preview/17-sim-ir-ir-backbone-freeze.md` §10 (~line 346) — rewrite "sim-ir re-freeze 아님" → a Phase-2 deliberate re-freeze entry recording the v2→v3 flip (NetKind::Real + ConstRepr::Real + 4 SysFuncId). **Flag to human before pinning** (§1.1).

---

## 8. Invariant checklist (one-liners)

1. No `f64` field in any `SchemaHash` type — reals cross the IR as `to_bits() u64` in `BitPacked.val[0]`, `width=64`, `unk=[0]`.
2. `is_real` lives ONLY on the non-schema runtime `Value`/`NetSlot`; never in IR.
3. `BinOp` UNCHANGED; `NetKind` +1, `ConstRepr` +1, `SysFuncId` +4 — ONE deliberate `format_version 2→3` flip.
4. Re-pin `EXPECTED_SIMIR_HASH` (new); `EXPECTED_PROCESS_HASH` UNCHANGED (sanity gate).
5. All old `.velab`/`.vu` go stale → FORMAT gate rejects them — intended, not a regression.
6. `realtime` ⇒ `NetKind::Real`; `$realtime` already exists as `SysFuncId::Realtime` (retargeted to real, no new id).
7. real→int ASSIGNMENT rounds half-away; `$rtoi` TRUNCATES — kept distinct. The round/convert happens in `write_lvalue` keyed on `(NetSlot.is_real, Value.is_real)` (§3.4) — NOT an IR opcode, NOT "implied".
8. mixed int/real op promotes int→real (operand-driven); real makes the enclosing arith real. An X/Z integer entering a mixed real op decays to `0.0` (documented `.unwrap_or(0.0)` policy, §4.3), never panics, never X-propagates.
9. real delays round half-away to integer ticks at ELABORATE; scheduler sees only integer time.
10. `%`/`**`/bitwise/shift/select/concat/edge on real = permanent `ElabUnsupported`; `%b/%h/%o/%x`-on-real = static format-string `ElabUnsupported` (§4.1a); deferrals = distinct `ElabUnsupported` message. The arith real branch returns a defensive `f64::NAN` poison for `%`/`**` if the gate is ever bypassed — NEVER `unreachable!()`.
11. **compiler-FORCED arms** (exhaustive `match` with no `_`, will NOT compile without the arm): `vcd_var_type` (state.rs:322, on `NetKind`), `intern_const` (elaborate:1976, on `ConstRepr`), `eval_sysfunc` (eval.rs:642, on `SysFuncId`), and the `Expr::SysFunc` self-width match (width.rs:234, on `SysFuncId`). Adding `NetKind::Real`/`ConstRepr::Real`/the four `SysFuncId` variants breaks each of these at compile time, so ALL must gain their arms in the SAME atomic change (the two `SysFuncId` matches BOTH need all four `Rtoi/Itor/RealToBits/BitsToReal` arms). An implementer who follows §4.2's Time/Realtime split but forgets §4.3's four-arm additions at eval.rs:642 AND width.rs:234 will NOT compile — that is the intended forcing function. Everything else (`map_net_kind_or_wire`, `range_to_dims`, etc.) is additive/semantic, not compiler-forced.
12. `$realtobits`/`$bitstoreal` = same bits, only `is_real` flips — round-trip identity for non-NaN, non-X/Z. `$bitstoreal` of an X/Z vector returns a `f64::NAN` poison (§4.3), never a fabricated clean real.
13. **Real→int saturation/NaN policy (pinned):** `%d`/`real_to_int_round`/`$rtoi` convert via `round()`/`trunc()` then `as i64`/`as i128`, which SATURATES large magnitudes to the int extreme (`1e30` → `i64::MAX` = `9223372036854775807`) and maps `NaN` to `0` (`NaN.round() as i64 == 0`, so `%d` of a NaN real prints `"0"`). Overflow real *literals* intern as `±inf` (§0); `inf`/`-inf`/`NaN` print as `inf`/`-inf`/`NaN` under `%f`/`%g`/`%e`. All deterministic and byte-identical across the 3 OSes. Pinned by tests #16/#17.
