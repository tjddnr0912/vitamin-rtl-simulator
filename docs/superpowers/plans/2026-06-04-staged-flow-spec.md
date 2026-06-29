# Staged-Flow Spec — `vcmp` / `velab` / `vrun` (`.vu` → `.velab` → simulate+VCD)

**Status:** implementation-ready. **Date:** 2026-06-04. **Owner:** artifact+cli architect.
**Scope:** wire the three staged-flow applets on top of the existing `vita-artifact` header
container, `hdl-ast::SourceUnit` (`.vu` golden) and `sim-ir::SimIr` (`.velab` golden), with
schema-hash staleness gates between every stage. **No frozen-IR change. No new MsgCode.**

This spec is grounded in verified source (line numbers checked 2026-06-04):

- `crates/vita-artifact/src/header.rs` — `write_velab`/`read_velab` (57/70), `VelabHeader` (43–54),
  `MAGIC_VELAB = *b"VELAB\0\0\0"` (7), `CURRENT_FORMAT_VERSION: u32 = 2` (10), `Provenance::capture` (23).
- `crates/vita-artifact/src/gate.rs` — `ArtifactError` (9) with `format`/`schema`/`version` ctors,
  `ToolContext` (36) with `current()` (45) hardcoding `schema_hash::<sim_ir::SimIr>()`, `verify_header` (59).
- `crates/vita-artifact/src/lib.rs` — re-exports (5–9).
- `crates/hdl-ast/src/lib.rs` — `SourceUnit { items: Vec<TopItem>, span: Span }` derives
  `Serialize, Deserialize, SchemaHash` (82). **The module doc-comment (8–24) claiming "do NOT derive
  SchemaHash" is STALE** — ignore it; trust the derive + the live golden. **The implementing PR MUST also
  scrub/correct that stale doc-comment** (delete or fix `hdl-ast/src/lib.rs:9-18`) so the source text and
  the actual `SchemaHash` derive agree — otherwise a future reader "fixes" the spec back to the wrong
  assumption. Tracked as DEFERRED item 8.
- `crates/hdl-ast/tests/schema_hash.rs` — `schema_hash::<SourceUnit>()` pinned to
  `EXPECTED = [9,20,170,115,…,90]` (live).
- `crates/sim-ir/src/lib.rs` — `SimIr` (435) derives serde+SchemaHash, **no `Box`** (arena-flattened).
- `crates/sim-ir/tests/schema_hash.rs` — `schema_hash::<SimIr>()` pinned hex
  `7b46c1706bc026725c1812db7045df8770136fa5ac85d0e2c8bb44d41071bcd4`.
- `crates/elaborate/src/lib.rs` — `JoinMode` (80) derives `serde::{Serialize,Deserialize}` (NOT
  SchemaHash, intentional); `ForkModeTable = BTreeMap<(u32,u32),JoinMode>` (94, plain alias);
  `elaborate_with_modes(unit,&sink) -> (Option<SimIr>, ForkModeTable)` (99).
- `crates/sim-engine/src/lib.rs` — `SimOpts.fork_modes: ForkModeTable` (77), `Default` sets it empty (88),
  `simulate(ir,&sink,opts) -> SimResult` (126), `simulate_capture(ir,opts) -> (SimResult,String)` (175),
  re-exports `ForkModeTable, JoinMode` (41).
- `crates/cli/src/lib.rs` — `run_vita_str` staged blueprint (178–261), `StderrSink` (55),
  `emit_frontend_error`/`loc_from_span` (152/157), `sim_exit_code` (266), `run_vita` file loop (295),
  `resolve_applet` (340) already returns `Applet::Staged("vcmp"|"velab"|"vrun")`, `run` (372) stub arm
  (376–381) → `EXIT_CLI_ERROR`. `EXIT_OK=0`, `EXIT_USER_ERROR=1`, `EXIT_CLI_ERROR=3`.
- `crates/diag/src/code.rs` — `ArtFormatMismatch=VITA-E9001`, `ArtSchemaMismatch=E9002`,
  `ArtStaleUpstream=E9003` (gate deferred), `ArtVersionGate=E9004` (81–84). `code_num()` (19),
  `mnemonic()` (15), `default_severity()` (23) all `const fn`.

---

## 0. Design decisions (resolved)

| Question | Decision | Why |
|---|---|---|
| Separate `VuHeader` struct? | **NO.** Reuse `VelabHeader` verbatim for `.vu`, with a distinct magic `MAGIC_VU`. | The header fields (`format_version`/`schema_hash`/`provenance`/…) are language-neutral; only the magic and the *value* of `schema_hash` differ. A second struct duplicates the wire format and the gate logic. `verify_header` is already magic-agnostic. |
| May `vita-artifact` depend on `hdl-ast`? | **NO.** Keep the container language-neutral. The CLI computes the per-stage expected hash and passes it into a new `ToolContext::new(hash)` constructor. | `vita-artifact` today depends only on `sim-ir`+`vita-schema`+`diag`. Adding `hdl-ast` couples the container to the front-end AST. The `cli` already depends on both `hdl-ast` and `sim-ir`. |
| `.vu` vs `.velab` format_version? | **Share `CURRENT_FORMAT_VERSION = 2`.** The `schema_hash` divergence already distinguishes the two bodies; the magic distinguishes the files. | One header layout = one version const. A header *layout* change bumps it for both. |
| Where does `ForkModeTable` ride in `.velab`? | **A non-golden postcard trailer appended AFTER the `SimIr` frame in the body.** `body = postcard(SimIr) ++ postcard(ForkModeTable)`. | `ForkModeTable` deliberately stays OUT of the `SimIr` golden closure (the frozen `Terminator::Fork` has no mode field). The `schema_hash` gate covers only the `SimIr` frame; the trailer rides behind that gate, split out with `take_from_bytes`. |
| New MsgCode for `.vu`? | **NO.** `ArtFormatMismatch`/`ArtSchemaMismatch`/`ArtVersionGate` are body-agnostic and reused as-is. | The codes describe the *gate kind*, not the artifact kind. |
| Frozen-IR change? | **NONE.** No edit to any `sim-ir` type. `JoinMode`/`ForkModeTable` already serde-derive (verified). | The `.velab` golden stays at the pinned hash. |

---

## 1. The `.vu` container

### 1.1 `vita-artifact` additions (`crates/vita-artifact/src/header.rs`)

Add the VU magic and generic-over-magic internals; expose `write_vu`/`read_vu` as thin wrappers that
reuse the **identical** `VelabHeader` struct and `MAGIC ++ postcard(header) ++ body` layout.

