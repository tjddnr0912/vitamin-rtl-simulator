# 16 · Schema hash

`#[derive(SchemaHash)]` collapses the whole structural shape of a serialized type into a single
32-byte blake3 value. This document is the contract for that value: what it covers and what it
deliberately does not, the canonical string it is computed from, why the canonical key embeds
`module_path!()` and what that costs, what the derive accepts and rejects, how the hash reaches an
artifact header, and the determinism properties the mechanism rests on. The container that carries
the hash is [14-staged-artifacts.md](14-staged-artifacts.md); the frozen types it hashes are
[17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md).

---

## 1. The guarantee

One proposition:

> If any type reachable from a hash root changes shape — a field added, removed, reordered or
> retyped; an enum variant added, removed or reordered; an explicit discriminant changed; a
> wire-affecting serde attribute added or changed — the root hash flips. Every artifact on disk
> stamped with the earlier hash then refuses to load with a clean, actionable error instead of
> mis-parsing in silence.

There are two roots, one per artifact kind. Both are stamped into the same header field.

| Artifact | Root type | Stamped by | Gate context |
|---|---|---|---|
| `.vu` | `hdl_ast::SourceUnit` | `vcmp` | `ToolContext::new(schema_hash::<hdl_ast::SourceUnit>())` |
| `.velab` | `sim_ir::SimIr` | `velab` | `ToolContext::current()`, which is `ToolContext::new(schema_hash::<sim_ir::SimIr>())` |

### 1.1 In scope

| Covered | Detail |
|---|---|
| Shape | Field names, field types, variant names, variant payload shapes, explicit discriminants, fixed-array lengths |
| Order | Field order within a type and variant order within an enum, both as written in the source |
| serde attributes | The fourteen keys of §7, which change the wire bytes without changing the Rust shape |

### 1.2 Out of scope, deliberately

| Not covered | Why, and what covers it instead |
|---|---|
| The value graph | The runtime object graph is cyclic; the hash is a function of shape only, so cycles in values are irrelevant to it |
| Diagnostic types | `sim-ir` does not depend on `diag`, so no `MsgCode`, `Severity`, `Diagnostic`, `LogEvent`, span or source-location type is reachable from either root. Locations ride out of band ([13-diagnostics-and-logging.md](13-diagnostics-and-logging.md)) |
| Everything out of band | Trailer segments, engine-facing sidecars and `sim_ir::COp`/`sim_ir::CBinOp` sit outside both closures. Their wire shape is gated by `format_version` instead ([14-staged-artifacts.md](14-staged-artifacts.md) §7) |
| The internals of a `with=` module | A `#[serde(with = "m")]` module whose encoding changes without its path changing is invisible to the derive. Layer 3 of §10 covers that class |

Layer 1 is therefore a complete oracle for Rust shape and for the serde attribute surface, and not
a complete oracle for wire bytes. The boundary is stated here so it is not mistaken for coverage.

---

## 2. Crate topology

Three crates, two of them leaves.

```
sim-ir, hdl-ast       ──►  vita-artifact-derive     proc-macro; deps syn, quote, proc-macro2
sim-ir, hdl-ast       ──►  vita-schema              SchemaShape, ShapeRegistry, schema_hash
vita-artifact         ──►  sim-ir, vita-schema, diag, serde, postcard
vita-schema           ──►  blake3                   leaf
vita-artifact-derive  ──►  syn, quote, proc-macro2  leaf
```

Two constraints fix that arrangement:

- The trait cannot live in `vita-artifact`. `vita-artifact` depends on `sim-ir`, so `sim-ir`
  implementing a trait defined in `vita-artifact` would close a cargo dependency cycle and fail to
  compile.
- The trait cannot live in the derive crate either. A crate with `proc-macro = true` exports macros
  and nothing else.

Hence a separate runtime leaf crate whose only dependency is blake3. A proc-macro runs inside
rustc, so the whole mechanism obeys the no-build-script rule of
[03-build-and-portability.md](03-build-and-portability.md); `vita`'s own crates carry no
`build.rs`.

`vita-artifact` stays language-neutral: it never names `hdl-ast`, and the expected root hash is
passed in by the caller.

---

## 3. The trait and the registry

