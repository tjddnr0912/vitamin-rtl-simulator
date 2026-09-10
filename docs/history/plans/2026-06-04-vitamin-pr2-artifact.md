# vitamin PR2 — `vita-artifact` (container header + schema_hash gate) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use `- [ ]` checkboxes.

**Goal:** Turn the PR1-B determinism machinery from "computed but inert" into "actually gating" — build the `.velab` artifact **header** (magic + format_version + `schema_hash` stamp + provenance + staleness fields) with header-only decode and the three decode-time gates (`E-ART-FORMAT-MISMATCH`, `E-ART-SCHEMA-MISMATCH`, `E-ART-VERSION-GATE`).

**Architecture:** `vita-artifact` gains a real `VelabHeader` (serde + postcard) written as `MAGIC ++ postcard(header) ++ body`. Decode reads/checks the 8-byte magic, then `postcard::take_from_bytes` decodes the header alone (body untouched → no misparse, doc-15 E-ART-FORMAT-MISMATCH "헤더 전용 디코드"). A `ToolContext::current()` carries this build's `(format_version, schema_hash=vita_schema::schema_hash::<sim_ir::SuspendState>(), semver_major)`; `verify_header` compares and returns a `diag::MsgCode`-tagged `ArtifactError`. The full postcard **body** (SimIr root) and the RULE-V live source re-hash (`E-ART-STALE-UPSTREAM`) are **deferred to M3** — the `consumed`/`composite_input_hash`/`worklib_manifest_hash` fields round-trip in the header but their live-recheck gate is not built here.

**Tech Stack:** Rust 1.82 / edition 2021; `serde` (derive), `postcard 1.1` (`to_stdvec`/`take_from_bytes`), deps `sim-ir`/`vita-schema`/`diag`. No new external crates.

---

## Scope

**In scope:** `VelabHeader` + `Provenance` types (serde); `MAGIC_VELAB` + `CURRENT_FORMAT_VERSION` consts; `write_velab(header, body) -> Vec<u8>`; `read_velab(bytes) -> Result<(VelabHeader, &[u8]), ArtifactError>` (header-only decode, returns body slice); `ToolContext::current()`; `verify_header` (3 gates); `ArtifactError { code: MsgCode, message }`. Tests: round-trip, header-only decode preserves body bytes, each gate fires its code, happy path, schema-hash stamp-with-current passes / tampered fails.

**Out of scope (M3 / later):** the postcard **body** (SimIr root — needs the M3 Process/arena freeze); `E-ART-STALE-UPSTREAM` live re-hash (needs the source-digest/manifest subsystem); the `.vu` (hdl-ast) header; `vita-log` wiring.

**Root note:** the stamped/gated `schema_hash` uses `sim_ir::SuspendState` (the PR1-B golden root). M3 swaps this to `sim_ir::Process` — a one-line change in `ToolContext::current()` + a `format_version` bump.

---

## File Structure

```
crates/vita-artifact/
├── Cargo.toml                 # deps: serde, postcard, sim-ir, vita-schema, diag; dev: hex
├── src/lib.rs                 # re-exports
├── src/header.rs              # MAGIC_VELAB, CURRENT_FORMAT_VERSION, Provenance, VelabHeader, write/read
├── src/gate.rs                # ToolContext, ArtifactError, verify_header (3 codes)
├── tests/roundtrip.rs         # encode→decode, header-only decode preserves body
└── tests/gate.rs              # each gate fires correct MsgCode; happy path; stamp/tamper
```

---

## Task 1: Header types + encode/decode (header-only)

**Files:** Modify `crates/vita-artifact/Cargo.toml`; create `src/lib.rs`, `src/header.rs`; create `tests/roundtrip.rs`.

- [ ] **Step 1: Write the failing round-trip test** `crates/vita-artifact/tests/roundtrip.rs`

```rust
//! A written velab = MAGIC ++ postcard(header) ++ body; reading decodes the header
//! alone (postcard::take_from_bytes) and returns the untouched body slice.
use vita_artifact::{read_velab, write_velab, Provenance, VelabHeader, MAGIC_VELAB};

fn sample_header() -> VelabHeader {
    VelabHeader {
        format_version: 1,
        schema_hash: [0xAB; 32],
        composite_input_hash: [0x11; 32],
        global_time_precision: -12,
        consumed: vec![("work:top".to_string(), [0x22; 32])],
        worklib_manifest_hash: [0x33; 32],
        uses_dump: true,
        tool_semver_major: 0,
        provenance: Provenance {
            tool_version: "0.0.0".to_string(),
            git_sha: Some("deadbeef".to_string()),
            dirty: false,
            profile: "debug".to_string(),
        },
    }
}

#[test]
fn magic_prefixes_the_stream() {
    let bytes = write_velab(&sample_header(), b"BODYBYTES");
    assert_eq!(&bytes[..8], &MAGIC_VELAB);
}

#[test]
fn header_roundtrips_and_body_is_preserved() {
    let h = sample_header();
    let bytes = write_velab(&h, b"BODYBYTES");
    let (got, body) = read_velab(&bytes).expect("decode");
    assert_eq!(got, h);
    assert_eq!(body, b"BODYBYTES", "header-only decode must leave the body untouched");
}

#[test]
fn wrong_magic_is_format_mismatch() {
    let mut bytes = write_velab(&sample_header(), b"X");
    bytes[1] ^= 0xFF; // corrupt magic
    let err = read_velab(&bytes).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
}
```

