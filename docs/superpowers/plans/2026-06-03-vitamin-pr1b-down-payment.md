# vitamin PR1-B (SuspendState Down-Payment) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the vitamin cargo workspace plus the *determinism machinery* (`#[derive(SchemaHash)]` + `ShapeRegistry` + golden gates) and freeze a golden hash over the self-contained `SuspendState` runtime-state closure — deferring the root `Process` hash to M3.

**Architecture:** A 15-production + 2-dev cargo workspace (MSRV 1.82, **edition 2021**). Three leaf crates carry real code (`diag`, `vita-schema`, `vita-artifact-derive`); `sim-ir` carries the 9-type `SuspendState` closure (the only sim-ir subtree with **no** `Stmt`/`Expr`/`BasicBlock`/`Sensitivity` dependency); the other 11 production crates + `cli` are compile-green stubs. The schema hash is computed at runtime (blake3 is not `const fn`) via a `OnceLock`, fed a canonical, byte-identical, `BTreeMap`-ordered shape string. Every inter-type edge in the closure is a `u32`/`u64` arena index, so the type-reachability graph is a finite acyclic DAG.

**Tech Stack:** Rust 1.82 / edition 2021; `blake3 =1.8.2` (pinned below the edition-2024 boundary), `syn 2.0` (full+extra-traits), `quote`/`proc-macro2`, `serde` (derive), `serde-reflection 0.6` (dev), `hex` (test). cargo-only build (no `build.rs` shell-out). GitHub Actions: `checkout@v6`, `Swatinem/rust-cache@v2`, `dtolnay/rust-toolchain@stable`/`@1.82.0`.

---

## Scope

**In scope (PR1-B):**
- Cargo workspace skeleton: root `Cargo.toml` (17 members), `[profile.release]`, `rust-toolchain.toml` 1.82, `.gitignore`, CI (`build-native` 2-OS + `build-rhel` UBI + `msrv`), committed `Cargo.lock`.
- `diag` (real, leaf): `Severity` + 36-variant `MsgCode` + `LogEvent`/`Diagnostic`/`Frame`/`SourceLoc`/`TimeStamp`/`LogSink`, IO-free. **MsgCode↔doc-15 bijection gate** test.
- `vita-schema` (real, NEW leaf): `SchemaShape` trait + `ShapeRegistry` (`BTreeMap`/`BTreeSet`) + `schema_hash::<T>()` + `OnceLock` cache; dep `blake3` only.
- `vita-artifact-derive` (real, leaf): `#[derive(SchemaHash)]` proc-macro.
- `sim-ir` (real-partial): the 9 frozen `SuspendState`-closure types + `FourState`/`EdgeKind` (scalar leaf enums newly frozen here) with `serde` + `SchemaHash` derives, in the crate **root module**.
- Golden gates: `schema_hash_is_pinned` (over `SuspendState`), canonical-string golden file, frozen-no-serde-attr guard, `serde-reflection` RON golden (Layer 3).
- Stubs: 11 remaining production crates + `cli` multicall stub + 2 dev bins (all `cargo build --workspace` green).
- Doc sync: `03-build-and-portability.md` `edition 2024 → 2021`, `serde-reflection 0.4 → 0.6`.

**Out of scope (deferred to M3 / later PRs):** `Process`/`BasicBlock`/`Stmt`/`Expr`/`Sensitivity`/`Terminator` bodies and the **root** `Process` hash; `vita-artifact` header stamping; `vita-log`; real lexer/parser/elaborate/engine logic; `cargo audit` job.

---

## Why the root is `SuspendState`, not `Process`

`Process = { sensitivity: Sensitivity, body: Vec<BasicBlock>, entry: u32, suspend: SuspendState }`. Both `Sensitivity` and `BasicBlock`(→`Stmt`→`Expr`) are unspecified (14 §1 line 162 names only `Terminator`/`JoinKind`). But the `SuspendState` closure is **self-contained** — walking every field (16 §1 lines 37–42):

```
SuspendState → {resume_pc:u32, locals:Vec<FourState>, join_state:JoinState,
                wake_key:WakeKey, call_stack:Vec<Frame>, frame_arena:Vec<FourState>}
JoinState    → {parent:Option<u32>, children:Vec<u32>, detached:Vec<u32>, flags:ProcFlags}
WakeKey      → {cond:WakeCond, region:RegionTag, tie_break:u32}
WakeCond     → enum{ Edge{net:u32,kind:EdgeKind}, Level{nets:Vec<u32>}, WaitTrue{expr:u32},
                     TimeAbs{tick:u64}, NamedEvent{ev:u32}, Join{join_ref:u32} }
Frame        → {return_pc:u32, callee_entry:u32, locals_base:u32, locals_len:u32, is_automatic:bool}
ProcFlags(u8); RegionTag(unit-enum×4); EdgeKind(unit-enum); FourState(unit-enum)
```

The only members of the unspecified set this closure touches are **`FourState`** (via `locals`/`frame_arena`) and **`EdgeKind`** (via `WakeCond::Edge.kind`) — both are non-recursive scalar leaf enums with no `Stmt`/`Expr` edge, so PR1-B freezes them here. `Process`/`BasicBlock`/`Stmt`/`Expr`/`Sensitivity`/`Terminator` are **not defined** in this PR.

---

## Three load-bearing design locks (resolve research-verified defects)

1. **`extern crate self as sim_ir;` + all frozen types in the crate root module.** `schema_name()` is `concat!(module_path!(), "::", Ident)`, and `module_path!()` resolves to the *defining* module. Defining every frozen type directly in `sim-ir/src/lib.rs` makes `module_path!() == "sim_ir"`, so `schema_name()` is exactly `sim_ir::ProcFlags` etc. — matching doc 16's golden strings. Spelling each sibling field type as `sim_ir::Foo` (enabled by `extern crate self as sim_ir;`) makes the derive's rendered body reference (`render_full_path` = the spelled path) **equal** the child's `schema_name()` (registry key). This single trick resolves verdict Defects 2.3 + 2.4 and reproduces the doc-16 golden literals verbatim.