```rust
/// 8-byte magic prefix for the compiled `.vu` artifact (parse output).
/// Distinct from MAGIC_VELAB so a `.vu` fed to vrun fails the FORMAT gate, not the schema gate.
pub const MAGIC_VU: [u8; 8] = *b"VU\0\0\0\0\0\0";

/// Internal: serialize as `magic ++ postcard(header) ++ body`. The body is opaque.
fn write_with_magic(magic: &[u8; 8], header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 64 + body.len());
    out.extend_from_slice(magic);
    let header_bytes =
        postcard::to_stdvec(header).expect("postcard header encode is infallible for owned data");
    out.extend_from_slice(&header_bytes);
    out.extend_from_slice(body);
    out
}

/// Internal: check `magic`, decode the header ALONE, return the untouched body slice.
fn read_with_magic<'a>(
    magic: &[u8; 8],
    label: &str,
    bytes: &'a [u8],
) -> Result<(VelabHeader, &'a [u8]), ArtifactError> {
    if bytes.len() < magic.len() || bytes[..magic.len()] != *magic {
        return Err(ArtifactError::format(&format!("bad or missing {label} magic")));
    }
    let after_magic = &bytes[magic.len()..];
    let (header, body) = postcard::take_from_bytes::<VelabHeader>(after_magic)
        .map_err(|e| ArtifactError::format(&format!("undecodable {label} header: {e}")))?;
    Ok((header, body))
}

/// Serialize a `.vu` (compiled SourceUnit) artifact: `MAGIC_VU ++ postcard(header) ++ body`.
/// `body` is `postcard(hdl_ast::SourceUnit)` (the CLI owns that encode — the container is neutral).
pub fn write_vu(header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    write_with_magic(&MAGIC_VU, header, body)
}

/// Header-only decode of a `.vu`. Bad magic / undecodable header → E-ART-FORMAT-MISMATCH.
pub fn read_vu(bytes: &[u8]) -> Result<(VelabHeader, &[u8]), ArtifactError> {
    read_with_magic(&MAGIC_VU, "VU", bytes)
}
```

**Refactor the existing functions onto the shared internals (behavior-identical, keeps the public
signature and the existing `tests/roundtrip.rs` green):**

```rust
pub fn write_velab(header: &VelabHeader, body: &[u8]) -> Vec<u8> {
    write_with_magic(&MAGIC_VELAB, header, body)
}
pub fn read_velab(bytes: &[u8]) -> Result<(VelabHeader, &[u8]), ArtifactError> {
    read_with_magic(&MAGIC_VELAB, "velab", bytes)
}
```

> **Label casing — match the live lowercase.** The refactored `read_velab` passes `label = "velab"`
> (lowercase) so the messages stay **byte-identical to today**: `"undecodable velab header: …"`
> (verified `header.rs:76`, lowercase) and `"bad or missing velab magic"`. Note the live magic-mismatch
> string is `"bad or missing VELAB magic"` (uppercase `VELAB`, `header.rs:72`) while the generic
> `read_with_magic` interpolates `{label}` → `"bad or missing velab magic"` (lowercase). This is a
> **cosmetic casing change on the magic-mismatch path only**; no test asserts the string
> (`roundtrip.rs:43` `wrong_magic_is_format_mismatch` asserts only the `MsgCode`), so nothing breaks.
> If exact legacy parity on BOTH strings is required, special-case the VELAB magic-mismatch message to
> uppercase; otherwise accept the lowercase normalization. The `.vu` path uses `label = "VU"` (its
> strings are new, no legacy to match).

### 1.2 `ToolContext::new` — language-neutral constructor (`gate.rs`)

Keep `current()` (SimIr flavor) for the `.velab` gate; add a generic constructor the CLI uses for `.vu`.

```rust
impl ToolContext {
    /// Generic: build a tool context for ANY artifact body, with the caller-supplied
    /// expected structural hash (e.g. schema_hash::<SourceUnit>() for `.vu`).
    /// format_version + semver_major are fixed for this build.
    pub fn new(schema_hash: [u8; 32]) -> Self {
        ToolContext {
            format_version: CURRENT_FORMAT_VERSION,
            schema_hash,
            semver_major: env!("CARGO_PKG_VERSION_MAJOR")
                .parse()
                .expect("CARGO_PKG_VERSION_MAJOR is a valid u32"),
        }
    }

    /// The SimIr-flavored convenience (`.velab` gate). Unchanged.
    pub fn current() -> Self {
        Self::new(vita_schema::schema_hash::<sim_ir::SimIr>())
    }
}
```

`verify_header` is **unchanged** — it already gates `format_version` → `tool_semver_major` →
`schema_hash` purely from the decoded header vs the passed `ToolContext`, with no knowledge of the magic.
It works for `.vu` once `ToolContext::new(schema_hash::<SourceUnit>())` is passed.

### 1.3 `vita-artifact` lib re-exports (`crates/vita-artifact/src/lib.rs`)

```rust
pub use gate::{verify_header, ArtifactError, ToolContext};
pub use header::{
    read_velab, read_vu, write_velab, write_vu,
    Provenance, VelabHeader, CURRENT_FORMAT_VERSION, MAGIC_VELAB, MAGIC_VU,
};
```

### 1.4 Header field population for `.vu`

The CLI fills a `VelabHeader` for the `.vu` like this (RULE-V fields stamped but not yet gated — see §6):

```rust
fn vu_header(schema_hash: [u8; 32]) -> vita_artifact::VelabHeader {
    vita_artifact::VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash,                       // schema_hash::<SourceUnit>()
        composite_input_hash: [0u8; 32],   // RULE-V deferred (stamped zero)
        global_time_precision: 0,          // not meaningful pre-elaborate
        consumed: Vec::new(),              // RULE-V deferred
        worklib_manifest_hash: [0u8; 32],  // RULE-V deferred
        uses_dump: false,                  // unknown until elaborate; false for `.vu`
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        provenance: vita_artifact::Provenance::capture(),
    }
}
```

---

## 2. The `.velab` container extension (SimIr golden + non-golden fork trailer)

### 2.1 Body layout

```
.velab file  = MAGIC_VELAB ++ postcard(VelabHeader) ++ BODY
BODY         = postcard(sim_ir::SimIr)  ++  postcard(ForkModeTable)
                └── golden, schema_hash-gated ──┘  └── non-golden trailer ──┘
```

- `header.schema_hash = schema_hash::<SimIr>()` — gated by `verify_header` BEFORE the body is decoded.
- The `SimIr` frame is the golden payload (its shape is frozen / pinned).
- The `ForkModeTable` trailer is `BTreeMap<(u32,u32),JoinMode>` — span-free, fixed-width, sorted-key →
  3-OS byte-deterministic. An empty table encodes to one byte (postcard varint `0`), so the trailer is
  always present and cheap.

`take_from_bytes::<SimIr>(body)` consumes EXACTLY the SimIr frame and returns the remainder = the
trailer. No length prefix / offset field is needed in the header (self-delimiting, same primitive
`read_velab` uses for the header).

### 2.2 Encode (velab emit) — CLI side

```rust
let mut body = postcard::to_stdvec(&ir).expect("SimIr postcard encode infallible");
let trailer = postcard::to_stdvec(&fork_modes).expect("ForkModeTable postcard encode infallible");
body.extend_from_slice(&trailer);
let header = velab_header(schema_hash::<sim_ir::SimIr>(), &ir /* uses_dump probe, §3.2 */);
let bytes = vita_artifact::write_velab(&header, &body);
std::fs::write(out, &bytes)?;   // out-path error → EXIT_CLI_ERROR
```

### 2.3 Decode (vrun) — CLI side

```rust
let bytes = std::fs::read(velab_path)?;                       // read error → EXIT_CLI_ERROR
let (header, body) = vita_artifact::read_velab(&bytes)        // bad magic/header → E-ART-FORMAT-MISMATCH
    .map_err(emit_and_user_error)?;
let tool = vita_artifact::ToolContext::current();             // SimIr-flavored
vita_artifact::verify_header(&header, &tool)                  // schema/version gate
    .map_err(emit_and_user_error)?;
let (ir, rest): (sim_ir::SimIr, &[u8]) =
    postcard::take_from_bytes(body).map_err(body_decode_err)?;     // E-ART-FORMAT-MISMATCH
let fork_modes: sim_engine::ForkModeTable =                   // re-exported from elaborate (cli already deps sim-engine)
    postcard::from_bytes(rest).map_err(body_decode_err)?;          // E-ART-FORMAT-MISMATCH
```