Run: `cargo test -p vita-artifact --test roundtrip`
Expected: FAIL (crate is a stub).

- [ ] **Step 2: `crates/vita-artifact/src/header.rs`**

```rust
//! velab artifact header (doc-14 §1) — written/decoded independently of the body.
use serde::{Deserialize, Serialize};

use crate::gate::ArtifactError;

/// 8-byte magic prefix (doc-14 §1 "VELAB\0", padded to 8).
pub const MAGIC_VELAB: [u8; 8] = *b"VELAB\0\0\0";

/// Container format version. Bumped whenever the header layout changes
/// (e.g. when the M3 body fields or new gate fields are added).
pub const CURRENT_FORMAT_VERSION: u32 = 1;

/// Build provenance (Layer 2). Stamped for traceability, NEVER a staleness key
/// (doc-14 §2 RULE D2: a dirty tree must not force recompiles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub tool_version: String,
    pub git_sha: Option<String>,
    pub dirty: bool,
    pub profile: String,
}

impl Provenance {
    /// Capture from build-time env (no build.rs — option_env!/env!/cfg!).
    pub fn capture() -> Self {
        Provenance {
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha: option_env!("VITA_GIT_SHA").map(str::to_string),
            dirty: option_env!("VITA_GIT_DIRTY").is_some_and(|v| v == "1" || v == "true"),
            profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
        }
    }
}

/// velab header (doc-14 §1). Decodable before the body.
///
/// `composite_input_hash`/`consumed`/`worklib_manifest_hash` are stamped and
/// round-tripped here, but their RULE-V live-recheck gate (`E-ART-STALE-UPSTREAM`)
/// is deferred to a later PR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VelabHeader {
    pub format_version: u32,
    pub schema_hash: [u8; 32],
    pub composite_input_hash: [u8; 32],
    pub global_time_precision: i64,
    pub consumed: Vec<(String, [u8; 32])>,
    pub worklib_manifest_hash: [u8; 32],
    pub uses_dump: bool,
    pub tool_semver_major: u32,
    pub provenance: Provenance,
}

/// Serialize as `MAGIC_VELAB ++ postcard(header) ++ body`.
pub fn write_velab(header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 64 + body.len());
    out.extend_from_slice(&MAGIC_VELAB);
    let header_bytes = postcard::to_stdvec(header).expect("postcard header encode is infallible for owned data");
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(body);
    out
}

/// Check magic, then decode the header ALONE (body untouched). Returns the header
/// and the trailing body slice. A bad magic or undecodable header is a hard
/// `E-ART-FORMAT-MISMATCH` (doc-15) — the body is never deserialized, so a foreign
/// or truncated container can never misparse.
pub fn read_velab(bytes: &[u8]) -> Result<(VelabHeader, &[u8]), ArtifactError> {
    if bytes.len() < MAGIC_VELAB.len() || bytes[..MAGIC_VELAB.len()] != MAGIC_VELAB {
        return Err(ArtifactError::format("bad or missing VELAB magic"));
    }
    let after_magic = &bytes[MAGIC_VELAB.len()..];
    let (header, body) = postcard::take_from_bytes::<VelabHeader>(after_magic)
        .map_err(|e| ArtifactError::format(&format!("undecodable velab header: {e}")))?;
    Ok((header, body))
}
```

- [ ] **Step 3: `crates/vita-artifact/src/lib.rs`** (gate module stubbed in this task so it compiles; filled in Task 2)

```rust
//! vita-artifact — staged-artifact container: header (de)serialize, version/schema
//! staleness gates. (Body serialization + RULE-V live re-hash land in later PRs.)
mod gate;
mod header;

pub use gate::{verify_header, ArtifactError, ToolContext};
pub use header::{
    read_velab, write_velab, Provenance, VelabHeader, CURRENT_FORMAT_VERSION, MAGIC_VELAB,
};
```

- [ ] **Step 4: `crates/vita-artifact/Cargo.toml`**