2. **`canonical_string()` prepends the registry key.** The derive cannot bake `module_path!()` into the `local_shape()` body literal alongside the struct body, so `local_shape()` returns the body only (`"repr=@#[]struct{…}"`). The registry stores `BTreeMap<schema_name, local_shape>` and emits `name + "=" + shape + "\n"` per entry, yielding the self-identifying line `sim_ir::JoinState=repr=@#[]struct{…}`. (Minor reconciliation of doc 16's pseudocode, which assumed the fqname was embedded in `shape`.)

3. **No global `RUSTFLAGS=-D warnings` in CI** (verdict Defect 3.1) — `-D warnings` lives only in the `cargo clippy` step, never env-wide, so 1.82/macOS dependency warnings cannot spuriously fail the build.

---

## File Structure

```
016_claude_rtl/
├── Cargo.toml                      # [workspace] 17 members, [workspace.deps], [profile.release]
├── Cargo.lock                      # committed (--locked determinism)
├── rust-toolchain.toml             # channel 1.82.0 + components + targets
├── .gitignore                      # /target, **/*.rs.bk
├── .github/workflows/ci.yml        # build-native(2-OS) + build-rhel(UBI) + msrv(1.82)
└── crates/
    ├── diag/                       # REAL: Severity, MsgCode(36), LogEvent, LogSink, Diagnostic…
    │   ├── Cargo.toml
    │   ├── src/lib.rs              # re-exports
    │   ├── src/severity.rs
    │   ├── src/code.rs             # msgcodes! macro → MsgCode + metadata
    │   ├── src/event.rs            # LogEvent/Diagnostic/Frame/SourceLoc/TimeStamp/LogSink
    │   └── tests/bijection.rs      # MsgCode ↔ docs/preview/15 §0–9
    ├── vita-schema/                # REAL (NEW leaf): SchemaShape + ShapeRegistry + schema_hash
    │   ├── Cargo.toml              # dep: blake3 =1.8.2
    │   ├── src/lib.rs
    │   └── tests/registry.rs       # hand-written impls: determinism, dedup, collision
    ├── vita-artifact-derive/       # REAL: #[derive(SchemaHash)] proc-macro
    │   ├── Cargo.toml              # [lib] proc-macro=true; syn/quote/proc-macro2; dev: vita-schema
    │   ├── src/lib.rs
    │   └── tests/render.rs         # toy types → expected shape strings
    ├── sim-ir/                     # REAL-PARTIAL: 9 frozen types + FourState/EdgeKind
    │   ├── Cargo.toml              # deps: serde, vita-schema, vita-artifact-derive; dev: hex, serde-reflection, blake3
    │   ├── src/lib.rs              # extern crate self as sim_ir; all frozen types
    │   └── tests/
    │       ├── frozen_shapes.rs    # per-type local_shape() == doc-16 golden
    │       ├── schema_hash.rs      # schema_hash_is_pinned + canonical-string golden
    │       ├── no_serde_attrs.rs   # frozen cluster carries only #[] slots
    │       └── reflection.rs       # serde-reflection RON golden (Layer 3)
    ├── testdata/sim_ir_canonical.txt   # committed canonical-string golden
    ├── testdata/sim_ir_registry.ron    # committed serde-reflection golden
    └── (stubs) hdl-preprocess, hdl-lexer, hdl-parser, hdl-ast, elaborate,
        sim-engine, hdl-builtins, vcd-writer, vita-artifact, vita-log, cli,
        vcd-diff, corpus-runner            # empty lib.rs / multicall main stub
```

---

## Task 1: Workspace skeleton (all crates compile-green)

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `.github/workflows/ci.yml`
- Create: `crates/<name>/Cargo.toml` + `crates/<name>/src/lib.rs` for all 17 members (stubs for now)

- [ ] **Step 1: Root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = [
    "crates/hdl-preprocess",
    "crates/hdl-lexer",
    "crates/hdl-parser",
    "crates/hdl-ast",
    "crates/elaborate",
    "crates/sim-ir",
    "crates/sim-engine",
    "crates/hdl-builtins",
    "crates/vcd-writer",
    "crates/diag",
    "crates/vita-artifact",
    "crates/vita-artifact-derive",
    "crates/vita-schema",
    "crates/vita-log",
    "crates/cli",
    # dev/test only (publish = false)
    "crates/vcd-diff",
    "crates/corpus-runner",
]

[workspace.package]
edition = "2021"               # NOT 2024 — edition 2024 needs rustc >= 1.85; MSRV is pinned 1.82
rust-version = "1.82"
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-org/vitamin"

[workspace.dependencies]
serde            = { version = "1.0", features = ["derive"] }
postcard         = { version = "1.1", features = ["use-std"] }
blake3           = "=1.8.2"    # PIN: 1.8.3+ is edition 2024 (rustc >= 1.85). Pure-Rust, no const-fn hash.
syn              = { version = "2.0", features = ["full", "extra-traits"] }
quote            = "1.0"
proc-macro2      = "1.0"
# dev / test only
hex              = "0.4"
serde-reflection = "0.6"       # spec's 0.4 is stale; 0.6 = edition 2021, MSRV 1.72

[profile.release]
opt-level     = 3
lto           = "thin"
codegen-units = 1
strip         = "symbols"

[profile.dev.package."sim-engine"]
opt-level = 2
```

- [ ] **Step 2: `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.82.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
```

- [ ] **Step 3: `.gitignore`**

```gitignore
/target
**/*.rs.bk
```

- [ ] **Step 4: Stub crates.** For each of the 17 members create `crates/<name>/Cargo.toml`:

```toml
[package]
name = "<name>"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
# dev/test-only members add:  publish = false
```

and `crates/<name>/src/lib.rs`:

```rust
//! <name> — stub (PR1-B). Real implementation lands in a later PR.
```

For `cli`, instead use `src/main.rs` (a binary) — see Step 5. For `vcd-diff`/`corpus-runner` add `publish = false`.

- [ ] **Step 5: `cli` multicall stub.** `crates/cli/Cargo.toml`:

```toml
[package]
name = "cli"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "vita"
path = "src/main.rs"

[features]
separate-bins = []
```

`crates/cli/src/main.rs`:

```rust
//! vita multicall driver — stub (PR1-B). argv[0] basename dispatch lands later.
fn main() {
    let arg0 = std::env::args_os().next();
    let applet = arg0
        .as_deref()
        .and_then(|s| std::path::Path::new(s).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("vita")
        .to_string();
    eprintln!("vitamin: {applet}: not implemented yet (PR1-B scaffold)");
    std::process::exit(3); // exit class 3 = CLI/usage
}
```

- [ ] **Step 6: CI workflow** `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always   # NOTE: do NOT set RUSTFLAGS=-D warnings globally — it breaks dep builds on 1.82.

jobs:
  # Ubuntu + macOS native. The SCHEMA_HASH golden test runs identically on both
  # = the 2-platform determinism contract (a per-OS hash divergence fails CI).
  build-native:
    name: build & test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@1.82.0
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - name: build
        run: cargo build --workspace --locked
      - name: test
        run: cargo test --workspace --locked

  # RHEL9 UBI container on a GitHub ubuntu runner (glibc/pkg parity).
  build-rhel:
    name: build & test (RHEL9/UBI)
    runs-on: ubuntu-latest
    container: redhat/ubi9
    steps:
      - uses: actions/checkout@v6
      - name: install C linker
        run: dnf install -y gcc
      - uses: dtolnay/rust-toolchain@1.82.0
      - uses: Swatinem/rust-cache@v2
        with:
          key: rhel9
      - run: cargo build --workspace --locked
      - run: cargo test --workspace --locked

  # MSRV regression guard.
  msrv:
    name: MSRV 1.82.0
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@1.82.0
      - uses: Swatinem/rust-cache@v2
        with:
          key: msrv
      - run: cargo build --workspace --locked
      - run: cargo test --workspace --locked
```

- [ ] **Step 7: Build + lock**

Run: `cargo build --workspace && cargo fmt --all -- --check`
Expected: clean build of 17 stub crates; `Cargo.lock` generated.

- [ ] **Step 8: Commit**

```bash
git add 016_claude_rtl/Cargo.toml 016_claude_rtl/Cargo.lock 016_claude_rtl/rust-toolchain.toml \
        016_claude_rtl/.gitignore 016_claude_rtl/.github 016_claude_rtl/crates
git commit -m "Add vitamin cargo workspace skeleton (17 crates, MSRV 1.82, 3-OS CI)"
```

---

## Task 2: `diag` crate — Severity + MsgCode(36) + event model + bijection gate

**Files:**
- Modify: `crates/diag/Cargo.toml`
- Create: `crates/diag/src/{lib.rs,severity.rs,code.rs,event.rs}`
- Test: `crates/diag/tests/bijection.rs`

- [ ] **Step 1: Write the failing bijection test** `crates/diag/tests/bijection.rs`

```rust
//! CI sync gate: the MsgCode enum and docs/preview/15 §0–9 body codes are 1:1.
//! Appendix A reserved codes are excluded (they are not yet enum variants).
use diag::MsgCode;

const DOC15: &str = include_str!("../../../docs/preview/15-error-code-reference.md");

/// Extract body mnemonics: lines `### VITA-... · `CODE` (sev)` before "## 부록 A".
fn doc_mnemonics() -> Vec<String> {
    let body = DOC15.split("## 부록 A").next().unwrap();
    let mut out = Vec::new();
    for line in body.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix("### ") {
            // form: VITA-E0001 · `E-CLI-BAD-FLAG` (Error)
            if let Some(start) = rest.find('`') {
                if let Some(end) = rest[start + 1..].find('`') {
                    out.push(rest[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    out
}

#[test]
fn msgcode_matches_doc15_body_one_to_one() {
    let mut doc: Vec<String> = doc_mnemonics();
    let mut enum_codes: Vec<String> =
        MsgCode::ALL.iter().map(|c| c.mnemonic().to_string()).collect();
    doc.sort();
    doc.dedup();
    enum_codes.sort();

    assert_eq!(enum_codes.len(), 36, "MsgCode must have exactly 36 body variants");
    assert_eq!(
        enum_codes, doc,
        "MsgCode enum and docs/preview/15 §0–9 diverged.\n\
         Adding a code requires a doc entry (and vice-versa)."
    );
}

#[test]
fn mnemonics_and_numbers_are_unique() {
    let mut m: Vec<&str> = MsgCode::ALL.iter().map(|c| c.mnemonic()).collect();
    let mut n: Vec<&str> = MsgCode::ALL.iter().map(|c| c.code_num()).collect();
    let (lm, ln) = (m.len(), n.len());
    m.sort(); m.dedup();
    n.sort(); n.dedup();
    assert_eq!(m.len(), lm, "duplicate mnemonic");
    assert_eq!(n.len(), ln, "duplicate VITA-#### number");
}
```

Run: `cargo test -p diag --test bijection`
Expected: FAIL — `diag::MsgCode` does not exist yet.

- [ ] **Step 2: `Severity`** `crates/diag/src/severity.rs`

```rust
/// 5-level severity lattice (13 §Severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Note,
    Info,
    Warning,
    Error,
    Fatal,
}

impl Severity {
    /// Output token prefix, e.g. `error` in `error[VITA-E3001]:`.
    pub const fn token(self) -> &'static str {
        match self {
            Severity::Note => "note",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Fatal => "fatal",
        }
    }
}
```

- [ ] **Step 3: `MsgCode` via `msgcodes!` macro** `crates/diag/src/code.rs`

```rust
use crate::Severity;

/// Generates the exhaustive `MsgCode` enum + metadata accessors from a table.
macro_rules! msgcodes {
    ($( $variant:ident => ($mnemonic:literal, $num:literal, $sev:ident, $title:literal) ),* $(,)?) => {
        /// Stable, exhaustive diagnostic code. mnemonic is the primary stable key.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum MsgCode { $( $variant ),* }

        impl MsgCode {
            /// All body codes, declaration order.
            pub const ALL: &'static [MsgCode] = &[ $( MsgCode::$variant ),* ];

            /// Primary stable mnemonic, e.g. `E-ELAB-MULTIDRIVER`.
            pub const fn mnemonic(self) -> &'static str {
                match self { $( MsgCode::$variant => $mnemonic ),* }
            }
            /// grep-friendly number, e.g. `VITA-E3001`.
            pub const fn code_num(self) -> &'static str {
                match self { $( MsgCode::$variant => $num ),* }
            }
            /// Default severity for this code.
            pub const fn default_severity(self) -> Severity {
                match self { $( MsgCode::$variant => Severity::$sev ),* }
            }
            /// Short title (the `vita explain` headline).
            pub const fn title(self) -> &'static str {
                match self { $( MsgCode::$variant => $title ),* }
            }
        }
    };
}

msgcodes! {
    // 0xxx GENERAL / SYSTEM
    CliBadFlag             => ("E-CLI-BAD-FLAG",            "VITA-E0001", Error,   "unknown or invalid command-line flag"),
    LimitErrors            => ("F-LIMIT-ERRORS",            "VITA-F0002", Fatal,   "error limit reached; aborting stage"),
    // 1xxx PREPROCESS
    PpIncludeNotFound      => ("E-PP-INCLUDE-NOT-FOUND",    "VITA-E1001", Error,   "`include file not found on search path"),
    PpMacroArity           => ("E-PP-MACRO-ARITY",         "VITA-E1002", Error,   "function-like macro called with wrong arity"),
    LintUnclosed           => ("W-LINT-UNCLOSED",          "VITA-W1003", Warning, "inline lint_off never closed before EOF"),
    // 2xxx PARSE
    DupUnit                => ("E-DUP-UNIT",                "VITA-E2001", Error,   "design unit redefined"),
    ParseUnexpectedToken   => ("E-PARSE-UNEXPECTED-TOKEN",  "VITA-E2002", Error,   "unexpected token"),
    ParseImplicitNet       => ("W-PARSE-IMPLICIT-NET",      "VITA-W2003", Warning, "implicit net inferred (default_nettype wire)"),
    // 3xxx ELABORATE
    ElabMultidriver        => ("E-ELAB-MULTIDRIVER",        "VITA-E3001", Error,   "net driven by multiple structural drivers"),
    ElabPortMismatch       => ("E-ELAB-PORT-MISMATCH",      "VITA-E3002", Error,   "instance port binding incompatible with module"),
    ElabUnresolvedInstance => ("E-ELAB-UNRESOLVED-INSTANCE","VITA-E3003", Error,   "cannot resolve instantiated module"),
    ElabUserError          => ("E-ELAB-USER-ERROR",         "VITA-E3004", Error,   "elaboration-time $error"),
    ElabUserFatal          => ("F-ELAB-USER-FATAL",         "VITA-F3005", Fatal,   "elaboration-time $fatal"),
    ElabUserInfo           => ("I-ELAB-USER-INFO",          "VITA-I3006", Info,    "elaboration-time $info"),
    ElabUserWarning        => ("W-ELAB-USER-WARNING",       "VITA-W3007", Warning, "elaboration-time $warning"),
    ElabWidthTrunc         => ("W-ELAB-WIDTH-TRUNC",        "VITA-W3008", Warning, "width mismatch truncated/extended"),
    // 4xxx RUNTIME
    RunAssertFail          => ("E-RUN-ASSERT-FAIL",         "VITA-E4001", Error,   "assertion failed (no action block)"),
    RunRange               => ("E-RUN-RANGE",               "VITA-E4002", Error,   "runtime index/select out of range"),
    RunUserError           => ("E-RUN-USER-ERROR",          "VITA-E4003", Error,   "runtime $error"),
    RunFatal               => ("F-RUN-FATAL",               "VITA-F4004", Fatal,   "runtime $fatal (implicit $finish)"),
    RunUserInfo            => ("I-RUN-USER-INFO",           "VITA-I4005", Info,    "runtime $info"),
    RunNoLocations         => ("W-RUN-NO-LOCATIONS",        "VITA-W4006", Warning, "snapshot has no location side-table"),
    RunUserWarning         => ("W-RUN-USER-WARNING",        "VITA-W4007", Warning, "runtime $warning"),
    // 8xxx FILELIST
    FlistCycle             => ("E-FLIST-CYCLE",             "VITA-E8001", Error,   "filelist cycle"),
    FlistDepth             => ("E-FLIST-DEPTH",             "VITA-E8002", Error,   "filelist nesting exceeded depth cap"),
    FlistDupCtxConflict    => ("E-FLIST-DUP-CTX-CONFLICT",  "VITA-E8003", Error,   "same source twice under differing sticky context"),
    FlistGlob              => ("E-FLIST-GLOB",              "VITA-E8004", Error,   "wildcard not allowed in filelist"),
    FlistNotFound          => ("E-FLIST-NOT-FOUND",         "VITA-E8005", Error,   "filelist or referenced path not found"),
    FlistUndefEnv          => ("E-FLIST-UNDEF-ENV",         "VITA-E8006", Error,   "undefined environment variable in filelist"),
    FlistWrongStage        => ("E-FLIST-WRONG-STAGE",       "VITA-E8007", Error,   "filelist directive wrong for invoking stage"),
    FlistMixedBase         => ("W-FLIST-MIXED-BASE",        "VITA-W8008", Warning, "-f inside -F frame re-anchors to CWD"),
    FlistOverride          => ("W-FLIST-OVERRIDE",          "VITA-W8009", Warning, "single-value knob overridden (last-wins)"),
    // 9xxx ARTIFACT
    ArtFormatMismatch      => ("E-ART-FORMAT-MISMATCH",     "VITA-E9001", Error,   "artifact magic/format_version mismatch"),
    ArtSchemaMismatch      => ("E-ART-SCHEMA-MISMATCH",     "VITA-E9002", Error,   "artifact schema_hash mismatch"),
    ArtStaleUpstream       => ("E-ART-STALE-UPSTREAM",      "VITA-E9003", Error,   "stale upstream snapshot (RULE V)"),
    ArtVersionGate         => ("E-ART-VERSION-GATE",        "VITA-E9004", Error,   "producer tool semver-major incompatible"),
}
```

- [ ] **Step 4: Event model + `LogSink`** `crates/diag/src/event.rs`

```rust
use crate::{MsgCode, Severity};

/// Resolved source location (file:line:col + byte span).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub byte_start: u32,
    pub byte_end: u32,
}

/// Simulation timestamp attached to runtime severity events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeStamp {
    pub ticks: u64,
}

/// Context frame: include/macro/-f stack or instance/hierarchy path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub label: String,
    pub location: Option<SourceLoc>,
}

/// One diagnostic. `diag` renders exactly this; it knows nothing of counts/exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: MsgCode,
    pub message: String,
    pub location: Option<SourceLoc>,
    pub context: Vec<Frame>,
    pub sim_time: Option<TimeStamp>,
}

/// Non-diagnostic progress event (banner, "reading file X", run summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub message: String,
}

/// User-visible RTL text ($display/$write/$monitor/$strobe) — no severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtlText {
    pub text: String,
    pub sim_time: Option<TimeStamp>,
}

/// Single event model (13 §single event model). One stream, fanned out by the sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    Diagnostic(Diagnostic),
    Progress(ProgressEvent),
    RtlOutput(RtlText),
}

/// Boundary trait. Emitters take `&dyn LogSink` and depend only on `diag`,
/// keeping `diag` a leaf (no IO / tracing). The concrete sink lives in `vita-log`.
pub trait LogSink {
    fn emit(&self, event: LogEvent);
}
```

- [ ] **Step 5: `lib.rs` re-exports + `Cargo.toml`** `crates/diag/src/lib.rs`

```rust
//! diag — diagnostic data model + renderer boundary (leaf, IO-free).
mod code;
mod event;
mod severity;

pub use code::MsgCode;
pub use event::{Diagnostic, Frame, LogEvent, LogSink, ProgressEvent, RtlText, SourceLoc, TimeStamp};
pub use severity::Severity;
```

`crates/diag/Cargo.toml` (no workspace deps — pure leaf):

```toml
[package]
name = "diag"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p diag`
Expected: PASS — `msgcode_matches_doc15_body_one_to_one` (36 == 36, sets equal), `mnemonics_and_numbers_are_unique`.

- [ ] **Step 7: Commit**

```bash
git add 016_claude_rtl/crates/diag
git commit -m "Add diag crate: Severity + MsgCode(36) + LogEvent/LogSink + doc-15 bijection gate"
```

---

## Task 3: `vita-schema` crate — SchemaShape + ShapeRegistry + schema_hash

**Files:**
- Modify: `crates/vita-schema/Cargo.toml`
- Create: `crates/vita-schema/src/lib.rs`
- Test: `crates/vita-schema/tests/registry.rs`

- [ ] **Step 1: Write the failing registry test** `crates/vita-schema/tests/registry.rs`

```rust
//! Determinism / dedup / collision behaviour, exercised with hand-written impls
//! (no derive dependency — this crate is pure runtime).
use vita_schema::{schema_hash, ShapeRegistry, SchemaShape};

// Leaf type: `demo::Flags` newtype(u8).
struct Flags;
impl SchemaShape for Flags {
    fn schema_name() -> &'static str { "demo::Flags" }
    fn local_shape() -> &'static str { "repr=@#[]newtype(#[]u8)" }
    fn register(reg: &mut ShapeRegistry) {
        if !reg.insert_once(Self::schema_name(), Self::local_shape()) { return; }
    }
}

// Parent type referencing Flags by name, twice (dedup must intern once).
struct Pair;
impl SchemaShape for Pair {
    fn schema_name() -> &'static str { "demo::Pair" }
    fn local_shape() -> &'static str { "repr=@#[]struct{#[]a:demo::Flags,#[]b:demo::Flags}" }
    fn register(reg: &mut ShapeRegistry) {
        if !reg.insert_once(Self::schema_name(), Self::local_shape()) { return; }
        <Flags as SchemaShape>::register(reg);
        <Flags as SchemaShape>::register(reg); // second call is a no-op (dedup)
    }
}

#[test]
fn canonical_string_is_sorted_and_self_identifying() {
    let mut reg = ShapeRegistry::new();
    Pair::register(&mut reg);
    let s = reg.canonical_string();
    // sentinel first, then fqname-sorted entries, each `name=shape`, '\n' separated.
    assert_eq!(
        s,
        "vita-schema-v1\n\
         demo::Flags=repr=@#[]newtype(#[]u8)\n\
         demo::Pair=repr=@#[]struct{#[]a:demo::Flags,#[]b:demo::Flags}\n"
    );
}

#[test]
fn hash_is_stable_and_order_independent() {
    let h1 = schema_hash::<Pair>();
    let h2 = schema_hash::<Pair>();
    assert_eq!(h1, h2);
    // Hashing the same canonical string must equal blake3 of that string.
    let mut reg = ShapeRegistry::new();
    Pair::register(&mut reg);
    let expect: [u8; 32] = blake3::hash(reg.canonical_string().as_bytes()).into();
    assert_eq!(h1, expect);
}

#[test]
#[should_panic(expected = "collision")]
fn name_collision_with_differing_shape_panics() {
    let mut reg = ShapeRegistry::new();
    reg.insert_once("demo::Flags", "repr=@#[]newtype(#[]u8)");
    reg.insert_once("demo::Flags", "repr=@#[]newtype(#[]u16)"); // same name, diff shape -> panic
}
```

Add `blake3` as a dev-dependency for the test's cross-check.

Run: `cargo test -p vita-schema --test registry`
Expected: FAIL — crate is still a stub.

- [ ] **Step 2: Implement `vita-schema`** `crates/vita-schema/src/lib.rs`

```rust
//! vita-schema — structural schema-shape registry + blake3 hash (leaf; dep: blake3).
//!
//! `#[derive(SchemaHash)]` (in vita-artifact-derive) implements `SchemaShape` for
//! every participating sim-ir / hdl-ast type. The hash collapses the whole
//! type-reachability closure (acyclic DAG) into one 32-byte blake3 value.
use alloc_compat::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

mod alloc_compat {
    pub use std::collections::{BTreeMap, BTreeSet};
}

/// Implemented by every participating serde type. Children referenced by name,
/// never inlined (so a nested change flips the registry entry, hence the root).
pub trait SchemaShape {
    /// Rename-STABLE canonical key, `concat!(module_path!(),"::",Ident)`.
    fn schema_name() -> &'static str;
    /// LOCAL shape body only (this type's fields/variants + own serde attrs).
    /// Children appear as their `schema_name()` reference strings, not inlined.
    fn local_shape() -> &'static str;
    /// Register self then recurse into each distinct child (DFS, insert_once-guarded).
    fn register(reg: &mut ShapeRegistry);
}

/// Order-stable registry. No HashMap/HashSet anywhere (3-OS byte stability).
pub struct ShapeRegistry {
    entries: BTreeMap<&'static str, &'static str>, // schema_name -> local_shape
    visited: BTreeSet<&'static str>,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        ShapeRegistry { entries: BTreeMap::new(), visited: BTreeSet::new() }
    }

    /// Insert this type once. Returns false if already present (short-circuits DFS).
    /// Re-registration MUST carry an identical shape, else two types alias one name.
    pub fn insert_once(&mut self, name: &'static str, shape: &'static str) -> bool {
        if !self.visited.insert(name) {
            // plain assert (survives release builds) — a real collision is a correctness bug.
            assert_eq!(
                self.entries.get(name),
                Some(&shape),
                "SchemaHash name collision: {name} registered with two different shapes"
            );
            return false;
        }
        self.entries.insert(name, shape);
        true
    }

    /// Deterministic canonical string: sentinel + fqname-sorted `name=shape\n` lines.
    pub fn canonical_string(&self) -> String {
        let mut s = String::new();
        s.push_str("vita-schema-v1\n"); // format-version sentinel + non-empty baseline
        for (name, shape) in &self.entries {
            // BTreeMap iteration = fqname lexicographic = identical on every OS.
            s.push_str(name);
            s.push('=');
            s.push_str(shape);
            s.push('\n'); // literal 0x0A, never CRLF
        }
        s
    }
}

impl Default for ShapeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Drive the reachability closure from `T`, canonicalize, hash once.
pub fn schema_hash<T: SchemaShape>() -> [u8; 32] {
    let mut reg = ShapeRegistry::new();
    T::register(&mut reg);
    blake3::hash(reg.canonical_string().as_bytes()).into()
}

/// One-time cached hash for a fixed root. (blake3::hash is not a const fn, so the
/// value is produced on first access rather than as a literal const.)
pub fn cached_hash<T: SchemaShape>(cell: &'static OnceLock<[u8; 32]>) -> [u8; 32] {
    *cell.get_or_init(schema_hash::<T>)
}
```

`crates/vita-schema/Cargo.toml`:

```toml
[package]
name = "vita-schema"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
blake3 = { workspace = true }

[dev-dependencies]
blake3 = { workspace = true }
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p vita-schema`
Expected: PASS — canonical string exact match, hash stable + equals blake3 of canonical, collision panics.

- [ ] **Step 4: Commit**

```bash
git add 016_claude_rtl/crates/vita-schema 016_claude_rtl/Cargo.lock
git commit -m "Add vita-schema leaf crate: SchemaShape trait + ShapeRegistry + blake3 schema_hash"
```

---

## Task 4: `vita-artifact-derive` — `#[derive(SchemaHash)]` proc-macro

**Files:**
- Modify: `crates/vita-artifact-derive/Cargo.toml`
- Create: `crates/vita-artifact-derive/src/lib.rs`
- Test: `crates/vita-artifact-derive/tests/render.rs`

- [ ] **Step 1: Write the failing render test** `crates/vita-artifact-derive/tests/render.rs`

```rust
//! Acceptance: derived local_shape()/register() render the canonical grammar.
//! Toy types live in a submodule so module_path! resolves to a known string.
use vita_artifact_derive::SchemaHash;
use vita_schema::{ShapeRegistry, SchemaShape};

mod sim_ir {
    use super::*;

    #[derive(SchemaHash)]
    pub struct ProcFlags(pub u8);

    #[derive(SchemaHash)]
    pub enum RegionTag { Active, Inactive, Nba, Monitor }

    #[derive(SchemaHash)]
    pub struct JoinState {
        pub parent: Option<u32>,
        pub children: Vec<u32>,
        pub detached: Vec<u32>,
        pub flags: sim_ir::ProcFlags,
    }
}

#[test]
fn procflags_newtype_shape() {
    assert_eq!(sim_ir::ProcFlags::schema_name(), "render::sim_ir::ProcFlags");
    assert_eq!(sim_ir::ProcFlags::local_shape(), "repr=@#[]newtype(#[]u8)");
}

#[test]
fn regiontag_enum_shape() {
    assert_eq!(
        sim_ir::RegionTag::local_shape(),
        "repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}"
    );
}

#[test]
fn joinstate_struct_shape_and_children() {
    assert_eq!(
        sim_ir::JoinState::local_shape(),
        "repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,\
         #[]detached:Vec<u32>,#[]flags:render::sim_ir::ProcFlags}"
            .replace(['\n', ' '], "") // tolerate test-source wrapping
            .as_str()
    );
    // register() interns ProcFlags exactly once.
    let mut reg = ShapeRegistry::new();
    sim_ir::JoinState::register(&mut reg);
    let s = reg.canonical_string();
    assert_eq!(s.matches("render::sim_ir::ProcFlags=").count(), 1);
}
```

> **Note on the golden strings:** here the test module path is `render::sim_ir`, so `schema_name()` is `render::sim_ir::ProcFlags`. In `sim-ir` (Task 5) the types live in the crate root with `extern crate self as sim_ir;`, so the *same* derive yields `sim_ir::ProcFlags` — matching doc 16. The derive logic is identical; only `module_path!()` differs by call site.

Run: `cargo test -p vita-artifact-derive --test render`
Expected: FAIL — derive does not exist yet.

- [ ] **Step 2: Implement the derive** `crates/vita-artifact-derive/src/lib.rs`

(Research-verified syn 2.0.x skeleton, with verdict fixes applied: 2.1 propagate `?`, 2.2 sort multi-`alias`.)

```rust
//! vita-artifact-derive — `#[derive(SchemaHash)]`
//! Emits `impl vita_schema::SchemaShape` for the annotated (concrete) type.
//! Verified against syn 2.0.x. MSRV 1.82, edition 2021.
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Error, Expr, Fields,
    GenericArgument, GenericParam, LitStr, Meta, PathArguments, Type, TypeArray, TypePath,
    TypeTuple, Variant,
};

#[proc_macro_derive(SchemaHash, attributes(serde))]
pub fn derive_schema_hash(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(ast: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    // (6) Hard guard: no generics of any kind (frozen schema types are monomorphic).
    if let Some(p) = ast.generics.params.iter().next() {
        let what = match p {
            GenericParam::Type(_) => "type",
            GenericParam::Lifetime(_) => "lifetime",
            GenericParam::Const(_) => "const-generic",
        };
        return Err(Error::new_spanned(
            p,
            format!("SchemaHash does not support {what} parameters; schema types must be concrete"),
        ));
    }

    let ident = &ast.ident;
    let ident_str = ident.to_string();
    let container_serde = render_serde_attrs(&ast.attrs)?;

    let mut children: Vec<String> = Vec::new();
    let shape_body = render_local_shape(ast, &mut children)?;
    // local_shape carries the structural body only; the registry prepends
    // schema_name() (module_path! is a runtime token, not bakeable here).
    let local_shape_str = format!("repr=@#[{container_serde}]{shape_body}");

    let child_paths: Vec<Type> = children
        .iter()
        .map(|s| syn::parse_str::<Type>(s))
        .collect::<Result<_, _>>()?;
    let register_calls = child_paths.iter().map(|ty| {
        quote! { <#ty as vita_schema::SchemaShape>::register(reg); }
    });

    Ok(quote! {
        impl vita_schema::SchemaShape for #ident {
            fn schema_name() -> &'static str {
                // module_path! emitted as a TOKEN — evaluated at the consumer's call site.
                ::core::concat!(::core::module_path!(), "::", #ident_str)
            }
            fn local_shape() -> &'static str { #local_shape_str }
            fn register(reg: &mut vita_schema::ShapeRegistry) {
                if !reg.insert_once(
                    <Self as vita_schema::SchemaShape>::schema_name(),
                    <Self as vita_schema::SchemaShape>::local_shape(),
                ) {
                    return;
                }
                #( #register_calls )*
            }
        }
    })
}

fn render_local_shape(ast: &DeriveInput, children: &mut Vec<String>) -> Result<String, Error> {
    match &ast.data {
        Data::Struct(s) => render_struct(s, children),
        Data::Enum(e) => render_enum(e, children),
        Data::Union(u) => Err(Error::new_spanned(u.union_token, "SchemaHash does not support unions")),
    }
}

fn render_struct(s: &DataStruct, children: &mut Vec<String>) -> Result<String, Error> {
    Ok(match &s.fields {
        Fields::Named(named) => {
            let mut parts = Vec::new();
            for f in &named.named {
                let attrs = render_serde_attrs(&f.attrs)?;
                let name = f.ident.as_ref().unwrap();
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{attrs}]{name}:{texpr}"));
            }
            format!("struct{{{}}}", parts.join(","))
        }
        Fields::Unnamed(unnamed) => {
            let mut parts = Vec::new();
            for f in &unnamed.unnamed {
                let attrs = render_serde_attrs(&f.attrs)?;
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{attrs}]{texpr}"));
            }
            if parts.len() == 1 {
                format!("newtype({})", parts[0])
            } else {
                format!("tuple({})", parts.join(","))
            }
        }
        Fields::Unit => "unit".to_string(),
    })
}

fn render_enum(e: &DataEnum, children: &mut Vec<String>) -> Result<String, Error> {
    let mut variants = Vec::new();
    for v in &e.variants {
        variants.push(render_variant(v, children)?);
    }
    Ok(format!("enum{{{}}}", variants.join(",")))
}

fn render_variant(v: &Variant, children: &mut Vec<String>) -> Result<String, Error> {
    let attrs = render_serde_attrs(&v.attrs)?;
    let name = v.ident.to_string();
    let disc = match &v.discriminant {
        Some((_eq, expr)) => format!("={}", render_disc_expr(expr)),
        None => String::new(),
    };
    let body = match &v.fields {
        Fields::Unit => String::new(),
        Fields::Named(named) => {
            let mut parts = Vec::new();
            for f in &named.named {
                let fattrs = render_serde_attrs(&f.attrs)?;
                let fname = f.ident.as_ref().unwrap();
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{fattrs}]{fname}:{texpr}"));
            }
            format!("{{{}}}", parts.join(","))
        }
        Fields::Unnamed(unnamed) => {
            let mut parts = Vec::new();
            for f in &unnamed.unnamed {
                let fattrs = render_serde_attrs(&f.attrs)?;
                let texpr = render_type_expr(&f.ty, children)?;
                parts.push(format!("#[{fattrs}]{texpr}"));
            }
            format!("({})", parts.join(","))
        }
    };
    Ok(format!("#[{attrs}]{name}{disc}{body}"))
}

fn render_disc_expr(expr: &Expr) -> String {
    quote!(#expr).to_string().split_whitespace().collect()
}

const PRIMITIVES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    "bool", "char", "str", "String", "f32", "f64",
];

fn render_type_expr(ty: &Type, children: &mut Vec<String>) -> Result<String, Error> {
    match ty {
        Type::Array(TypeArray { elem, len, .. }) => {
            let inner = render_type_expr(elem, children)?;
            let n: String = quote!(#len).to_string().split_whitespace().collect();
            Ok(format!("[{inner};{n}]"))
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            let mut inner = Vec::new();
            for e in elems {
                inner.push(render_type_expr(e, children)?);
            }
            Ok(format!("({})", inner.join(",")))
        }
        Type::Reference(r) => render_type_expr(&r.elem, children),
        Type::Path(tp) => render_path_type(tp, children),
        Type::Paren(p) => render_type_expr(&p.elem, children),
        Type::Group(g) => render_type_expr(&g.elem, children),
        other => Err(Error::new_spanned(other, "SchemaHash: unsupported field type construct")),
    }
}

fn render_path_type(tp: &TypePath, children: &mut Vec<String>) -> Result<String, Error> {
    if tp.qself.is_some() {
        return Err(Error::new_spanned(
            tp,
            "SchemaHash: qualified-path (<T as Trait>) field types are not supported",
        ));
    }
    let last = tp
        .path
        .segments
        .last()
        .ok_or_else(|| Error::new_spanned(tp, "SchemaHash: empty type path"))?;
    let head = last.ident.to_string();
    if head == "HashMap" || head == "HashSet" {
        return Err(Error::new_spanned(
            tp,
            format!("SchemaHash: `{head}` is forbidden (nondeterministic order); use BTreeMap/BTreeSet"),
        ));
    }
    let args: Vec<&Type> = match &last.arguments {
        PathArguments::AngleBracketed(ab) => ab
            .args
            .iter()
            .filter_map(|a| match a {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            })
            .collect(),
        PathArguments::None => Vec::new(),
        PathArguments::Parenthesized(p) => {
            return Err(Error::new_spanned(p, "SchemaHash: Fn-style type args not supported"))
        }
    };
    match (head.as_str(), args.len()) {
        ("Option", 1) => Ok(format!("Option<{}>", render_type_expr(args[0], children)?)),
        ("Vec", 1) => Ok(format!("Vec<{}>", render_type_expr(args[0], children)?)),
        ("BTreeSet", 1) => Ok(format!("BTreeSet<{}>", render_type_expr(args[0], children)?)),
        ("BTreeMap", 2) => Ok(format!(
            "BTreeMap<{},{}>",
            render_type_expr(args[0], children)?,
            render_type_expr(args[1], children)?
        )),
        (h, 0) if PRIMITIVES.contains(&h) => Ok(h.to_string()),
        _ => {
            let full = render_full_path(tp);
            if !children.iter().any(|c| c == &full) {
                children.push(full.clone()); // macro-time dedup, source order, plain Vec
            }
            Ok(full)
        }
    }
}

fn render_full_path(tp: &TypePath) -> String {
    tp.path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn render_serde_attrs(attrs: &[syn::Attribute]) -> Result<String, Error> {
    let mut slots: Vec<(usize, String)> = Vec::new();
    let order = |k: &str| -> usize {
        match k {
            "rename" => 0, "rename_all" => 1, "skip" => 2, "skip_serializing_if" => 3,
            "with" => 4, "default" => 5, "flatten" => 6, "tag" => 7, "content" => 8,
            "untagged" => 9, "transparent" => 10, "deny_unknown_fields" => 11,
            "alias" => 12, "other" => 13, _ => usize::MAX,
        }
    };
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        if let Meta::Path(_) = &attr.meta {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let key = meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
            let o = order(&key);
            match key.as_str() {
                "rename" | "rename_all" | "skip_serializing_if" | "with" | "tag" | "content"
                | "alias" => {
                    let lit: LitStr = meta.value()?.parse()?;
                    slots.push((o, format!("{key}={:?}", lit.value())));
                    Ok(())
                }
                "default" => {
                    if meta.input.peek(syn::Token![=]) {
                        let lit: LitStr = meta.value()?.parse()?;
                        slots.push((o, format!("default={:?}", lit.value())));
                    } else {
                        slots.push((o, "default".to_string()));
                    }
                    Ok(())
                }
                "skip" | "flatten" | "untagged" | "transparent" | "deny_unknown_fields"
                | "other" => {
                    slots.push((o, key));
                    Ok(())
                }
                _ => {
                    // verdict fix 2.1: propagate the Result instead of `let _ =`.
                    if meta.input.peek(syn::Token![=]) {
                        let _: Expr = meta.value()?.parse()?;
                    } else if meta.input.peek(syn::token::Paren) {
                        meta.parse_nested_meta(|_| Ok(()))?;
                    }
                    Ok(())
                }
            }
        })
        .map_err(|e| Error::new_spanned(attr, format!("SchemaHash: bad #[serde(..)]: {e}")))?;
    }
    // verdict fix 2.2: canonical order across keys AND within a repeated key (alias),
    // so source ordering of `alias="a",alias="b"` cannot change the output string.
    slots.sort();
    Ok(slots.into_iter().map(|(_, s)| s).collect::<Vec<_>>().join(","))
}
```

`crates/vita-artifact-derive/Cargo.toml`:

```toml
[package]
name = "vita-artifact-derive"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lib]
proc-macro = true

[dependencies]
syn = { workspace = true }
quote = { workspace = true }
proc-macro2 = { workspace = true }

[dev-dependencies]
vita-schema = { path = "../vita-schema" }   # tests need the SchemaShape trait + registry
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p vita-artifact-derive`
Expected: PASS — newtype/enum/struct render to the canonical grammar; `ProcFlags` interned once.

- [ ] **Step 4: Commit**

```bash
git add 016_claude_rtl/crates/vita-artifact-derive 016_claude_rtl/Cargo.lock
git commit -m "Add vita-artifact-derive: #[derive(SchemaHash)] proc-macro (syn 2, verdict fixes)"
```

---

## Task 5: `sim-ir` — frozen `SuspendState` closure (9 types + FourState/EdgeKind)

**Files:**
- Modify: `crates/sim-ir/Cargo.toml`
- Create: `crates/sim-ir/src/lib.rs`
- Test: `crates/sim-ir/tests/frozen_shapes.rs`

- [ ] **Step 1: Write the failing per-type shape test** `crates/sim-ir/tests/frozen_shapes.rs`

(Golden strings are doc 16 §"실제 frozen 타입 렌더" verbatim.)

```rust
use sim_ir::*;
use vita_schema::SchemaShape;

#[test]
fn procflags() {
    assert_eq!(ProcFlags::schema_name(), "sim_ir::ProcFlags");
    assert_eq!(ProcFlags::local_shape(), "repr=@#[]newtype(#[]u8)");
}
#[test]
fn region_tag() {
    assert_eq!(
        RegionTag::local_shape(),
        "repr=@#[]enum{#[]Active,#[]Inactive,#[]Nba,#[]Monitor}"
    );
}
#[test]
fn join_state() {
    assert_eq!(
        JoinState::local_shape(),
        "repr=@#[]struct{#[]parent:Option<u32>,#[]children:Vec<u32>,#[]detached:Vec<u32>,#[]flags:sim_ir::ProcFlags}"
    );
}
#[test]
fn wake_cond() {
    assert_eq!(
        WakeCond::local_shape(),
        "repr=@#[]enum{#[]Edge{#[]net:u32,#[]kind:sim_ir::EdgeKind},#[]Level{#[]nets:Vec<u32>},\
         #[]WaitTrue{#[]expr:u32},#[]TimeAbs{#[]tick:u64},#[]NamedEvent{#[]ev:u32},#[]Join{#[]join_ref:u32}}"
    );
}
#[test]
fn suspend_state() {
    assert_eq!(
        SuspendState::local_shape(),
        "repr=@#[]struct{#[]resume_pc:u32,#[]locals:Vec<sim_ir::FourState>,#[]join_state:sim_ir::JoinState,\
         #[]wake_key:sim_ir::WakeKey,#[]call_stack:Vec<sim_ir::Frame>,#[]frame_arena:Vec<sim_ir::FourState>}"
    );
}
```

Run: `cargo test -p sim-ir --test frozen_shapes`
Expected: FAIL — `sim_ir` types do not exist yet.

- [ ] **Step 2: Define the frozen closure** `crates/sim-ir/src/lib.rs`

> **Self-alias is load-bearing:** `extern crate self as sim_ir;` lets sibling fields be spelled `sim_ir::Foo`, so the derive's body reference equals each child's `schema_name()` (= `sim_ir::Foo`), reproducing doc 16's golden strings exactly.

```rust
//! sim-ir — language-neutral simulation IR.
//!
//! PR1-B defines ONLY the frozen `SuspendState` runtime-state closure
//! (06 process model · shapes FROZEN 2026-06-02). The unspecified types
//! `Process`/`BasicBlock`/`Stmt`/`Expr`/`Sensitivity`/`Terminator` are deferred
//! to M3 — they require the net/expr arena freeze before the *root* hash can lock.
//! `FourState` and `EdgeKind` are scalar leaf enums newly frozen here (the only
//! members of the unspecified set the SuspendState closure transitively touches).
extern crate self as sim_ir;

use serde::{Deserialize, Serialize};
use vita_artifact_derive::SchemaHash;

/// Scalar 4-state logic value (IEEE 1364 §6). NEWLY FROZEN in PR1-B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum FourState {
    Zero,
    One,
    X,
    Z,
}

/// Edge kind for an edge-sensitive wake condition. NEWLY FROZEN in PR1-B.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum EdgeKind {
    Posedge,
    Negedge,
    AnyEdge,
}

/// [SD1] vvp 1-bit flag bitset; newtype so the schema shape is distinct from a bare u8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct ProcFlags(pub u8);

/// [SD4] IEEE 1364 4 regions; 17-region split is an intentional Phase-2 flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum RegionTag {
    Active,
    Inactive,
    Nba,
    Monitor,
}

/// [SD5] closed 6-variant set of process-suspend conditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub enum WakeCond {
    Edge { net: u32, kind: sim_ir::EdgeKind },
    Level { nets: Vec<u32> },
    WaitTrue { expr: u32 },
    TimeAbs { tick: u64 },
    NamedEvent { ev: u32 },
    Join { join_ref: u32 },
}

/// [SD4] region stored explicitly (never re-derived → keeps logic out of the hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct WakeKey {
    pub cond: sim_ir::WakeCond,
    pub region: sim_ir::RegionTag,
    pub tie_break: u32,
}

/// [SD2] integer-indexed call frame (not a native call stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct Frame {
    pub return_pc: u32,
    pub callee_entry: u32,
    pub locals_base: u32,
    pub locals_len: u32,
    pub is_automatic: bool,
}

/// [SD1] vvp two-set fork/join port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct JoinState {
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub detached: Vec<u32>,
    pub flags: sim_ir::ProcFlags,
}

/// [resume/reserved] RULE D2 atomic-freeze unit (16 §1) — PR1-B golden root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SchemaHash)]
pub struct SuspendState {
    pub resume_pc: u32,
    pub locals: Vec<sim_ir::FourState>,
    pub join_state: sim_ir::JoinState,
    pub wake_key: sim_ir::WakeKey,
    pub call_stack: Vec<sim_ir::Frame>,
    pub frame_arena: Vec<sim_ir::FourState>,
}
```

`crates/sim-ir/Cargo.toml`:

```toml
[package]
name = "sim-ir"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
vita-schema = { path = "../vita-schema" }
vita-artifact-derive = { path = "../vita-artifact-derive" }

[dev-dependencies]
blake3 = { workspace = true }
hex = { workspace = true }
serde-reflection = { workspace = true }
```

- [ ] **Step 3: Run shape tests**

Run: `cargo test -p sim-ir --test frozen_shapes`
Expected: PASS — every frozen type's `local_shape()` equals its doc-16 golden literal, with FQ child references via the self-alias.

- [ ] **Step 4: Commit**

```bash
git add 016_claude_rtl/crates/sim-ir 016_claude_rtl/Cargo.lock
git commit -m "Add sim-ir frozen SuspendState closure (9 types + FourState/EdgeKind, SchemaHash derive)"
```

---

## Task 6: Golden gates — pinned hash, canonical golden, no-serde guard, RON Layer 3

**Files:**
- Create: `crates/sim-ir/tests/{schema_hash.rs,no_serde_attrs.rs,reflection.rs}`
- Create: `crates/testdata/{sim_ir_canonical.txt,sim_ir_registry.ron}`

- [ ] **Step 1: Compute the canonical string + hash once (scaffolding helper).** Add a temporary `#[test]` that prints, run it, capture output:

```rust
// crates/sim-ir/tests/schema_hash.rs  (initial form — prints, then we pin)
use vita_schema::{schema_hash, ShapeRegistry, SchemaShape};

#[test]
fn dump_canonical() {
    let mut reg = ShapeRegistry::new();
    sim_ir::SuspendState::register(&mut reg);
    println!("--CANON--\n{}--END--", reg.canonical_string());
    println!("HASH={}", hex::encode(schema_hash::<sim_ir::SuspendState>()));
}
```

Run: `cargo test -p sim-ir --test schema_hash dump_canonical -- --nocapture`
Action: copy the canonical block into `crates/testdata/sim_ir_canonical.txt` and the hash hex into `EXPECTED` below. Then replace the file with Step 2.

- [ ] **Step 2: Pin the golden** `crates/sim-ir/tests/schema_hash.rs`

```rust
//! Golden #1 (pinned hash, 2-platform determinism contract) + Golden #2 (canonical string diff).
use vita_schema::{schema_hash, ShapeRegistry, SchemaShape};

/// blake3 of the SuspendState-closure canonical string. Locked PR1-B.
/// Flips iff a frozen field/variant/serde-attr moves -> all .velab invalidated.
const EXPECTED_HASH: &str = "<64 hex chars from Step 1>";

const GOLDEN_CANON: &str = include_str!("../../testdata/sim_ir_canonical.txt");

#[test]
fn schema_hash_is_pinned() {
    let got = hex::encode(schema_hash::<sim_ir::SuspendState>());
    assert_eq!(
        got, EXPECTED_HASH,
        "SCHEMA_HASH changed — a frozen sim-ir shape/serde-attr moved.\n\
         If intentional: all .velab invalid -> bump format_version + update both goldens."
    );
}

#[test]
fn canonical_string_golden() {
    let mut reg = ShapeRegistry::new();
    sim_ir::SuspendState::register(&mut reg);
    assert_eq!(
        reg.canonical_string(),
        GOLDEN_CANON,
        "canonical shape string drifted — see the exact changed registry line above"
    );
}
```

Run: `cargo test -p sim-ir --test schema_hash`
Expected: PASS (hash + canonical match the just-captured goldens).

- [ ] **Step 3: Frozen-no-serde-attr guard** `crates/sim-ir/tests/no_serde_attrs.rs`

```rust
//! 16 §312: the frozen cluster must carry only `#[]` attr slots. If anyone adds a
//! serde attr to a frozen type, its local_shape() stops being all-`#[]` and the
//! hash flips — this guard makes that an explicit, named failure.
use vita_schema::SchemaShape;

fn assert_no_serde(name: &str, shape: &str) {
    // every attr slot in a frozen type must be the empty `#[]`.
    assert!(
        !shape.contains("#[serde") && !slot_nonempty(shape),
        "{name} carries a serde attr; frozen cluster must be attr-free: {shape}"
    );
}

/// true if any `#[...]` slot has content between the brackets.
fn slot_nonempty(shape: &str) -> bool {
    let bytes = shape.as_bytes();
    let mut i = 0;
    while let Some(p) = shape[i..].find("#[") {
        let open = i + p + 2;
        if bytes.get(open) != Some(&b']') {
            return true;
        }
        i = open + 1;
    }
    false
}

#[test]
fn frozen_types_are_attr_free() {
    assert_no_serde("ProcFlags", sim_ir::ProcFlags::local_shape());
    assert_no_serde("RegionTag", sim_ir::RegionTag::local_shape());
    assert_no_serde("EdgeKind", sim_ir::EdgeKind::local_shape());
    assert_no_serde("FourState", sim_ir::FourState::local_shape());
    assert_no_serde("Frame", sim_ir::Frame::local_shape());
    assert_no_serde("WakeCond", sim_ir::WakeCond::local_shape());
    assert_no_serde("WakeKey", sim_ir::WakeKey::local_shape());
    assert_no_serde("JoinState", sim_ir::JoinState::local_shape());
    assert_no_serde("SuspendState", sim_ir::SuspendState::local_shape());
}
```

Run: `cargo test -p sim-ir --test no_serde_attrs`
Expected: PASS.

- [ ] **Step 4: Layer-3 serde-reflection RON golden** `crates/sim-ir/tests/reflection.rs`

```rust
//! Layer 3 (16 §"Layer 3"): trace the *actual* serde derives with serde-reflection
//! and diff the resulting Registry (RON) against a committed golden. Catches wire
//! drift that the syn-attr Layer-1 path cannot see (e.g. `with=` internals).
use serde_reflection::{Samples, Tracer, TracerConfig};

fn traced_registry_ron() -> String {
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<sim_ir::SuspendState>(&samples).unwrap();
    let registry = tracer.registry().unwrap();
    ron::ser::to_string_pretty(&registry, ron::ser::PrettyConfig::default()).unwrap()
}

#[test]
fn serde_reflection_ron_golden() {
    let golden = include_str!("../../testdata/sim_ir_registry.ron");
    assert_eq!(
        traced_registry_ron().trim_end(),
        golden.trim_end(),
        "serde wire format drifted (Layer 3). Update sim_ir_registry.ron only if intentional."
    );
}
```

Action: on first run the golden file is absent — generate it:
`cargo test -p sim-ir --test reflection -- --nocapture` will fail; capture `traced_registry_ron()` by temporarily printing it, write to `crates/testdata/sim_ir_registry.ron`, then re-run.

> If `ron` is not already a dev-dep of sim-ir, add `ron = "0.8"` under `[dev-dependencies]`.

Run: `cargo test -p sim-ir --test reflection`
Expected: PASS.

- [ ] **Step 5: Full workspace gate**

Run: `cargo test --workspace --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all -- --check`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add 016_claude_rtl/crates/sim-ir 016_claude_rtl/crates/testdata 016_claude_rtl/Cargo.lock
git commit -m "Add sim-ir golden gates: pinned SuspendState hash + canonical + no-serde guard + serde-reflection RON"
```

---

## Task 7: Doc sync (03) + final verification

**Files:**
- Modify: `docs/preview/03-build-and-portability.md`

- [ ] **Step 1: Fix the edition + serde-reflection drift in doc 03.**
  - `edition = "2024"` → `edition = "2021"` (line ~57), with an inline note: `# edition 2024는 rustc>=1.85 — MSRV 1.82와 비호환이라 2021 고정`.
  - `# serde-reflection = "0.4"` → `# serde-reflection = "0.6"` (line ~83), note `# 0.4는 stale; 0.6 = edition 2021, MSRV 1.72`.
  - Add a one-line note near `blake3 = "1"` (line ~70): `# 구현 핀 =1.8.2 (1.8.3+는 edition 2024 → rustc>=1.85)`.

- [ ] **Step 2: Confirm no stale references.**

Run: `grep -rn 'edition = "2024"' 016_claude_rtl/docs/preview/03-build-and-portability.md`
Expected: no matches.

- [ ] **Step 3: Final full gate + commit**

```bash
cargo test --workspace --locked
git add 016_claude_rtl/docs/preview/03-build-and-portability.md
git commit -m "Update doc 03: edition 2021 (MSRV 1.82 compat) + serde-reflection 0.6 + blake3 pin note"
```

---

## Self-Review

**Spec coverage (PR1-B scope list):**
- Workspace skeleton / profile / toolchain / CI / Cargo.lock → Task 1 ✓
- diag real (Severity + MsgCode 36 + LogEvent/LogSink) + bijection → Task 2 ✓
- vita-schema real (trait + registry + schema_hash + OnceLock) → Task 3 ✓ (`cached_hash` provides the OnceLock path)
- vita-artifact-derive real → Task 4 ✓
- sim-ir partial (9 frozen + FourState/EdgeKind, serde+SchemaHash) → Task 5 ✓
- Golden gates (pinned hash / canonical / no-serde / Layer-3 RON) → Task 6 ✓
- Stubs (11 prod + cli + 2 dev) → Task 1 ✓
- Doc sync (edition, serde-reflection) → Task 7 ✓

**Type consistency:** `schema_name()`/`local_shape()`/`register()` signatures identical across the trait (Task 3), derive output (Task 4), and hand impls (Task 3 test). `ShapeRegistry::{new,insert_once,canonical_string}` and `schema_hash::<T>()` names match everywhere. `MsgCode::{ALL,mnemonic,code_num,default_severity,title}` consistent between Task 2 impl and the bijection test. Frozen field types spelled `sim_ir::Foo` consistently (Task 5) so derive output matches the golden literals (Task 5/6).

**Placeholder scan:** `EXPECTED_HASH` and the two `testdata/` goldens are *captured*, not invented (Task 6 Steps 1/4 generate them first, then pin) — this is the one unavoidable "fill from a real run" and is explicitly sequenced, not a hand-waved TODO.

**Known residual / non-blocking flags (carried from 16-spec):**
- Root `Process` hash is deferred to M3 (needs BasicBlock/Stmt/Expr/Sensitivity freeze). PR1-B locks only the SuspendState closure — stated up front.
- `hdl-ast` generic check (16 residual #2) is moot here: hdl-ast is a stub in PR1-B; the reject-all-generics rule only meets sim-ir's concrete types.
- Layer-1 ≠ full wire oracle (16 residual #3) — Layer-3 RON golden (Task 6 Step 4) is the backstop, as designed.