> Note: the `take_from_bytes`/`from_bytes` body-decode failures are reported as
> `ArtFormatMismatch` (a structurally corrupt body behind a valid-shape header is a format problem,
> not a schema-hash divergence — the hash already matched). See §4.

### 2.4 serde derives on the fork types — **already present, no change**

Verified: `JoinMode` derives `serde::{Serialize,Deserialize}` (`elaborate/src/lib.rs:80`);
`ForkModeTable` is a `BTreeMap` alias and inherits serde transitively. **No derive needs to be added.**
`JoinMode` deliberately does NOT derive `SchemaHash` and the alias cannot carry it — correct, because
the trailer must never enter the `SimIr` golden closure. (If future work wants the trailer staleness-
protected, wrap it as `struct ForkModeTrailer(ForkModeTable)` deriving `SchemaHash` and fold its hash
into the header — a design choice, explicitly DEFERRED, see §6.)

---

## 3. The three CLI applets (concrete functions)

All three live in `crates/cli/src/lib.rs`, mirror `run_vita`'s contract (slice/opts in, doc-13 `i32`
out, never `process::exit`), and reuse `StderrSink`, `emit_frontend_error`, `sim_exit_code`,
`VitaOpts::sim_opts()`, and the prefix/suffix stages of `run_vita_str`.

### 3.0 New `cli` dependencies (`crates/cli/Cargo.toml`)

```toml
vita-artifact = { path = "../vita-artifact" }
vita-schema   = { path = "../vita-schema" }
postcard      = { workspace = true }
```

(`hdl-ast`, `sim-ir`, `sim-engine`, `elaborate`, `diag` already present.)

### 3.1 Shared helper: emit an `ArtifactError` and map to exit code

```rust
/// Render an artifact-gate rejection through the sink as an Error diagnostic
/// (no source location — artifact-level), then return EXIT_USER_ERROR.
/// Gate rejections are design/data errors (doc-13: code 1), NOT CLI usage errors.
fn emit_artifact_error(sink: &StderrSink, e: &vita_artifact::ArtifactError) -> i32 {
    sink.emit(LogEvent::Diagnostic(Diagnostic {
        severity: Severity::Error,
        code: e.code,                 // ArtFormatMismatch / ArtSchemaMismatch / ArtVersionGate
        message: e.message.clone(),
        location: None,
        context: Vec::new(),
        sim_time: None,
    }));
    EXIT_USER_ERROR
}

/// Read a file as bytes; a read failure is a CLI/usage error (exit 3).
fn read_artifact_bytes(path: &str) -> Result<Vec<u8>, i32> {
    std::fs::read(path).map_err(|e| {
        eprintln!("error[{}]: cannot read '{path}': {e}", MsgCode::FlistNotFound.code_num());
        EXIT_CLI_ERROR
    })
}

/// Default output path: replace **only the final** extension component on the input
/// (std `Path::with_extension` semantics — never panics, replaces the last `.ext` only).
/// e.g. default_out("a.sv","vu") -> "a.vu"; default_out("a.vu","velab") -> "a.velab";
///      default_out("a.b.sv","vu") -> "a.b.vu" (only the trailing `.sv` is swapped).
/// Contract: a bare name without a directory yields `name.ext` (e.g. "top" -> "top.vu").
/// An empty / dot-leading / directory-like input is *user error* — it is not special-cased
/// here; it surfaces later as a read error (file won't exist) or, for vcmp, the empty-source
/// guard in `dispatch_vcmp`. Callers MUST run `out` through `same_path` before writing (an
/// input whose extension already equals `ext` would otherwise default to clobbering itself).
fn default_out(input: &str, ext: &str) -> String {
    let p = std::path::Path::new(input);
    p.with_extension(ext).to_string_lossy().into_owned()
}

/// True iff two path strings denote the same file. Robust form compares
/// `std::fs::canonicalize` when BOTH paths already exist (handles `./a.sv` vs `a.sv`,
/// symlinks, `..`); otherwise falls back to a raw string compare (the output usually
/// does not exist yet, so canonicalize would fail — string equality still catches the
/// common `-o a.sv` / `vcmp a.vu` self-clobber). Never panics.
fn same_path(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Reject when the resolved output path would overwrite any positional input.
/// Guards both the `default_out` self-clobber (`vcmp foo.vu` -> default `foo.vu`) and an
/// explicit `-o a.sv` that names an input. Returns `Err(EXIT_CLI_ERROR)` after emitting.
fn reject_out_clobbers_input(inputs: &[String], out: &str) -> Result<(), i32> {
    if inputs.iter().any(|p| same_path(p, out)) {
        eprintln!(
            "error[{}]: output '{out}' would overwrite an input file",
            MsgCode::CliBadFlag.code_num()
        );
        return Err(EXIT_CLI_ERROR);
    }
    Ok(())
}
```

### 3.2 `run_vcmp` — source(s) → `.vu`

Reuses the preprocess→lex→parse prefix of `run_vita_str` verbatim, then stops at the `SourceUnit` and
writes the `.vu`. **This prefix is refactored into a single shared `frontend_to_unit` helper** so the
one-shot path, the staged `vcmp` path, AND the round-trip tests all parse through *byte-identically the
same* preprocess→lex→parse pipeline (no test may bypass the preprocessor — see TEST 3/5). It is `pub`
(or `pub(crate)` + a `#[doc(hidden)]` `pub` test re-export) so `cli/tests/staged_flow.rs` can build a
reference `SourceUnit` the same way production does.

```rust
/// Read a single source file, then run the preprocess→lex→parse front-end, emitting any
/// diagnostics through `sink`. Returns `Some(unit)` on a clean parse, `None` if read /
/// preprocess / lex / parse failed OR the parse produced no design units (the caller maps
/// `None` to EXIT_USER_ERROR; the read-failure case is handled by the caller's read loop for
/// multi-file `vcmp`, so this helper's single-file read failure also returns `None` after
/// emitting). The full pipeline (incl. the preprocessor) runs even for directive-free input,
/// so byte offsets / spans match the production path exactly.
pub fn frontend_to_unit(file: &str, sink: &StderrSink) -> Option<hdl_ast::SourceUnit> {
    let text = std::fs::read_to_string(file).ok()?;
    let text = if text.ends_with('\n') { text } else { format!("{text}\n") };
    frontend_text_to_unit(file, &text, sink)
}

/// The preprocess→lex→parse core, factored so multi-file `vcmp` (which concatenates first)
/// and single-file `frontend_to_unit` share one implementation. Returns `None` (after emitting)
/// on any front-end error or an empty unit.
pub fn frontend_text_to_unit(file: &str, text: &str, sink: &StderrSink) -> Option<hdl_ast::SourceUnit> {
    // preprocess → lex → parse, identical to run_vita_str 181–242 (body below in run_vcmp).
    // … (the preprocess/lex/parse block shown inline in run_vcmp moves here verbatim) …
    unimplemented!("see the inline block in run_vcmp; this is the extracted shared core")
}
```

`run_vcmp` calls `frontend_text_to_unit` on the concatenated multi-file text (so a stray `-o`-less
multi-source `vcmp a.sv b.sv` still works); the body below is shown inline for clarity and is the
verbatim source of `frontend_text_to_unit`.