```toml
[package]
name = "vita-artifact"
version = "0.0.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
postcard = { workspace = true }
sim-ir = { path = "../sim-ir" }
vita-schema = { path = "../vita-schema" }
diag = { path = "../diag" }

[dev-dependencies]
hex = { workspace = true }
```

- [ ] **Step 5:** Create a minimal `src/gate.rs` placeholder so Task 1 compiles (Task 2 replaces it):

```rust
use diag::MsgCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError {
    pub code: MsgCode,
    pub message: String,
}

impl ArtifactError {
    pub fn format(msg: &str) -> Self {
        ArtifactError { code: MsgCode::ArtFormatMismatch, message: msg.to_string() }
    }
}

pub struct ToolContext;
pub fn verify_header(_h: &crate::VelabHeader, _t: &ToolContext) -> Result<(), ArtifactError> {
    Ok(())
}
```

- [ ] **Step 6:** Run `cargo test -p vita-artifact --test roundtrip`. Expected: 3 PASS.

- [ ] **Step 7: Commit** (git rules in the Context section below).

```
Add vita-artifact velab header: magic + serde header, header-only decode (body preserved)
```

---

## Task 2: ToolContext + 3 decode-time gates

**Files:** Replace `crates/vita-artifact/src/gate.rs`; create `tests/gate.rs`.

- [ ] **Step 1: Write the failing gate test** `crates/vita-artifact/tests/gate.rs`

```rust
//! The three header gates fire their exact MsgCode; the happy path passes; a
//! schema_hash stamped with the current build verifies, a tampered one fails.
use diag::MsgCode;
use vita_artifact::{verify_header, Provenance, ToolContext, VelabHeader, CURRENT_FORMAT_VERSION};

fn header_for(tool: &ToolContext) -> VelabHeader {
    VelabHeader {
        format_version: tool.format_version,
        schema_hash: tool.schema_hash,
        composite_input_hash: [0; 32],
        global_time_precision: 0,
        consumed: vec![],
        worklib_manifest_hash: [0; 32],
        uses_dump: false,
        tool_semver_major: tool.semver_major,
        provenance: Provenance::capture(),
    }
}

#[test]
fn current_build_header_verifies() {
    let tool = ToolContext::current();
    assert!(verify_header(&header_for(&tool), &tool).is_ok());
}

#[test]
fn format_version_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.format_version = CURRENT_FORMAT_VERSION + 1;
    assert_eq!(verify_header(&h, &tool).unwrap_err().code, MsgCode::ArtFormatMismatch);
}

#[test]
fn schema_hash_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.schema_hash[0] ^= 0xFF; // tamper one byte
    assert_eq!(verify_header(&h, &tool).unwrap_err().code, MsgCode::ArtSchemaMismatch);
}

#[test]
fn semver_major_mismatch() {
    let tool = ToolContext::current();
    let mut h = header_for(&tool);
    h.tool_semver_major = tool.semver_major + 1;
    assert_eq!(verify_header(&h, &tool).unwrap_err().code, MsgCode::ArtVersionGate);
}

#[test]
fn stamped_schema_hash_is_the_sim_ir_root() {
    // The stamp equals vita_schema::schema_hash over the PR1-B golden root.
    let tool = ToolContext::current();
    let expected = vita_schema::schema_hash::<sim_ir::SuspendState>();
    assert_eq!(tool.schema_hash, expected);
}
```

Run: `cargo test -p vita-artifact --test gate`
Expected: FAIL (the placeholder gate always returns Ok / has no real ToolContext).

- [ ] **Step 2: Replace `crates/vita-artifact/src/gate.rs`**