```rust
pub trait SchemaShape {
    /// Rename-STABLE canonical key, `concat!(module_path!(), "::", Ident)`.
    fn schema_name() -> &'static str;
    /// LOCAL shape body only (this type's fields/variants + own serde attrs).
    /// Children appear as their reference strings, never inlined.
    fn local_shape() -> &'static str;
    /// Register self then recurse into each distinct child (DFS, insert_once-guarded).
    fn register(reg: &mut ShapeRegistry);
}

pub struct ShapeRegistry {
    entries: BTreeMap<&'static str, &'static str>, // schema_name -> local_shape
    visited: BTreeSet<&'static str>,
}
```

A proc-macro sees one type's syntax tree at a time. It reads a field's type *path tokens*, never
the shape those tokens point at, so a pure-syntax derive cannot inline a child's body. Composition
therefore has to happen at runtime, and the chosen model is a sorted registry: each type emits its
own body as a compile-time `&'static str` and a `register` method that recurses into each distinct
child; the closure is interned into one shared, name-sorted registry.

Three properties follow, and each is load-bearing:

| Property | Consequence |
|---|---|
| A child is referenced by name, never inlined | A change inside a child rewrites that child's own registry line, which changes the canonical string, which flips the root hash. This is the mechanism by which a nested change propagates |
| The registry deduplicates by name | A type reached from several parents is interned once. `insert_once` returns `false` on a repeat and short-circuits the walk, which also terminates the recursive `hdl-ast` closure |
| A repeat under a different shape is a collision | `insert_once` asserts with a plain `assert_eq!`, which survives a release build: `SchemaHash name collision: {name} registered with two different shapes`. Two distinct types aliasing one key would otherwise hash as one |

The generated `register` body is the same for every type:

```rust
fn register(reg: &mut vita_schema::ShapeRegistry) {
    if !reg.insert_once(Self::schema_name(), Self::local_shape()) {
        return;                       // already interned: dedup + recursion guard
    }
    <sim_ir::ProcFlags as vita_schema::SchemaShape>::register(reg);
    // ... one call per distinct child user type, in source order ...
}
```

Child collection happens at macro-expansion time: the derive peels `Option`, `Vec`, `BTreeMap`,
`BTreeSet`, `Box`, arrays, tuples and references down to leaf paths, drops the primitives, and
deduplicates the remainder with a plain `Vec` and a linear scan — a hash-ordered set is banned even
inside the macro.

A flat, materialized string is hashed once rather than a leaf-up Merkle tree over child hashes. A
Merkle tree would also be stable and would dedup naturally, but it discards the reviewable
artifact: a developer would see two opaque digests rather than the one changed line.

---

## 4. The canonical key, and why a frozen type may not move

`schema_name()` expands to `concat!(module_path!(), "::", Ident)`. The derive emits
`module_path!()` **as a token**, so the compiler expands it in the crate and module where the
`#[derive]` is written — the type's own defining module — not inside the macro body.

That single decision has four consequences.

| Consequence | Detail |
|---|---|
| The key is fully qualified | `sim_ir::Frame` and a `Frame` in any other crate cannot collide on a bare key |
| The key is the Rust identifier, never a serde rename | The registry key has to be rename-stable, or adding `#[serde(rename)]` would silently re-key and reorder the registry and double-count as a reordering. The wire effect of a rename is captured exactly once, in the attribute slot of §7 |
| The macro must not evaluate `module_path!()` itself | Calling it inside the macro body would stamp the derive crate's own path onto every type: a wrong-but-stable key set, with real collisions between same-named types, and no compile error to reveal it |
| A `SchemaHash` type may not move between modules | Moving one changes `module_path!()`, changes the key, changes the canonical string and flips the root hash. Every `.vu` and `.velab` on disk becomes stale |

The last row is the reason the frozen `sim-ir` types and every `hdl-ast` type live at their crate
root. It applies to a pure refactor: relocating a frozen type into a submodule is an artifact-format
change with no other visible symptom.

`sim-ir` declares `extern crate self as sim_ir;`, so cross-type fields are spelled `sim_ir::Foo` and
the body reference string is byte-equal to the child's registry key. `crates/sim-ir/tests/body_refs.rs`
enforces that: every `sim_ir::X` token appearing in a body must be a registry key, and no bare
user-type identifier may appear in type position. `hdl-ast` spells cross-type fields bare, so its
body references and its keys differ in spelling; the hash only needs to be a stable function of
shape, so that is sound, and the dangling-reference guard is specific to `sim-ir`, where the
equality is the thing being checked.