```rust
/// `vcmp`: read+preprocess+lex+parse the source(s) into a `SourceUnit`, then write
/// a `.vu` artifact. `out` is the `-o` path (or `default_out(sources[0],"vu")`).
/// Exit: 0 ok / 1 lex|parse|empty-unit / 3 missing-file|write-error.
pub fn run_vcmp(sources: &[String], out: &str, opts: &VitaOpts) -> i32 {
    let _ = opts; // vcmp ignores sim knobs; kept for signature symmetry
    if sources.is_empty() {
        eprintln!("error[{}]: no source files given", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    let sink = StderrSink::new();

    // read+concat (mirrors run_vita 303–322): read error → exit 3.
    let mut text = String::new();
    for path in sources {
        match std::fs::read_to_string(path) {
            Ok(s) => { text.push_str(&s); if !s.ends_with('\n') { text.push('\n'); } }
            Err(e) => {
                eprintln!("error[{}]: cannot read '{path}': {e}", MsgCode::FlistNotFound.code_num());
                return EXIT_CLI_ERROR;
            }
        }
    }
    let file = sources[0].as_str();

    // ── preprocess (run_vita_str 181–204) ──
    let base_dir = std::path::Path::new(file).parent().unwrap_or_else(|| std::path::Path::new("."));
    let pp = hdl_preprocess::preprocess_str(base_dir, file, &text, &hdl_preprocess::PreOpts::default());
    for d in &pp.diags {
        let loc = pp.map.resolve_span(d.at, d.at);
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: d.severity, code: d.code, message: d.message.clone(),
            location: Some(loc), context: Vec::new(), sim_time: None,
        }));
    }
    if pp.has_errors() { return EXIT_USER_ERROR; }
    let expanded: &str = &pp.text;

    // ── lex (206–215) ──
    let (tokens, lex_errors) = hdl_lexer::lex(expanded);
    if !lex_errors.is_empty() {
        for e in &lex_errors {
            let (mnemonic, _) = e.kind.msg_code_hint();
            let msg = format!("lex error: {} ({mnemonic})", lex_error_message(e.kind));
            emit_frontend_error(&sink, &pp.map, e.span.start, e.span.end, msg);
        }
        return EXIT_USER_ERROR;
    }

    // ── parse (217–242) ──
    let (unit, parse_errors) = hdl_parser::parse(&tokens, expanded);
    if !parse_errors.is_empty() {
        for e in &parse_errors {
            let found = match e.found { Some(k) => format!("{k:?}"), None => "end of file".into() };
            emit_frontend_error(&sink, &pp.map, e.span.lo as usize, e.span.hi as usize,
                format!("expected {}, found {found}", e.expected));
        }
        return EXIT_USER_ERROR;
    }
    let Some(unit) = unit else {
        sink.emit(LogEvent::Diagnostic(Diagnostic {
            severity: Severity::Error, code: MsgCode::ParseUnexpectedToken,
            message: "no design units found in source".into(),
            location: None, context: Vec::new(), sim_time: None,
        }));
        return EXIT_USER_ERROR;
    };

    // ── write `.vu` ──
    let body = postcard::to_stdvec(&unit).expect("SourceUnit postcard encode infallible");
    let header = vu_header(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    let bytes = vita_artifact::write_vu(&header, &body);
    if let Err(e) = std::fs::write(out, &bytes) {
        eprintln!("error[{}]: cannot write '{out}': {e}", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    EXIT_OK
}
```

### 3.3 `run_velab` — `.vu` → `.velab`

```rust
/// `velab`: read a `.vu`, gate the hdl-ast hash, decode the `SourceUnit`, elaborate
/// (with fork modes), then write a `.velab` = header(SimIr hash) + body(SimIr ++ ForkModeTable).
/// Exit: 0 ok / 1 gate-reject|elab-fail|corrupt-body / 3 missing-file|write-error.
pub fn run_velab(vu_path: &str, out: &str, opts: &VitaOpts) -> i32 {
    let _ = opts;
    let sink = StderrSink::new();

    let bytes = match read_artifact_bytes(vu_path) { Ok(b) => b, Err(code) => return code };

    // header-only decode (bad magic/header → E-ART-FORMAT-MISMATCH)
    let (header, body) = match vita_artifact::read_vu(&bytes) {
        Ok(x) => x,
        Err(e) => return emit_artifact_error(&sink, &e),
    };
    // staleness gate: this `.vu` must match the hdl-ast shape THIS velab was built against.
    let tool = vita_artifact::ToolContext::new(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    if let Err(e) = vita_artifact::verify_header(&header, &tool) {
        return emit_artifact_error(&sink, &e);                  // E-ART-SCHEMA-MISMATCH etc.
    }
    // decode the SourceUnit body (corrupt body behind a valid header → E-ART-FORMAT-MISMATCH)
    let unit: hdl_ast::SourceUnit = match postcard::from_bytes(body) {
        Ok(u) => u,
        Err(e) => return emit_artifact_error(&sink,
            &vita_artifact::ArtifactError::format(&format!("undecodable .vu body: {e}"))),
    };

    // ── elaborate (run_vita_str 244–252) ──
    let (ir, fork_modes) = elaborate::elaborate_with_modes(&unit, &sink);
    let Some(ir) = ir else { return EXIT_USER_ERROR; };         // elab error already emitted

    // ── write `.velab` (body = postcard(SimIr) ++ postcard(ForkModeTable)) ──
    let mut velab_body = postcard::to_stdvec(&ir).expect("SimIr postcard encode infallible");
    let trailer = postcard::to_stdvec(&fork_modes).expect("ForkModeTable postcard encode infallible");
    velab_body.extend_from_slice(&trailer);
    let vheader = velab_header(vita_schema::schema_hash::<sim_ir::SimIr>(), &ir);
    let out_bytes = vita_artifact::write_velab(&vheader, &velab_body);
    if let Err(e) = std::fs::write(out, &out_bytes) {
        eprintln!("error[{}]: cannot write '{out}': {e}", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    EXIT_OK
}

/// Build the `.velab` header. `uses_dump` is probed from the IR (does any process
/// reference $dumpvars/$dumpfile?); v1 conservatively stamps `false` — it is a hint,
/// not a gate. `global_time_precision` is 0 until the timescale pass threads it (DEFERRED).
fn velab_header(schema_hash: [u8; 32], _ir: &sim_ir::SimIr) -> vita_artifact::VelabHeader {
    vita_artifact::VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash,
        composite_input_hash: [0u8; 32],   // RULE-V deferred
        global_time_precision: 0,          // timescale pass deferred
        consumed: Vec::new(),              // RULE-V deferred (would list the `.vu` hash)
        worklib_manifest_hash: [0u8; 32],  // RULE-V deferred
        uses_dump: false,                  // v1 hint
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        provenance: vita_artifact::Provenance::capture(),
    }
}
```

### 3.4 `run_vrun` — `.velab` → simulate + VCD