```rust
//! Decode-time staleness gates (doc-14 §2 RULE D2, doc-15 9xxx).
//! Policy is version-GATE (refuse-and-rebuild), never silent migration.
use diag::MsgCode;

use crate::header::{VelabHeader, CURRENT_FORMAT_VERSION};

/// A gate rejection, tagged with the stable diagnostic code to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError {
    pub code: MsgCode,
    pub message: String,
}

impl ArtifactError {
    pub fn format(msg: &str) -> Self {
        ArtifactError { code: MsgCode::ArtFormatMismatch, message: msg.to_string() }
    }
    pub fn schema(msg: &str) -> Self {
        ArtifactError { code: MsgCode::ArtSchemaMismatch, message: msg.to_string() }
    }
    pub fn version(msg: &str) -> Self {
        ArtifactError { code: MsgCode::ArtVersionGate, message: msg.to_string() }
    }
}

/// This build's identity, compared against an artifact header.
pub struct ToolContext {
    pub format_version: u32,
    pub schema_hash: [u8; 32],
    pub semver_major: u32,
}

impl ToolContext {
    /// The running tool's expected values. `schema_hash` is the structural hash of
    /// the frozen sim-ir root (PR1-B: `SuspendState`; M3 swaps to `Process`).
    pub fn current() -> Self {
        ToolContext {
            format_version: CURRENT_FORMAT_VERSION,
            schema_hash: vita_schema::schema_hash::<sim_ir::SuspendState>(),
            semver_major: env!("CARGO_PKG_VERSION_MAJOR")
                .parse()
                .expect("CARGO_PKG_VERSION_MAJOR is a valid u32"),
        }
    }
}

/// Header gates, lower→higher: format (magic/version) is the lowest gate, then the
/// tool semver-major, then the structural schema hash. Any mismatch is a hard error
/// with a rebuild hint — never silent reuse (doc-15 9xxx, doc-14 §5).
pub fn verify_header(h: &VelabHeader, tool: &ToolContext) -> Result<(), ArtifactError> {
    if h.format_version != tool.format_version {
        return Err(ArtifactError::format(&format!(
            "format_version={} but this tool expects {}; regenerate with `velab`",
            h.format_version, tool.format_version
        )));
    }
    if h.tool_semver_major != tool.semver_major {
        return Err(ArtifactError::version(&format!(
            "produced by vitamin {}.x, this tool is {}.x; regenerate or install a matching vitamin",
            h.tool_semver_major, tool.semver_major
        )));
    }
    if h.schema_hash != tool.schema_hash {
        return Err(ArtifactError::schema(
            "sim-ir type shape changed between builds; rerun `velab`",
        ));
    }
    Ok(())
}
```

- [ ] **Step 3:** Run `cargo test -p vita-artifact`. Expected: all roundtrip + gate tests PASS.

- [ ] **Step 4: Commit.**

```
Add vita-artifact gates: ToolContext + verify_header (FORMAT/VERSION/SCHEMA codes)
```

---

## Task 3: Workspace gate + dep-graph doc note

**Files:** none new (verification + optional doc note).

- [ ] **Step 1: Full workspace gate.**

Run:
```
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```
All green. Fix any fmt/clippy issues in the files added this PR.

- [ ] **Step 2 (optional doc note):** In `docs/preview/14-staged-artifacts.md`, near the velab header block (§1), add a one-line note: `> **PR2 구현:** 헤더 (de)serialize + magic + schema_hash/format/version 게이트 구현(vita-artifact). 본문 postcard·RULE-V live re-hash는 M3.` Only if it does not disrupt the existing prose.

- [ ] **Step 3: Commit** (if Step 2 made a change).

```
Update doc 14: note PR2 vita-artifact header/gate implementation
```

---

## Context (for every implementer)

- **Working dir IS** `/Users/seongwookjang/project/git/violet_sw/016_claude_rtl`. The `vita-artifact` stub exists at `crates/vita-artifact/`.
- **Git repo root is the PARENT** `/Users/seongwookjang/project/git/violet_sw` (shared monorepo; siblings 005/006/007/014/GEMINI.md belong to other sessions and may be dirty). Branch is `vitamin-pr2-artifact`.
- **GIT SAFETY:** stage ONLY `016_claude_rtl/crates/vita-artifact`, `016_claude_rtl/docs/preview/14-...` (Task 3 only), and `016_claude_rtl/Cargo.lock`. NEVER `git add -A`/`.`. Use `git -C /Users/seongwookjang/project/git/violet_sw add 016_claude_rtl/crates/vita-artifact 016_claude_rtl/Cargo.lock` then commit; confirm `git diff --cached --name-only` shows only those prefixes.
- **Commit footer:** end every commit message with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- **postcard 1.x API:** `postcard::to_stdvec(&value) -> Result<Vec<u8>>` (needs `use-std`, enabled) and `postcard::take_from_bytes::<T>(bytes) -> Result<(T, &[u8])>` (decode a prefix, return remainder). If the exact signature differs in the locked version, adapt to produce: encode a header to bytes, and decode a header from the front of a buffer returning the trailing body slice. Do not pull new crates.

## Self-Review
- **Spec coverage:** header types+magic+provenance → T1; header-only decode preserves body → T1; 3 gates (FORMAT/SCHEMA/VERSION) with exact MsgCodes → T2; schema_hash stamped = sim_ir::SuspendState root → T2; workspace green → T3. STALE-UPSTREAM + body correctly deferred (stated).
- **Type consistency:** `VelabHeader` fields identical across header.rs and both test files; `ArtifactError { code, message }` and `ToolContext { format_version, schema_hash, semver_major }` consistent T1↔T2; `MsgCode::{ArtFormatMismatch,ArtSchemaMismatch,ArtVersionGate}` are the exact diag variant names from PR1-B.
- **No placeholders:** the Task-1 `gate.rs` placeholder is explicitly replaced in Task 2; it exists only so T1 compiles in isolation.