---

## 5. The canonical shape grammar

Two layers: a **type expression** (how a field's type is rendered) and a **container definition**
(the body baked into `local_shape()`).

### 5.1 Lexical rules

- UTF-8, ASCII in practice. **No insignificant whitespace is ever emitted** — every byte is
  meaningful, which removes the `\n`-versus-`\r\n` drift class outright.
- Single-ASCII separators only: `< > ( ) { } [ ] , ; : = @ # |`, with `::` as the path separator.
- Separator is `,`; there is never a trailing one. A zero-field struct is `struct{}`.
- An array length and an explicit discriminant are rendered as their token text with all
  whitespace removed, so `[u8; 4]` renders `[u8;4]` and `= 1 + 2` renders `=1+2`.
- No JSON or RON intermediate ever enters the hash input. RON appears only in the Layer-3 golden.

### 5.2 Type expressions

```ebnf
texpr    ::= prim | option | seq | set | map | array | tuple | fqpath

prim     ::= "u8" | "u16" | "u32" | "u64" | "u128"
           | "i8" | "i16" | "i32" | "i64" | "i128"
           | "bool" | "char" | "str" | "String"

option   ::= "Option" "<" texpr ">"
seq      ::= "Vec" "<" texpr ">"
set      ::= "BTreeSet" "<" texpr ">"
map      ::= "BTreeMap" "<" texpr "," texpr ">"
array    ::= "[" texpr ";" uint "]"
tuple    ::= "(" texpr ("," texpr)* ")"

fqpath   ::= ident ("::" ident)*     (* a participating user type, referenced by name *)
uint     ::= "0" | [1-9][0-9]*
ident    ::= [A-Za-z_][A-Za-z0-9_]*
```

`prim` is exhaustive: those fourteen tokens are the complete `PRIMITIVES` table. `usize`, `isize`,
`f32` and `f64` have no production because they are compile errors (§6).

Four constructs are **transparent** — they render as their inner type and contribute no token of
their own:

| Construct | Rendering | Why |
|---|---|---|
| `Box<T>` | `<T>` | postcard serializes `Box<T>` byte-identically to `T`, so the shape must match too. This is also how the recursive `Box<Expr>` inside the `hdl-ast` AST terminates: the inner type registers once and `insert_once` short-circuits the self-reference |
| `&T` | `<T>` | reference wrapper |
| `(T)` | `<T>` | parenthesized type |
| a macro group | `<T>` | invisible grouping |

Anything else with a path — a user type — renders as its full `::`-joined path *as spelled* and is
pushed as a child to register.

### 5.3 Container definitions

```ebnf
local_shape ::= "repr=" repr_value "@" cattrs body

repr_value  ::= ""                             (* reserved slot; always emitted empty *)
cattrs      ::= "#[" attr_item ("," attr_item)* "]"      (* "#[]" when there are none *)

body        ::= "unit"
              | "newtype" "(" elem ")"                      (* 1-field tuple struct *)
              | "tuple" "(" elem ("," elem)* ")"            (* n-field tuple struct *)
              | "struct" "{" field ("," field)* "}"         (* "struct{}" when empty *)
              | "enum" "{" variant ("," variant)* "}"

elem        ::= fattrs texpr
field       ::= fattrs ident ":" texpr          (* ident = the Rust field identifier *)
variant     ::= vattrs ident disc vbody         (* variants in SOURCE order *)
disc        ::= "=" int_lit | ""
vbody       ::= "" | "(" elem ("," elem)* ")" | "{" field ("," field)* "}"
fattrs      ::= cattrs
vattrs      ::= cattrs
int_lit     ::= "-"? uint
```

`repr_value` is a reserved slot: the grammar carries it so a future representation tag can enter
the shape without a grammar version bump, and the derive emits it empty for every type.

A registry line is the key, `=`, and the body:

```
<schema_name> "=" <local_shape>
```

so every line is self-identifying and a golden diff points at exactly one type.

### 5.4 Worked renderings

Full registry lines, pinned verbatim by `crates/testdata/sim_ir_canonical.txt`; the body half of
each is asserted individually by `crates/sim-ir/tests/frozen_shapes.rs`:

```
sim_ir::ProcFlags=repr=@#[]newtype(#[]u8)
sim_ir::RegionTag=repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}
sim_ir::JoinState=repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,#[]detached:Vec<u32>,#[]flags:sim_ir::ProcFlags}
sim_ir::SuspendState=repr=@#[]struct{#[]resume_pc:u32,#[]locals:Vec<sim_ir::FourState>,#[]join_state:sim_ir::JoinState,#[]wake_key:sim_ir::WakeKey,#[]call_stack:Vec<sim_ir::Frame>,#[]frame_arena:Vec<sim_ir::FourState>}
```

Read the third and fourth lines together to see the two rules working:

- `flags:sim_ir::ProcFlags` is a name reference. `ProcFlags(u8)` becoming `ProcFlags(u16)` leaves
  the `JoinState` line byte-identical and rewrites the `ProcFlags` line — and the root still flips,
  because the canonical string concatenates both.
- `locals` and `frame_arena` both render `Vec<sim_ir::FourState>`. The two field appearances are
  preserved because field count and field names are load-bearing; only the *definition* of
  `FourState` is deduplicated.

`ProcFlags` also shows why a newtype is distinct from its inner type: as a field type a bare `u8`
renders `u8`, but `ProcFlags` owns a registry entry whose body is `newtype(#[]u8)`. The postcard
bytes are identical; the shapes are not, and the hash follows the shape.

---

## 6. What the derive refuses

Every rejection is a `compile_error!` at the offending span, so the failure lands on the author's
machine rather than on a mis-decoded artifact.

| Rejected | Message |
|---|---|
| A type, lifetime or const-generic parameter | `SchemaHash does not support {type\|lifetime\|const-generic} parameters; schema types must be concrete` |
| A `union` | `SchemaHash does not support unions` |
| A `HashMap` or `HashSet` field type | ``SchemaHash: `{head}` is forbidden (nondeterministic order); use BTreeMap/BTreeSet`` |
| A `usize`, `isize`, `f32` or `f64` field type | ``SchemaHash: `{head}` is forbidden in schema types (platform-variant width / float breaks 3-OS byte-identity); use a fixed-width integer (u32/u64) instead`` |
| A qualified path `<T as Trait>::X` | `SchemaHash: qualified-path (<T as Trait>) field types are not supported` |
| `Fn(..)`-style type arguments | `SchemaHash: Fn-style type args not supported` |
| Any other type construct | `SchemaHash: unsupported field type construct` |
| A malformed `#[serde(..)]` | `SchemaHash: bad #[serde(..)]: {e}` |

The generic rejection is what keeps the grammar closed: every participating type is monomorphic, so
a shape string is a fixed `const` and never a function of instantiation.

`BTreeSet<T>` has an explicit production for a reason. Without one it would fall through to the
user-type arm, render as a path and recurse into a `BTreeSet::register` that does not exist.

---

## 7. serde attributes participate in the shape

A serde attribute can change the wire bytes while leaving the Rust shape untouched, so the
attribute surface is part of the hash. The derive collects `#[serde(..)]` keys, sorts them by a
fixed priority so that source order does not matter, and joins them with `,`.

| Key | Level | Wire effect | Rendered as | Priority |
|---|---|---|---|---|
| `rename="X"` | field / variant | wire field or variant name | `rename="X"` | 0 |
| `rename_all="..."` | container | renaming rule for every member | `rename_all="snake_case"` | 1 |
| `skip` | field / variant | removes the field from the wire | `skip` | 2 |
| `skip_serializing_if="path"` | field | conditional removal, value-dependent | `skip_serializing_if="path"` | 3 |
| `with="mod"` | field | replaces the (de)serialize impl outright | `with="mod"` | 4 |
| `default` / `default="path"` | field / container | permits a missing field on decode | `default` or `default="path"` | 5 |
| `flatten` | field | inlines into the parent, restructuring the wire | `flatten` | 6 |
| `tag="t"` | enum container | internally tagged: adds a tag field | `tag="t"` | 7 |
| `content="c"` | enum container | adjacently tagged, with `tag` | `content="c"` | 8 |
| `untagged` | enum container | removes the discriminant tag | `untagged` | 9 |
| `transparent` | container | newtype passthrough; wire becomes the inner type | `transparent` | 10 |
| `deny_unknown_fields` | container | decode strictness | `deny_unknown_fields` | 11 |
| `alias="X"` | field | accepts an additional name on decode | `alias="X"` | 12 |
| `other` | variant | catch-all variant, changing deserialize meaning | `other` | 13 |

Any other `#[serde(..)]` key is parsed and dropped, which keeps a malformed attribute loud while
leaving a shape-neutral one out of the hash.

Two rendering choices are worth stating explicitly:

- `skip_serializing_if` and `default="path"` hash the *presence* and the path string, not the
  predicate or constructor body. The function is a code reference, not a shape, and it can change
  without changing the wire grammar; its presence is what makes the field conditional.
- `with` hashes the module path string, because two different modules can produce entirely
  different wire bytes and the path is the only signal the macro has. What the module does
  internally is the Layer-1 blind spot of §1.2.

Under postcard a plain `rename` does not move a byte — the format is non-self-describing and field
names are not on the wire. It is captured anyway: it is cheap, it matters for any readable dump,
and it interacts with `alias` and `flatten`. The binary-critical keys — `skip`, `with`, `flatten`,
`tag`, `content`, `untagged`, `transparent`, `default`, `other` — are the ones that must never be
missed, and all of them are covered.

**Frozen types carry no serde attributes at all.** Every attribute slot in the `sim_ir::SimIr`
canonical string is exactly `#[]`, and `crates/sim-ir/tests/no_serde_attrs.rs` root-walks the whole
closure to assert it, so a new frozen type is covered without touching the test. `hdl-ast` contains
no `#[serde(..)]` either. The grammar keeps the productions so that attaching an attribute to a
frozen type later flips the hash, which is the correct outcome for a deliberate wire change.

---

## 8. Determinism rules

The canonical string, and therefore the hash, must be byte-identical on every supported platform
and toolchain. Nine rules hold that.

1. **No hash-ordered container anywhere.** `ShapeRegistry` is `BTreeMap` plus `BTreeSet`; the
   macro-time child dedup is a `Vec` plus a linear scan. A `HashMap`/`HashSet` field type is a
   compile error.
2. **Cross-type order is fully-qualified-name lexicographic**, from `BTreeMap` iteration over
   `&'static str` — a plain byte comparison, with no locale involved. `core::any::TypeId` appears
   nowhere in the design: its `Hash` and `Ord` are not stable across compiler releases. Name order
   is also strictly stronger than encounter order, because it yields the same substring regardless
   of which root drove the walk.
3. **Order within a type is source order**, baked into the `const` at macro-expansion time.
   Reordering fields or variants reorders a substring and flips the hash.
4. **Primitive widths are literal tokens.** `u32`, `u64` and `usize` are three different strings,
   and the last is a compile error. `crates/sim-ir/tests/no_float_usize.rs` additionally asserts
   that none of `usize`, `isize`, `f32`, `f64` appears anywhere in the SimIr canonical string,
   which catches an aliased reintroduction. Sequence lengths are safe: postcard varint-encodes
   `Vec` and `String` lengths, so a length is width-independent and only the element type enters
   the hash.
5. **The key is `module_path!()::Ident`**, a Rust path, never a serde rename (§4).
6. **No float, no locale, no clock enters the string.** The only values hashed are identifiers,
   type tokens, serde attribute string literals, discriminant integers and array lengths, all
   rendered in base ten without separators.
7. **Diagnostic types are outside both closures.** `sim-ir` does not depend on `diag`, and no
   `sim-ir` field names a diagnostic type, so none appears in the walk. `sim-ir` is span-free;
   locations ride an independent side table.
8. **Every participating type is concrete** — no type parameter, no lifetime, no const generic.
9. **The record separator is a literal `\n` (0x0A), never CRLF.** The Layer-3 RON golden pins its
   newline explicitly for the same reason.

---

## 9. The canonical string and the hash

```rust
pub fn canonical_string(&self) -> String {
    let mut s = String::new();
    s.push_str("vita-schema-v1\n");   // grammar-version sentinel + non-empty baseline
    for (name, shape) in &self.entries {
        s.push_str(name);
        s.push('=');
        s.push_str(shape);
        s.push('\n');
    }
    s
}

pub fn schema_hash<T: SchemaShape>() -> [u8; 32] {
    let mut reg = ShapeRegistry::new();
    T::register(&mut reg);
    blake3::hash(reg.canonical_string().as_bytes()).into()
}
```

The `vita-schema-v1` sentinel is the grammar version. It gives an empty registry a fixed baseline
and provides a single place to force a deliberate flip if the grammar itself changes.

The full string is materialized and hashed once. Every factor that decides a byte is
platform-invariant:

| Factor | Why it cannot vary |
|---|---|
| The set of participating types | The source AST plus `--locked` dependency resolution |
| Cross-type order | Lexicographic `str` ordering, a byte comparison; no `TypeId`, no locale |
| Order within a type | Source order, baked at compile time |
| Type tokens | Literal strings; the platform-variant widths are compile errors |
| Whitespace | None is emitted; the separator is a literal 0x0A |
| Discriminants and array lengths | Base-ten literals, no platform formatting |
| serde attribute strings | Copied verbatim from the syntax tree |
| The hash function | blake3, pure Rust, no C, pinned exactly at `=1.8.2` |

`local_shape()` and `schema_name()` are genuine compile-time `&'static str` values, because
everything a single type knows about itself is known at macro-expansion time. The hash is not:
`blake3::hash` is not a `const fn`, so the 32-byte value is produced at first use rather than
written as a literal `const`. `vita-schema` offers `cached_hash::<T>(&'static OnceLock<[u8;32]>)`
for a one-time cache. The semantics that matter — a stable 32-byte structural key per build — hold
either way, and the canonical string, not the hash function, is the contract; substituting a
`const fn` hasher over the same string would be a change of implementation, not of contract.

---

## 10. Three layers

| Layer | Mechanism | What it catches | Where |
|---|---|---|---|
| 1 | `#[derive(SchemaHash)]` + the `vita-schema` registry + blake3 | Rust shape edits and wire-affecting serde attributes. This is the runtime staleness and decode key stamped into the header | `vita-artifact-derive`, `vita-schema`, `vita-artifact` |
| 2 | The build fingerprint (`Provenance`) | Nothing. Tool version, git sha, dirty flag and profile are stamped for traceability and are explicitly excluded from the staleness key | `crates/vita-artifact/src/header.rs` |
| 3 | A `serde-reflection` RON golden diffed in `cargo test` | serde wire drift the syntax-attribute path cannot see, including a `with=` module whose encoding changes without its path changing | `crates/sim-ir/tests/reflection.rs` + `crates/testdata/sim_ir_registry.ron` |

Layer 3 runs the real serde derives through a tracer and serializes the resulting `Registry` to
RON, so it reflects `rename`, `skip`, `with`, `default`, `flatten` and `tag` automatically. It
cannot be Layer 1: it needs sample values and a runtime tracer, and it cannot be stamped into a
header. The division of labour is deliberate — Layer 1 is the runtime staleness gate, Layer 3 is
the reviewable wire oracle for a deliberate format change. The RON serializer's newline is pinned
to `"\n"` explicitly, because the pretty-printer's default picks the platform newline and would
break a byte-compared golden.

---

## 11. Goldens

| Golden | Location | Test |
|---|---|---|
| SimIr root hash `37fa4f1f37d433ad94a8a6f03d4ee6dd9d03317ac322b4bdb885a4030194aad6` | `EXPECTED_SIMIR_HASH`, `crates/sim-ir/tests/schema_hash.rs` | `schema_hash_is_pinned` |
| `Process` sub-pin `61db2e207ed69c2ff1dbf3fc0473b7ed9906fbeb6c42128ef9edf382b081f277` | `EXPECTED_PROCESS_HASH`, same file | `process_subpin` — a cheap regression signal for the runtime cluster, not the gate |
| SimIr canonical string, 38 lines (one sentinel plus 37 entries) | `crates/testdata/sim_ir_canonical.txt` | `canonical_string_golden` |
| SimIr serde-reflection registry | `crates/testdata/sim_ir_registry.ron` | `serde_reflection_ron_golden` |
| hdl-ast `SourceUnit` root hash `0de851401a5c5c02590e7fef62bbbc96a84b7b71954d4d79a7b78d4711da9bd1` | `EXPECTED`, `crates/hdl-ast/tests/schema_hash.rs` | `schema_hash_is_pinned`, `schema_hash_is_deterministic` |

Both hash goldens are single literals checked on every CI runner. Because the canonical string is
byte-identical everywhere, the *same* literal passes on every platform; a platform-dependent hash
would fail at least one runner. The CI matrix is Ubuntu, macOS and a RHEL 9 UBI container
(`.github/workflows/ci.yml`).

Regeneration is a sanctioned switch, never a hand edit:

```bash
REGEN_GOLDEN=1 cargo test -p sim-ir --test schema_hash -- --nocapture
```

That rewrites `crates/testdata/sim_ir_canonical.txt` and prints both hashes to paste in. The same
environment variable rewrites the RON golden from `crates/sim-ir/tests/reflection.rs` and prints
the two cli-local wire-shape hashes from `crates/cli/src/tests.rs`.

### 11.1 What a developer sees when a frozen field changes

Changing `Frame.return_pc` from `u32` to `u64` produces four signals, in this order:

1. `schema_hash_is_pinned` fails: `SCHEMA_HASH changed — a frozen sim-ir shape/serde-attr moved.
   If intentional: all .velab invalid -> bump format_version + update both goldens.`
2. `canonical_string_golden` fails on one line, naming the type and the field:
   `sim_ir::Frame=…struct{#[]return_pc:u32,…}` against `…return_pc:u64,…`.
3. `serde_reflection_ron_golden` fails, showing `Frame.return_pc` as `U32` against `U64`.
4. If all three are overridden and the build ships, an existing `.velab` meets the header gate and
   is refused with `E-ART-SCHEMA-MISMATCH` (`VITA-E9002`, exit 2) before a single body byte is
   deserialized. There is no silent mis-parse and no migration path.

The legitimate way forward is the deliberate re-freeze: bump `format_version`, regenerate every
artifact, and update both goldens through `REGEN_GOLDEN=1`. See
[17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) for the protocol and its cost.

---

## 12. The runtime gate

`vita-artifact` stamps the hash into the header at write time and compares it at read time.
`verify_header` runs three ordered checks, lowest first:

| Order | Condition | Code | Message |
|---|---|---|---|
| 1 | `format_version` differs | `E-ART-FORMAT-MISMATCH` (`VITA-E9001`) | ``format_version={got} but this tool expects {want}; regenerate with `velab` `` |
| 2 | `tool_semver_major` differs | `E-ART-VERSION-GATE` (`VITA-E9004`) | `produced by vitamin {got}.x, this tool is {want}.x; regenerate or install a matching vitamin` |
| 3 | `schema_hash` differs | `E-ART-SCHEMA-MISMATCH` (`VITA-E9002`) | ``sim-ir type shape changed between builds; rerun `velab` `` |

The header is decoded alone, so the body is never touched by a rejected artifact. Policy is
version-gate — refuse and rebuild — and there is no migration machinery anywhere in the crate. All
three rejections exit 2, which is distinct from a design error so that CI rebuilds upstream instead
of debugging RTL. Full gate and exit-code detail is in
[14-staged-artifacts.md](14-staged-artifacts.md), and the code entries are in
[15-error-code-reference.md](15-error-code-reference.md).

---

## 13. Status at HEAD

| Item | State |
|---|---|
| The derive, the registry, the canonical grammar, all eight rejections, all fourteen serde keys | Implemented |
| Both roots stamped and gated; three goldens for `sim-ir`, one for `hdl-ast` | Implemented |
| Frozen types attribute-free; no `usize`/`isize`/`f32`/`f64` token; no dangling body reference | Enforced by tests that root-walk the closure, so a new frozen type is covered automatically |
| `repr_value` in the grammar | A reserved slot; the derive always emits it empty and reads no `#[repr]` |
| `cached_hash` | Available in `vita-schema`; every stamp and gate at HEAD calls `schema_hash::<T>()` directly, so nothing exercises the cache |
| The gate-3 message text | Names `sim-ir` unconditionally, so a stale `.vu` — whose root is `hdl_ast::SourceUnit` — receives wording that names the wrong crate. The code and the exit class are correct |
| The `with=` blind spot | Real in the grammar, with no exposure at HEAD: neither `sim-ir` nor `hdl-ast` contains a single `#[serde(..)]` attribute, and a test asserts it for the `sim-ir` closure |

---

## Related documents

- [14-staged-artifacts.md](14-staged-artifacts.md) — the container, the header, the trailers, the staleness gate
- [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) — the frozen types this hash covers
- [03-build-and-portability.md](03-build-and-portability.md) — the no-build-script rule and the dependency pins
- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) — why locations ride out of band
- [15-error-code-reference.md](15-error-code-reference.md) — the 9xxx artifact codes
- [09-testing-and-verification.md](09-testing-and-verification.md) — the golden gates that hold the determinism contract