```rust
/// `vrun`: read a `.velab`, gate the SimIr hash, decode SimIr+ForkModeTable, simulate
/// (threading fork_modes into SimOpts), writing the VCD. Returns the doc-13 sim exit code.
/// Exit: 0 clean / 1 gate-reject|corrupt-body|runtime-fatal / 3 missing-file.
pub fn run_vrun(velab_path: &str, opts: &VitaOpts) -> i32 {
    let sink = StderrSink::new();

    let bytes = match read_artifact_bytes(velab_path) { Ok(b) => b, Err(code) => return code };

    let (header, body) = match vita_artifact::read_velab(&bytes) {
        Ok(x) => x,
        Err(e) => return emit_artifact_error(&sink, &e),       // bad magic → E-ART-FORMAT-MISMATCH
    };
    let tool = vita_artifact::ToolContext::current();          // SimIr-flavored
    if let Err(e) = vita_artifact::verify_header(&header, &tool) {
        return emit_artifact_error(&sink, &e);                 // schema/version → E-ART-SCHEMA-MISMATCH / E-ART-VERSION-GATE
    }

    // split the golden SimIr frame from the fork trailer.
    let (ir, rest): (sim_ir::SimIr, &[u8]) = match postcard::take_from_bytes(body) {
        Ok(x) => x,
        Err(e) => return emit_artifact_error(&sink,
            &vita_artifact::ArtifactError::format(&format!("undecodable .velab SimIr body: {e}"))),
    };
    let fork_modes: sim_engine::ForkModeTable = match postcard::from_bytes(rest) {
        Ok(m) => m,
        Err(e) => return emit_artifact_error(&sink,
            &vita_artifact::ArtifactError::format(&format!("undecodable .velab fork trailer: {e}"))),
    };

    // ── simulate (run_vita_str 254–260) ──
    let sim_opts = SimOpts { fork_modes, ..opts.sim_opts() };
    let result = sim_engine::simulate(&ir, &sink, sim_opts);
    sim_exit_code(&result)
}
```

**The CLI names `sim_engine::ForkModeTable` for the `vrun` decode.** `vita-artifact` deliberately does
NOT re-export `ForkModeTable` (verified `vita-artifact/src/lib.rs:5-9` re-exports only the gate + header
items), and must not — `ForkModeTable` lives in `elaborate`, and re-exporting it would couple the
language-neutral container to the front-end. `sim-engine` already re-exports it
(`sim-engine/src/lib.rs:40`, `pub use elaborate::{ForkModeTable, JoinMode}`) and `cli` already depends
on `sim-engine`, so `sim_engine::ForkModeTable` is the canonical spelling. `sim_engine::ForkModeTable`,
`elaborate::ForkModeTable`, and `SimOpts.fork_modes`'s type are all the **same** `elaborate` alias, so
the assignment into `SimOpts.fork_modes` typechecks without conversion. All 12 tests already use
`sim_engine::ForkModeTable`. The `velab` encode side uses the table returned by `elaborate_with_modes`
directly, so it needs no named type.

### 3.5 Wiring into `resolve_applet` / dispatch (`run`, replacing the stub at 376–381)

`resolve_applet` already returns `Applet::Staged("vcmp"|"velab"|"vrun")` + the remaining args. Replace
the stub arm with arg parsing + dispatch:

```rust
pub fn run(argv: &[String]) -> i32 {
    let (applet, args) = resolve_applet(argv);
    match applet {
        Applet::Vita => run_vita(&args, &VitaOpts::default()),
        Applet::Staged("vcmp")  => dispatch_vcmp(&args),
        Applet::Staged("velab") => dispatch_velab(&args),
        Applet::Staged("vrun")  => dispatch_vrun(&args),
        Applet::Staged(other) => {
            eprintln!("error[{}]: unknown staged applet '{other}'", MsgCode::CliBadFlag.code_num());
            EXIT_CLI_ERROR
        }
    }
}

/// Parse a flat arg list into (positional paths, -o value). `-o`/`--out` consume the next arg.
/// Unknown flags → Err(EXIT_CLI_ERROR). Mirrors the tiny v1 flag surface (doc-13 bucket-C is later).
fn parse_io_args(args: &[String]) -> Result<(Vec<String>, Option<String>), i32> {
    let mut pos = Vec::new();
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--out" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("error[{}]: '-o' needs an argument", MsgCode::CliBadFlag.code_num());
                    return Err(EXIT_CLI_ERROR);
                };
                out = Some(v.clone());
                i += 2;
            }
            s if s.starts_with('-') && s.len() > 1 => {
                eprintln!("error[{}]: unknown flag '{s}'", MsgCode::CliBadFlag.code_num());
                return Err(EXIT_CLI_ERROR);
            }
            _ => { pos.push(args[i].clone()); i += 1; }
        }
    }
    Ok((pos, out))
}

fn dispatch_vcmp(args: &[String]) -> i32 {
    let (pos, out) = match parse_io_args(args) { Ok(x) => x, Err(c) => return c };
    if pos.is_empty() {
        eprintln!("error[{}]: vcmp: no source files", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    let out = out.unwrap_or_else(|| default_out(&pos[0], "vu"));
    if let Err(c) = reject_out_clobbers_input(&pos, &out) { return c; }
    run_vcmp(&pos, &out, &VitaOpts::default())
}

fn dispatch_velab(args: &[String]) -> i32 {
    let (pos, out) = match parse_io_args(args) { Ok(x) => x, Err(c) => return c };
    if pos.len() != 1 {
        eprintln!("error[{}]: velab: expected exactly one .vu input", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    let out = out.unwrap_or_else(|| default_out(&pos[0], "velab"));
    if let Err(c) = reject_out_clobbers_input(&pos, &out) { return c; }
    run_velab(&pos[0], &out, &VitaOpts::default())
}

fn dispatch_vrun(args: &[String]) -> i32 {
    let (pos, out) = match parse_io_args(args) { Ok(x) => x, Err(c) => return c };
    if pos.len() != 1 {
        eprintln!("error[{}]: vrun: expected exactly one .velab input", MsgCode::CliBadFlag.code_num());
        return EXIT_CLI_ERROR;
    }
    // vrun accepts -o as a VCD path override (parity with one-shot vita -o).
    // Guard: a `-o` that names the input `.velab` would clobber the file being read.
    if let Some(ref o) = out {
        if let Err(c) = reject_out_clobbers_input(&pos, o) { return c; }
    }
    // `..Default::default()` so adding a bucket-C VitaOpts field later (the flag surface is
    // DEFERRED — §6 DEFERRED item 3) is non-breaking — VitaOpts derives Default
    // (verified cli/src/lib.rs:32).
    let opts = VitaOpts { vcd_path_override: out, ..Default::default() };
    run_vrun(&pos[0], &opts)
}
```

`main.rs` (the thin `cli::run` wrapper) is **unchanged**. The existing test
`unknown_applet_via_run_exits_three` (455–461) MUST be updated — `vcmp top.sv` now runs vcmp and fails
only because `top.sv` is missing (still exit 3 via the read-error path), but the assertion meaning
changes; replace it with the staged tests in §5.

---

## 4. Staleness gate behavior & error → exit-code mapping

The gate kinds and their codes are reused verbatim from `gate.rs` / `diag::MsgCode`. The CLI maps every
artifact-gate rejection to **exit 1 (user/design error)** — the artifact is structurally a design/data
problem the user fixes by re-running the upstream stage, not a CLI-usage error.

| Condition | `ArtifactError` ctor | MsgCode (mnemonic / num) | Severity | CLI exit |
|---|---|---|---|---|
| `.vu`/`.velab` magic wrong / header undecodable | `::format` | `ArtFormatMismatch` / E-ART-FORMAT-MISMATCH / VITA-E9001 | Error | **1** |
| Body undecodable behind a valid header (SourceUnit / SimIr / trailer) | `::format` | `ArtFormatMismatch` / VITA-E9001 | Error | **1** |
| `format_version` mismatch | `::format` | `ArtFormatMismatch` / VITA-E9001 | Error | **1** |
| `tool_semver_major` mismatch | `::version` | `ArtVersionGate` / E-ART-VERSION-GATE / VITA-E9004 | Error | **1** |
| **`.vu` SourceUnit hash ≠ velab's expected** | `::schema` | `ArtSchemaMismatch` / E-ART-SCHEMA-MISMATCH / VITA-E9002 | Error | **1** |
| **`.velab` SimIr hash ≠ vrun's expected** | `::schema` | `ArtSchemaMismatch` / VITA-E9002 | Error | **1** |
| Missing / unreadable input file | (no ArtifactError) | `FlistNotFound` printed to stderr | — | **3** |
| Missing `-o` arg / unknown flag / wrong arg count | (no ArtifactError) | `CliBadFlag` printed to stderr | — | **3** |
| Clean simulate | — | — | — | **0** |
| Runtime `$fatal` / `$error` / delta-limit | — | (sim diagnostics) | — | **1** (via `sim_exit_code`) |

`verify_header` gate order is fixed: `format_version` → `tool_semver_major` → `schema_hash`. So a stale
`.vu`/`.velab` from a compatible build (same version, same semver-major, different shape) deterministically
hits **E-ART-SCHEMA-MISMATCH**. A future-format artifact hits **E-ART-FORMAT-MISMATCH** first.

**RULE-V (E-ART-STALE-UPSTREAM / VITA-E9003) is DEFERRED.** `composite_input_hash`/`consumed`/
`worklib_manifest_hash` are stamped (zeroed in v1) and round-tripped but their live upstream-rehash gate
is not wired. The code exists in `diag` but is unused by the staged flow. State this in user docs.

---

## 5. Tests (12 cases, exact assertions)

New file `crates/cli/tests/staged_flow.rs` (integration tests; the CLI lib is the SUT). Temp files use a
per-call nonce for parallel safety, mirroring `run_on_temp` (cli/src/lib.rs:392). Plus two unit tests in
`vita-artifact` for the `.vu` container round-trip.

### 5.A `vita-artifact/tests/vu_roundtrip.rs`

```rust
//! `.vu` container = MAGIC_VU ++ postcard(header) ++ body; header-only decode preserves the body.
use vita_artifact::{read_vu, write_vu, Provenance, VelabHeader, MAGIC_VU};

fn hdr(schema: [u8; 32]) -> VelabHeader {
    VelabHeader {
        format_version: vita_artifact::CURRENT_FORMAT_VERSION,
        schema_hash: schema,
        composite_input_hash: [0; 32],
        global_time_precision: 0,
        consumed: Vec::new(),
        worklib_manifest_hash: [0; 32],
        uses_dump: false,
        tool_semver_major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        provenance: Provenance::capture(),
    }
}

// TEST 1: VU magic prefixes the stream and the body is preserved byte-for-byte.
#[test]
fn vu_magic_and_body_preserved() {
    let bytes = write_vu(&hdr([0xCD; 32]), b"VU-BODY-BYTES");
    assert_eq!(&bytes[..8], &MAGIC_VU);
    let (got, body) = read_vu(&bytes).expect("decode");
    assert_eq!(got.schema_hash, [0xCD; 32]);
    assert_eq!(body, b"VU-BODY-BYTES", "header-only decode must leave the body untouched");
}

// TEST 2: a `.vu` fed to read_velab fails the FORMAT gate (magic mismatch), not the schema gate.
#[test]
fn vu_read_as_velab_is_format_mismatch() {
    let bytes = write_vu(&hdr([0xCD; 32]), b"x");
    let err = vita_artifact::read_velab(&bytes).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
}
```

### 5.B `cli/tests/staged_flow.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_staged_{}_{n}.{ext}", std::process::id()))
}
fn write(p: &std::path::Path, s: &str) { std::fs::write(p, s).unwrap(); }
fn s(p: &std::path::Path) -> String { p.to_string_lossy().into_owned() }

const CLEAN_TB: &str =
    "module tb; reg a; initial begin a=1; $display(\"a=%b\",a); #5 $finish; end endmodule";

// TEST 3: `.vu` round-trip — vcmp writes a file whose decoded SourceUnit equals
//          the directly-parsed SourceUnit (byte-equal via postcard re-encode).
#[test]
fn vu_roundtrip_sourceunit_byte_equal() {
    let src = tmp("sv"); write(&src, CLEAN_TB);
    let vu = tmp("vu");
    assert_eq!(cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default()), cli::EXIT_OK);

    // decode the `.vu` body back to a SourceUnit
    let bytes = std::fs::read(&vu).unwrap();
    let (_h, body) = vita_artifact::read_vu(&bytes).expect("read_vu");
    let decoded: hdl_ast::SourceUnit = postcard::from_bytes(body).expect("decode SourceUnit");

    // Build the reference through the SAME preprocess→lex→parse path run_vcmp uses, so the
    // comparison does not silently depend on preprocess-identity or token-span invariance.
    // `cli::frontend_to_unit` is the §3.2 helper (read+preprocess+lex+parse → SourceUnit),
    // re-exported pub(crate)→pub for tests. We pass the on-disk file (with its trailing '\n')
    // exactly as run_vcmp does, so any preprocessing/span behavior is identical on both sides.
    let reference = cli::frontend_to_unit(&s(&src), &cli::StderrSink::new())
        .expect("reference frontend parse");
    assert_eq!(decoded, reference,
        "round-tripped SourceUnit must equal the production frontend's SourceUnit");

    let _ = std::fs::remove_file(&src); let _ = std::fs::remove_file(&vu);
}

// TEST 4: `.velab` round-trip — SimIr AND ForkModeTable survive the body layout.
#[test]
fn velab_roundtrip_simir_and_forktable() {
    let src = tmp("sv"); write(&src, CLEAN_TB);
    let vu = tmp("vu"); let velab = tmp("velab");
    assert_eq!(cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default()), cli::EXIT_OK);
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()), cli::EXIT_OK);

    let bytes = std::fs::read(&velab).unwrap();
    let (h, body) = vita_artifact::read_velab(&bytes).expect("read_velab");
    assert_eq!(h.schema_hash, vita_schema::schema_hash::<sim_ir::SimIr>(), "header carries SimIr hash");
    // split SimIr frame from trailer exactly as vrun does
    let (_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).expect("SimIr frame");
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).expect("fork trailer");
    assert!(modes.is_empty(), "fork-free design → empty trailer (one varint byte)");

    let _ = std::fs::remove_file(&src); let _ = std::fs::remove_file(&vu); let _ = std::fs::remove_file(&velab);
}

// TEST 5: END-TO-END staged chain produces the SAME $display output as one-shot vita.
//          vcmp a.sv -> a.vu ; velab a.vu -> a.velab ; vrun a.velab.
#[test]
fn staged_chain_matches_oneshot_display() {
    let src = tmp("sv"); write(&src, CLEAN_TB);
    let vu = tmp("vu"); let velab = tmp("velab");

    // one-shot reference output via simulate_capture — parse through the SAME front-end
    // helper run_vcmp uses (no preprocessor bypass), so the reference IR is built on an
    // identical SourceUnit to the staged path.
    let ref_unit = cli::frontend_to_unit(&s(&src), &cli::StderrSink::new()).unwrap();
    let ref_ir = elaborate::elaborate(&ref_unit, &cli::StderrSink::new()).unwrap();
    let (ref_res, ref_out) = sim_engine::simulate_capture(&ref_ir, sim_engine::SimOpts::default());

    // staged chain
    assert_eq!(cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default()), cli::EXIT_OK);
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()), cli::EXIT_OK);

    // vrun via the public path returns the same exit class as one-shot
    let code = cli::run_vrun(&s(&velab), &cli::VitaOpts::default());
    assert_eq!(code, cli::EXIT_OK);

    // and the staged SimIr produces byte-identical $display text as the reference
    let bytes = std::fs::read(&velab).unwrap();
    let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
    let (staged_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
    let (staged_res, staged_out) =
        sim_engine::simulate_capture(&staged_ir, sim_engine::SimOpts { fork_modes: modes, ..Default::default() });
    assert_eq!(staged_out, ref_out, "staged $display transcript must equal one-shot");
    assert!(ref_out.contains("a=1") && staged_out.contains("a=1"));
    assert_eq!(staged_res.exit_class, ref_res.exit_class);

    let _ = std::fs::remove_file(&src); let _ = std::fs::remove_file(&vu); let _ = std::fs::remove_file(&velab);
}

// TEST 6: END-TO-END VCD parity — staged path writes a VCD byte-equal to one-shot vita -o.
#[test]
fn staged_chain_matches_oneshot_vcd() {
    let dump = "module tb; reg a; initial begin $dumpfile(\"IGNORED\"); $dumpvars(0,tb); a=1; #5 $finish; end endmodule";
    let src = tmp("sv"); write(&src, dump);
    let vu = tmp("vu"); let velab = tmp("velab");
    let vcd_oneshot = tmp("vcd"); let vcd_staged = tmp("vcd");

    // one-shot
    assert_eq!(cli::run_vita(&[s(&src)],
        &cli::VitaOpts { vcd_path_override: Some(s(&vcd_oneshot)), ..Default::default() }), cli::EXIT_OK);
    // staged
    assert_eq!(cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default()), cli::EXIT_OK);
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()), cli::EXIT_OK);
    assert_eq!(cli::run_vrun(&s(&velab),
        &cli::VitaOpts { vcd_path_override: Some(s(&vcd_staged)), ..Default::default() }), cli::EXIT_OK);

    let a = std::fs::read_to_string(&vcd_oneshot).unwrap();
    let b = std::fs::read_to_string(&vcd_staged).unwrap();
    assert!(a.contains("$enddefinitions"));
    assert_eq!(a, b, "staged VCD must be byte-identical to one-shot VCD");

    for p in [&src,&vu,&velab,&vcd_oneshot,&vcd_staged] { let _ = std::fs::remove_file(p); }
}

// TEST 7: a schema-mismatch `.vu` is rejected with E-ART-SCHEMA-MISMATCH.
//          (Forge a `.vu` whose header carries the WRONG SourceUnit hash; velab must reject.)
#[test]
fn velab_rejects_stale_vu_schema_mismatch() {
    // valid SourceUnit body, but header schema_hash deliberately corrupted.
    let (toks, _) = hdl_lexer::lex(CLEAN_TB);
    let (unit, _) = hdl_parser::parse(&toks, CLEAN_TB);
    let body = postcard::to_stdvec(&unit.unwrap()).unwrap();
    let mut h = forge_vu_header(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    h.schema_hash[0] ^= 0xFF;                          // wrong shape signature
    let bytes = vita_artifact::write_vu(&h, &body);
    let vu = tmp("vu"); std::fs::write(&vu, &bytes).unwrap();

    // run_velab must reject at the gate → exit 1 (and emit E-ART-SCHEMA-MISMATCH).
    let velab = tmp("velab");
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()), cli::EXIT_USER_ERROR);
    assert!(!velab.exists(), "rejected .vu must not produce a .velab");

    // direct gate assertion proves the exact code.
    let (got, _) = vita_artifact::read_vu(&bytes).unwrap();
    let tool = vita_artifact::ToolContext::new(vita_schema::schema_hash::<hdl_ast::SourceUnit>());
    let err = vita_artifact::verify_header(&got, &tool).unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtSchemaMismatch);

    let _ = std::fs::remove_file(&vu);
}

// TEST 8: a bad-magic file is rejected with E-ART-FORMAT-MISMATCH (and vrun/velab exit 1).
#[test]
fn vrun_rejects_bad_magic() {
    let junk = tmp("velab"); std::fs::write(&junk, b"NOTVELAB....garbage").unwrap();
    assert_eq!(cli::run_vrun(&s(&junk), &cli::VitaOpts::default()), cli::EXIT_USER_ERROR);
    let err = vita_artifact::read_velab(b"NOTVELAB....garbage").unwrap_err();
    assert_eq!(err.code, diag::MsgCode::ArtFormatMismatch);
    let _ = std::fs::remove_file(&junk);
}

// TEST 9: a `.velab` whose SimIr header hash != vrun expected → E-ART-SCHEMA-MISMATCH.
#[test]
fn vrun_rejects_stale_velab_schema_mismatch() {
    let src = tmp("sv"); write(&src, CLEAN_TB);
    let vu = tmp("vu"); let velab = tmp("velab");
    cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default());
    cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default());

    // corrupt the header schema_hash in place, re-stitch, and feed to vrun.
    let bytes = std::fs::read(&velab).unwrap();
    let (mut h, body) = vita_artifact::read_velab(&bytes).unwrap();
    h.schema_hash[0] ^= 0xFF;
    let bad = vita_artifact::write_velab(&h, body);
    let stale = tmp("velab"); std::fs::write(&stale, &bad).unwrap();
    assert_eq!(cli::run_vrun(&s(&stale), &cli::VitaOpts::default()), cli::EXIT_USER_ERROR);

    let tool = vita_artifact::ToolContext::current();
    let (got, _) = vita_artifact::read_velab(&bad).unwrap();
    assert_eq!(vita_artifact::verify_header(&got, &tool).unwrap_err().code,
               diag::MsgCode::ArtSchemaMismatch);

    for p in [&src,&vu,&velab,&stale] { let _ = std::fs::remove_file(p); }
}

// TEST 10: missing input file → CLI/usage error (exit 3) for all three applets.
#[test]
fn missing_input_exits_three() {
    let nope = "/nonexistent/path/staged_xyz.vu".to_string();
    assert_eq!(cli::run_velab(&nope, "/tmp/x.velab", &cli::VitaOpts::default()), cli::EXIT_CLI_ERROR);
    assert_eq!(cli::run_vrun("/nonexistent/path/staged_xyz.velab", &cli::VitaOpts::default()), cli::EXIT_CLI_ERROR);
    assert_eq!(cli::run_vcmp(&["/nonexistent/path/x.sv".into()], "/tmp/x.vu", &cli::VitaOpts::default()), cli::EXIT_CLI_ERROR);
}

// TEST 11: corrupt body behind a VALID header → E-ART-FORMAT-MISMATCH (truncate the SimIr frame).
#[test]
fn vrun_rejects_corrupt_body() {
    let src = tmp("sv"); write(&src, CLEAN_TB);
    let vu = tmp("vu"); let velab = tmp("velab");
    cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default());
    cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default());

    // keep the header+magic, append only 1 body byte → undecodable SimIr frame.
    // `[0x01]` makes the SimIr's first field (a Vec) claim len=1 with no following element,
    // so `take_from_bytes::<SimIr>` runs out of bytes — a structurally incomplete frame.
    let bytes = std::fs::read(&velab).unwrap();
    let (h, _body) = vita_artifact::read_velab(&bytes).unwrap();
    let truncated = vita_artifact::write_velab(&h, &[0x01]); // 1-byte bogus body
    let bad = tmp("velab"); std::fs::write(&bad, &truncated).unwrap();
    assert_eq!(cli::run_vrun(&s(&bad), &cli::VitaOpts::default()), cli::EXIT_USER_ERROR);

    // Pin the failure to the BODY-DECODE boundary (not any incidental exit-1 path): the header
    // gate must PASS (hash unchanged), and only the SimIr frame decode must fail. This mirrors
    // TEST 8/9's direct-gate assertions and distinguishes a true corrupt-frame rejection from an
    // accidental valid-but-wrong decode that merely happens to reach exit 1 downstream.
    let (got, body) = vita_artifact::read_velab(&truncated).expect("magic+header still decode");
    assert!(vita_artifact::verify_header(&got, &vita_artifact::ToolContext::current()).is_ok(),
        "header gate must PASS — corruption is in the body, not the header");
    assert!(postcard::take_from_bytes::<sim_ir::SimIr>(body).is_err(),
        "the 1-byte SimIr frame must fail to decode (E-ART-FORMAT-MISMATCH boundary)");

    for p in [&src,&vu,&velab,&bad] { let _ = std::fs::remove_file(p); }
}

// TEST 12: THE FORK TRAILER survives the staged path — a fork design run via the staged
//          chain interleaves concurrently, exactly matching one-shot vita (the deferred FORK 13).
//          Marked #[ignore] until fork-join elaboration (2026-06-04-fork-join-spec) lands; the
//          ASSERTIONS are final so it flips to an active gate the moment forks elaborate.
#[test]
#[ignore = "enable once fork-join elaboration emits a non-empty ForkModeTable"]
fn fork_trailer_survives_staged_path() {
    // Two children that print in interleaved sim-time; `join` => parent waits for both.
    const FORK_TB: &str = "module tb; initial begin \
        fork \
          begin #1 $display(\"c1\"); end \
          begin #2 $display(\"c2\"); end \
        join \
        $display(\"done\"); $finish; end endmodule";
    let src = tmp("sv"); write(&src, FORK_TB);
    let vu = tmp("vu"); let velab = tmp("velab");
    assert_eq!(cli::run_vcmp(&[s(&src)], &s(&vu), &cli::VitaOpts::default()), cli::EXIT_OK);
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &cli::VitaOpts::default()), cli::EXIT_OK);

    // the trailer MUST be non-empty (the join carries a JoinMode::All entry).
    let bytes = std::fs::read(&velab).unwrap();
    let (_h, body) = vita_artifact::read_velab(&bytes).unwrap();
    let (staged_ir, rest): (sim_ir::SimIr, &[u8]) = postcard::take_from_bytes(body).unwrap();
    let modes: sim_engine::ForkModeTable = postcard::from_bytes(rest).unwrap();
    assert!(!modes.is_empty(), "fork design must persist a non-empty ForkModeTable trailer");

    // staged run interleaves c1 before c2, then done — identical to one-shot.
    let (staged_res, staged_out) = sim_engine::simulate_capture(
        &staged_ir, sim_engine::SimOpts { fork_modes: modes, ..Default::default() });
    let (toks, _) = hdl_lexer::lex(FORK_TB);
    let (unit, _) = hdl_parser::parse(&toks, FORK_TB);
    let (ir, fm) = elaborate::elaborate_with_modes(&unit.unwrap(), &cli::StderrSink::new());
    let (ref_res, ref_out) =
        sim_engine::simulate_capture(&ir.unwrap(), sim_engine::SimOpts { fork_modes: fm, ..Default::default() });
    assert_eq!(staged_out, ref_out);
    assert!(staged_out.find("c1").unwrap() < staged_out.find("c2").unwrap(), "concurrent interleave c1<c2");
    assert!(staged_out.contains("done"));
    assert_eq!(staged_res.exit_class, ref_res.exit_class);

    for p in [&src,&vu,&velab] { let _ = std::fs::remove_file(p); }
}
```

**Test harness notes:**

- TEST 7 needs `forge_vu_header(hash)` — a local copy of the §1.4 `vu_header` builder (the production
  one is private; the test re-implements the 9-field literal). Provide it inline in the test module.
- TESTS 3–12 require `cli` to **re-export** `StderrSink`, `VitaOpts`/`EXIT_*`/`run_vcmp/run_velab/run_vrun`,
  **and `frontend_to_unit`** publicly (the `run_*` fns are `pub`; `StderrSink` is already `pub`;
  `frontend_to_unit` is the §3.2 shared front-end helper that TEST 3 and TEST 5 call to build their
  reference `SourceUnit` through the production preprocess→lex→parse path). Add `pub use` if any are private.
- `cli/Cargo.toml` `[dev-dependencies]` must add `vita-artifact`, `vita-schema`, `postcard`, `hdl-ast`,
  `hdl-lexer`, `hdl-parser`, `elaborate`, `sim-ir`, `sim-engine`, `diag` (most already prod deps, re-list
  what the integration test names directly).
- Total: **2 (vita-artifact) + 10 (cli) = 12 named tests**, TEST 12 `#[ignore]` until forks elaborate.

---

## 6. Freeze confirmation, MsgCode, and DEFERRED list

### Frozen-IR / MsgCode

- **NO frozen-IR change.** Not one `sim-ir` type is edited; `schema_hash::<SimIr>()` stays at the pinned
  hash `7b46c170…1bcd4`. `hdl-ast::SourceUnit` is untouched (already derives `SchemaHash`, already
  pinned). `JoinMode`/`ForkModeTable` already serde-derive — no derive added, and they stay OUT of the
  golden closure by design.
- **NO new MsgCode.** The doc-15 bijection (36 codes, doc-15 ↔ `code.rs` gate) is **untouched**. The
  staged flow reuses `ArtFormatMismatch` (E9001), `ArtSchemaMismatch` (E9002), `ArtVersionGate` (E9004).
  `ArtStaleUpstream` (E9003) already exists but its gate stays deferred. The new `vita-artifact`
  surface (`MAGIC_VU`, `write_vu`/`read_vu`, `ToolContext::new`) introduces no new diagnostic.

### DEFERRED (explicit)

1. **RULE-V live upstream re-hash (E-ART-STALE-UPSTREAM / E9003).** `composite_input_hash` / `consumed`
   / `worklib_manifest_hash` are stamped (zeroed) and round-tripped but not gated. `velab` does not
   record the source `.vu` hash into `consumed`, and `vrun` does not re-hash its inputs. A later PR wires
   the live recheck.
2. **Incremental / cached rebuild.** No staleness-driven skip ("`.velab` newer than `.vu` → reuse"). Every
   stage recomputes. Out of MVP scope.
3. **Flag surface beyond `-o`/positional.** `+incdir+`, `-D`, `-y`/`-v` libraries, `-top`, timescale
   overrides — all DEFERRED to the doc-13 bucket-C surface (lands with `vita-log`). `PreOpts::default()`
   is used (no incdirs/defines) for `vcmp`.
4. **`global_time_precision` / `uses_dump` accuracy in the `.velab` header.** Stamped `0`/`false` in v1
   (hints, not gates). Threading the real timescale precision and a `$dumpvars` probe is a follow-up.
5. **Fork-trailer staleness protection.** The trailer is non-golden by design; if a future PR wants its
   shape gated, wrap it `struct ForkModeTrailer(ForkModeTable): SchemaHash` and fold into the header hash.
6. **Multi-`.vu` velab (separate-compilation / worklib).** `velab` takes exactly one `.vu` in v1. Linking
   multiple compiled units (the `worklib_manifest_hash` story) is DEFERRED.
7. **TEST 12 activation.** Gated `#[ignore]` until `2026-06-04-fork-join-spec` elaboration emits a
   non-empty `ForkModeTable`; assertions are already final.
8. **Scrub the stale `hdl-ast` module doc-comment.** `hdl-ast/src/lib.rs:9-18` still claims `SourceUnit`
   does NOT derive `SchemaHash`, contradicting the live derive (line 82) and the pinned golden. The PR
   that lands `.vu` wiring deletes/corrects those lines. Pure doc hygiene — no behavior change.
