# 14 · Staged artifacts

This is the authoritative specification for everything vita writes to disk between stages: the
`.vu` and `.velab` container formats, the work library, filelist expansion, the staleness gate,
the live upstream re-hash rule, and the equivalence contract between the staged chain and the
one-shot run. [04-architecture.md](04-architecture.md) summarises the execution model; this
document defines it.

---

## Design decisions

| # | Decision | Content |
|---|---|---|
| D1 | Dedicated crates | Container (de)serialization, the header, the version constant and the staleness gate live in `vita-artifact`. The structural shape hash is produced by the proc-macro crate `vita-artifact-derive` and composed at runtime by the leaf crate `vita-schema` ([16-schema-hash-spec.md](16-schema-hash-spec.md)). Body serialization and the trailer layout belong to `cli`, which keeps the container language-neutral. |
| D2 | Structural schema hash | The schema version is a structural hash derived from the *shape* of the serialized types (`#[derive(SchemaHash)]`), not a hand-incremented constant. The build fingerprint (tool version, git sha, dirty, profile) is stamped for traceability and is never a staleness key. |
| D3 | Multiple libraries | Compiled units are addressed by the logical key `library:unit`, not by path. A logical name resolves to a directory. |
| D4 | Span-free IR | `sim-ir` is language-neutral and carries no source spans. Runtime diagnostic locations travel out of band, in a side table keyed by IR node index; the `file → text` map travels on the `.vu`. |

---

## 1. Artifact layout

The pipeline leaves disk artifacts at two boundaries. The shapes differ: the compile output is a
directory of compiled units, the elaborate output is one self-contained file.

```
vcmp  (compile)     →  <first-source>.vu   or  <work-dir>/     (language-dependent)
velab (elaborate)   →  <top>.velab                             (language-neutral, self-contained)
vrun  (simulate)    →  waveform (when the RTL dumps) + stdout
```

### 1.1 The container

`.vu` and `.velab` share one container shape and one header type. Only the magic differs.

```
<8-byte magic> ++ postcard(VelabHeader) ++ <opaque body bytes>
```

| Constant | Value | Bytes |
|---|---|---|
| `MAGIC_VELAB` | `b"VELAB\0\0\0"` | `56 45 4C 41 42 00 00 00` |
| `MAGIC_VU` | `b"VU\0\0\0\0\0\0"` | `56 55 00 00 00 00 00 00` |

The two magics are deliberately distinct so that a `.vu` handed to `vrun` fails the format gate
rather than the schema gate, and the diagnostic names the real mistake.

The header is decoded alone, with `postcard::take_from_bytes`, before a single body byte is
deserialized. A bad magic, a short file or an undecodable header is `E-ART-FORMAT-MISMATCH`.

### 1.2 Header fields

`VelabHeader` has nine fields, in this wire order. It is `Serialize + Deserialize` but is **not**
`SchemaHash`-derived — the container version guards its layout.

| # | Field | Type | What the producers write |
|---|---|---|---|
| 1 | `format_version` | `u32` | `CURRENT_FORMAT_VERSION` = **31** |
| 2 | `schema_hash` | `[u8; 32]` | `.vu`: `schema_hash::<hdl_ast::SourceUnit>()`. `.velab`: `schema_hash::<sim_ir::SimIr>()` |
| 3 | `composite_input_hash` | `[u8; 32]` | The upstream digest (§2 RULE V). `vcmp`: blake3 over the concatenated raw source text plus the `-D`/`-I` surface. `velab`: blake3 of the whole consumed `.vu` file. `velab -L`: blake3 over every consumed compilation-unit blob, concatenated |
| 4 | `global_time_precision` | `i64` | The resolved design-wide precision exponent (`-9` = 1 ns) |
| 5 | `consumed` | `Vec<(String, [u8; 32])>` | Written empty. Reserved: the live consumption record rides trailer ⑨ |
| 6 | `worklib_manifest_hash` | `[u8; 32]` | Written all-zero. Reserved, same reason |
| 7 | `uses_dump` | `bool` | Written `false` |
| 8 | `tool_semver_major` | `u32` | `CARGO_PKG_VERSION_MAJOR` = **0** (workspace version 0.2.0) |
| 9 | `provenance` | `Provenance` | Captured at run time from build-time environment |

Fields 5, 6 and 7 are carried through unchanged and are gate-neutral: the gate never reads them.

Backend capability is not a header field. A `.velab` records resolved IR facts only. Whether a
given engine can execute a builtin or write a waveform is a property of the backend that loads
the snapshot, so it does not belong in the elaborate output.

### 1.3 Provenance

| Field | Type | Source |
|---|---|---|
| `tool_version` | `String` | `env!("CARGO_PKG_VERSION")` |
| `git_sha` | `Option<String>` | `option_env!("VITA_GIT_SHA")` — `None` on a plain local build |
| `dirty` | `bool` | `option_env!("VITA_GIT_DIRTY")` is `"1"` or `"true"` |
| `profile` | `String` | `"debug"` under `debug_assertions`, else `"release"` |

Capture uses `env!` / `option_env!` / `cfg!` only. vita's own crates carry no `build.rs`, so a
release wrapper injects the sha through the environment and an ordinary `cargo build` degrades
gracefully to "version and profile, sha unknown". Provenance exists for bug-report traceability
and is deliberately excluded from the staleness key: a dirty tree must not force a rebuild.

### 1.4 `.vu` body

```
postcard(hdl_ast::SourceUnit)                        # the schema-gated frame
++ postcard((unit_exp:        BTreeMap<String, i8>,  # timescale tail
             global_prec_exp: i8,
             prec_exp:        BTreeMap<String, i8>))
++ postcard((files:    Vec<(String, String)>,        # source-map tail
             segments: Vec<(u32, u32, u32, u32, bool)>))
```

- `unit_exp` is the per-module time-unit exponent, `prec_exp` the per-module time-precision
  exponent. Both feed the two-stage delay conversion of
  [08-timescale-and-timing.md](08-timescale-and-timing.md).
- The source-map tail carries exactly what a `SourceMap` resolver needs: the file name and full
  text of each source, and the expansion segments `(exp_start, exp_end, file_id, orig_start,
  collapsed)`. The machine-local `canon` and `dir` fields are deliberately not carried — they are
  absolute paths and would break byte-identity across platforms. `velab` rebuilds the resolver
  from this tail, which is what makes a staged elaborate diagnostic print the same
  `file:line:col` as the one-shot run.
- Read tolerance differs by tail. The timescale tail is tolerant: absent means empty maps and
  `global_prec_exp = -9`. The source-map tail is loud: absent or undecodable is
  `E-ART-FORMAT-MISMATCH`, because a silently location-less diagnostic is worse than a refusal.

### 1.5 `.velab` body — the golden frame plus fifteen trailers

```
postcard(sim_ir::SimIr)      # THE GOLDEN FRAME — the only schema-gated part
++ ① ++ ② ++ … ++ ⑮          # append-only trailer segments, in this exact order
```

| # | Segment | Type | Read tolerance | Meaning when empty |
|---|---|---|---|---|
| ① | `fork_modes` | `BTreeMap<(u32,u32), JoinMode>` | required | — |
| ② | `net_names` | `Vec<String>` | tolerant | flat `n{i}` waveform names |
| ③ | timescale | `(Vec<u64>, i8, Vec<u64>)` = (per-process multipliers, global precision exponent, per-process precision multipliers) | tolerant | `(vec![], -9, vec![])` = 1ns/1ns base |
| ④ | `severities` | `BTreeMap<u32, SeverityKind>` | tolerant | severity tasks degrade to plain `$display` |
| ⑤ | `radixes` | `BTreeMap<u32, u8>` | tolerant | decimal |
| ⑥ | `proc_scopes` | `Vec<String>` | tolerant | flat `top` for `%m` |
| ⑦ | `assign_ranks` | `BTreeSet<u32>` | tolerant | every Force/Release is a real force/release |
| ⑧ | `queue_bounds` | `BTreeMap<u32, u32>` | tolerant | every queue unbounded |
| ⑨ | `WorkConsumed` | work-library consumption record | tolerant | no work gate. Always written, even for an explicit-path build, so that ⑩ is unambiguous |
| ⑩ | `net_dims` | `BTreeMap<u32, Vec<(i64,u32)>>` | tolerant | 1-D zero-based element names |
| ⑪ | `final_procs` | `BTreeSet<u32>` | tolerant | no `final` blocks. Always written, same reason |
| ⑫ | `defer_marks` | `BTreeMap<u32, DeferRegion>` | tolerant | no deferred asserts |
| ⑬ | `defer_acts` | `BTreeMap<u32,(u32,DeferRegion)>` | tolerant | no deferred asserts |
| ⑭ | `StagedExtraSidecars` | 37-field struct | tolerant → `Default` | plain RTL costs about 13 length-zero bytes |
| ⑮ | `WorkStamps` | 3-field struct | tolerant → `Default` | every upstream entry re-hashes |

"Tolerant" means the reader checks for an exhausted stream first and substitutes the empty
default; a present-but-undecodable segment is always `E-ART-FORMAT-MISMATCH`, with a message
naming the segment. Every segment except the last decodes with `postcard::take_from_bytes`; the
last uses `postcard::from_bytes`.

Every trailer rides **outside** the hashed `SimIr` frame. The schema gate covers the type shape
of the golden frame, not these bytes. That is the reason a trailer change requires a
`format_version` bump rather than a schema-hash flip (§7), and the reason the two hand-maintained
trailers ⑭ and ⑮ carry their own pinned wire hashes.

### 1.6 Trailer ⑭ — `StagedExtraSidecars`

postcard encodes struct fields positionally, so field order is the wire contract and a rename is
wire-neutral. Fields 1–27 are unconditional; fields 28–37 carry `#[serde(default)]`, which is what
makes the tail append-only within one container version.

| # | Field | Type |
|---|---|---|
| 1 | `func_table` | `Vec<FuncMeta>` |
| 2 | `task_calls_proc` | `BTreeMap<(u32,u32), TaskCallInfo>` |
| 3 | `task_calls_func` | `BTreeMap<u32, TaskCallInfo>` |
| 4 | `two_state_nets` | `BTreeSet<u32>` |
| 5 | `class_handle_nets` | `BTreeSet<u32>` |
| 6 | `class_new_sites` | `BTreeMap<u32,u32>` |
| 7 | `class_layouts` | `Vec<Vec<(u32,bool,bool)>>` |
| 8 | `class_field_inits` | `Vec<Vec<Option<BitPacked>>>` |
| 9 | `class_vtable` | `Vec<Vec<u32>>` |
| 10 | `class_calls` | `BTreeMap<u32,(Option<u32>,u32)>` |
| 11 | `class_field_widths` | `BTreeMap<u32,(u32,bool)>` |
| 12 | `assert_fire` | `BTreeSet<u32>` |
| 13 | `assert_ctl` | `BTreeMap<u32,u8>` |
| 14 | `class_rand` | `Vec<Vec<RandBound>>` |
| 15 | `class_constraints` | `Vec<Vec<Vec<COp>>>` |
| 16 | `class_dist` | `Vec<Vec<DistField>>` |
| 17 | `class_randc` | `Vec<Vec<RandcField>>` |
| 18 | `randomize_with` | `Vec<RandWithCall>` |
| 19 | `clocking_inputs` | `BTreeSet<u32>` |
| 20 | `clocking_commit` | `BTreeMap<u32, Vec<(u32,u32)>>` |
| 21 | `clocking_outputs` | `BTreeMap<u32, Vec<(u32,u32)>>` |
| 22 | `ca_delays` | `BTreeMap<u32,(u32,u32,u32)>` (rise, fall, turn-off) |
| 23 | `wired_and_nets` | `BTreeSet<u32>` |
| 24 | `wired_or_nets` | `BTreeSet<u32>` |
| 25 | `timeformat_stmts` | `BTreeSet<u32>` |
| 26 | `handle_copy_stmts` | `BTreeMap<u32,(u32,u32)>` |
| 27 | `queue_slice_stmts` | `BTreeSet<u32>` |
| 28 | `func_names` | `Vec<String>` (FuncId → `module.function`) |
| 29 | `real_elem_dyn_nets` | `BTreeSet<u32>` |
| 30 | `string_elem_dyn_nets` | `BTreeSet<u32>` |
| 31 | `net_decl_ranges` | `BTreeMap<u32,(i64,i64)>` |
| 32 | `file_directed_stmts` | `BTreeSet<u32>` |
| 33 | `init_procs` | `Vec<u32>` (declaration-initializer ProcIds, in initialization order) |
| 34 | `stmt_locs` | `BTreeMap<u32, StmtLoc>` |
| 35 | `stmt_scopes` | `BTreeMap<u32, String>` (StmtId → `%m` named-block chain) |
| 36 | `expr_scopes` | `BTreeMap<u32, String>` (the `$sformatf` ExprId twin) |
| 37 | `proc_inst_scopes` | `Vec<String>` (ProcId → instance path, generate scopes stripped) |

Every field is populated one-to-one from the elaborator's own sidecar bundle, so the staged path
carries what the one-shot path holds in memory (§6).

### 1.7 Trailer ⑮ — `WorkStamps`

```rust
type FileStamp = (u64, u32, u64);   // (seconds since UNIX_EPOCH, subsec nanos, byte length)
struct WorkStamps {
    libs:  Vec<Option<FileStamp>>,
    blobs: Vec<Option<FileStamp>>,
    files: Vec<Option<FileStamp>>,
}
```

The three vectors run parallel to trailer ⑨'s `libs` / `blobs` / `files` — same order, same
length. `velab` records `Some(stamp)` only after re-reading the path and confirming that the bytes
still hash to the recorded digest, and rejects a length mismatch between the read and the stat.
`None` means "could not verify, always re-hash".

At `vrun` a matching live `(mtime, size)` skips the read and the blake3. Any mismatch, and any
absent stamp, falls back to the authoritative re-hash. This is a fast path, never a relaxation of
the rule: the content hash is always the deciding check. The residual hole is a rewrite that
keeps both the length and the recorded mtime.

### 1.8 Atomic write

Every artifact is written to `<out>.tmp.<pid>` and then renamed onto `<out>`; on a rename failure
the temporary file is removed. A same-directory rename is atomic on POSIX, so a crash mid-write
cannot leave a truncated artifact that the staleness gate would report as a format mismatch. No
`.tmp.` residue survives a successful write.

### 1.9 Serialization mechanism

- **serde derive at the boundary, `postcard 1.x` as the single encoder.** There is no second
  encoder and no fallback: two mutually incompatible artifact encodings would be two formats.
- **blake3 for every digest** — shape hashes, source digests, blob digests, manifest digests.
  Pure Rust, no C dependency, pinned to an exact version. Some recorded field names read
  `src_sha256`; the algorithm they name is blake3 throughout.
- **The one-shot run never calls serde.** `vita` streams the live `SourceUnit` and `SimIr` values
  from stage to stage in memory. Serialization happens only when `vcmp` or `velab` is asked to
  store something, so it is a purely optional boundary.

What postcard imposes on the design:

| Property | Consequence |
|---|---|
| Struct fields encode positionally | A field rename is wire-neutral; an add, remove, reorder or retype is not |
| Enum discriminants are positional | Appending a variant last leaves every existing value decoding unchanged |
| Integers are varint, `i64` zigzag | `u32` and `u64` encode identically for every value below 2^32, so widening a `Vec<u32>` element to `Vec<u64>` is wire-neutral; changing `(u32,u32)` to `(i64,u32)` is not |
| `Box<T>` encodes byte-identically to `T` | Mirrored by the derive's transparent `Box` arm, which is also how the recursive AST terminates |
| `Vec` and `String` lengths are varint | A length is width-independent, which is why `usize` is banned from schema types but `Vec::len()` is not a hazard |

### 1.10 The frozen process shape

The process body is a structured basic-block sequence, not a bytecode ISA, and the resume state is
physically reserved in the IR schema. The whole cluster is frozen as one atomic unit: any field
that changes shape flips `SCHEMA_HASH` and invalidates every `.velab` on disk, so a partial freeze
is not available. The rationale is in [06-simulation-engine.md](06-simulation-engine.md); the
shape is:

```rust
struct Process {                       // a velab body process node
    sensitivity: Sensitivity,          // static trigger set, ordered Vec
    body:        Vec<BasicBlock>,      // structured blocks; index = the resume_pc domain
    entry:       u32,                  // entry block (initial = once at t0, always* = re-armed)
    suspend:     SuspendState,         // the atomic freeze unit
}
struct SuspendState {
    resume_pc:   u32,                  // block index — never a pointer or a native PC
    locals:      Vec<FourState>,       // frame-0 locals, ordered Vec
    join_state:  JoinState,
    wake_key:    WakeKey,
    call_stack:  Vec<Frame>,           // LIFO; an inline-only process stores an empty Vec (1 byte)
    frame_arena: Vec<FourState>,       // per-process flat arena; a Frame addresses it by (base, len)
}
struct JoinState {
    parent:   Option<u32>,             // runtime process-table index, None = top level
    children: Vec<u32>,                // active children, appended in fork declaration order
    detached: Vec<u32>,                // join_any remainder plus join_none children — load-bearing
    flags:    ProcFlags,               // u8 bitset
}
struct Frame {
    return_pc: u32, callee_entry: u32, // return block / callee entry block
    locals_base: u32, locals_len: u32, // the locals window inside frame_arena
    is_automatic: bool,                // automatic ⇒ private window, static ⇒ fixed storage alias
}
struct WakeKey {
    cond: WakeCond, region: RegionTag, // the region is stored, never re-derived at wake time
    tie_break: u32,                    // flattened-hierarchy declaration order, not a user priority
}
struct ProcFlags(u8);                              // newtype so the shape is distinct from a bare u8
enum RegionTag { Active, Inactive, Nba, Monitor }  // the four IEEE 1364 regions that ride the IR
enum WakeCond {                                    // the closed set of process-suspend conditions
    Edge { net: u32, kind: EdgeKind }, Level { nets: Vec<u32> }, WaitTrue { expr: u32 },
    TimeAbs { tick: u64 }, NamedEvent { ev: u32 }, Join { join_ref: u32 },
}
// Terminator, inside body: Goto / Branch / Delay / Wait / Fork / Call / Return.
```

Invariants the shape does not express and the engine must therefore hold:

1. **No `usize` or `isize`, no `f32` or `f64`, no `HashMap` or `HashSet`** in any schema type.
   The derive rejects all of them at compile time; a widths-vary-by-target field or a
   hash-ordered container would break byte-identity across platforms.
2. `children`, `detached`, `call_stack` and `frame_arena` are appended in declaration order and
   removed order-preservingly. No `swap_remove`, no freed-slot reuse pool.
3. `WakeKey.region` is filled at elaborate time and stored, so scheduling stays inside the hash.
4. A new wait kind, or a wider region model, is an intentional `SCHEMA_HASH` flip plus a rebuild.
   Reserving spare variants in advance is not the route — it only bloats the hashed surface.

`RegionTag` names the four IEEE 1364 regions that ride the frozen IR. Three more regions —
preponed, observed and reactive — are modelled out of band, where they do not touch the golden
shape, so seven of IEEE 1800's seventeen are implemented and every construct needing one of the
remaining ten is honestly loud.

---

## 2. Hash binding

This section is the load-bearing part of the document. Skipping an unchanged stage is only sound
if every input that can change a stage's output is bound into that stage's staleness identity. One
omission means a stale artifact is silently reused and the simulation is wrong.

**RULE 0.** Classify a flag by *which stage's output it disturbs*, not by which binary parses it.
Three buckets:

- **A** — disturbs preprocessing output ⇒ binds into the compile-side source digest.
- **B** — disturbs the elaborated `SimIr` ⇒ binds into the snapshot's identity.
- **C** — disturbs neither ⇒ runtime or reporting only, hashed nowhere.

**RULE A (compile-side digest inputs).** Include-path, macro-define, source-library and language-
dialect flags change the preprocessed bytes or which unit a name resolves to, so they belong in
the *preimage* of the compile-side digest, not merely stamped beside it. Stamping alone would let
a dialect switch reuse an artifact whose bytes happen to match.

**RULE B (elaborate-side identity).** Top selection, parameter and defparam overrides, library
composition, library-map resolution and the resolved global time precision change the flattened
`SimIr`, so they belong to the snapshot's identity.

Status at HEAD: the `.velab` header's `composite_input_hash` is an **upstream** digest — the bytes
of the consumed `.vu`, or of the consumed compilation-unit blobs in library mode. Elaborate-side
argv is not folded in, and `-G` / `--param` is excluded deliberately: `vrun --upstream` must be
able to recompute the digest from the live file alone, and a digest that mixes in an override
cannot be reproduced from the file, so every override build would be rejected as stale against a
file that has not changed. Two snapshots elaborated from the same `.vu` with different overrides
are therefore indistinguishable to the gate; the snapshot is named by the user, and the gate's job
is upstream freshness rather than command reconstruction.

**RULE C (runtime only — hashed nowhere).** Plusargs, seeds, `$stop` handling, run-length limits,
logging, verbosity, warning gating, output naming, colour and version reporting change runtime
behaviour or packaging. Hashing any of them would break the guarantee that one `.velab` can be run
twice with different runtime flags and stay valid both times, and would force pointless rebuilds.

**RULE T (the timescale trap — double binding).** A global timescale default is the one input that
legitimately enters both digests. As an injected preprocessor directive it changes the compiled
bytes (RULE A); as the source of the global precision recorded in the `.velab` header it changes
the elaborated time model (RULE B). It must be bound on both sides.

**RULE S (directive inheritance).** Sticky compiler directives such as `` `timescale `` and
`` `default_nettype `` carry across file boundaries within a compilation unit
([08-timescale-and-timing.md](08-timescale-and-timing.md)). A later unit's preprocessed output
therefore depends on the order of its siblings: its own bytes are unchanged, yet it inherits a
different timescale when an earlier file changes. Two consequences follow. A per-unit source digest
must be computed over the bytes *after* inheritance, and the ordered list of compilation-unit files
must itself be a digest input, so that adding, removing or reordering a file invalidates. The
ordered list is produced by filelist expansion (§4), whose depth-first declaration-order flattening
is a precondition for that soundness. Every future sticky directive is treated the same way.

Status at HEAD: the compilation unit is a single concatenation, so per-unit preprocessed bytes are
not defined and the recorded digest is over the raw file bytes of every source plus the include
closure plus the `-D`/`-I` surface. Detection power is equivalent — any edit to any participating
byte, in a source or in an included file, trips the gate — but the recorded digest is not the
post-inheritance form the rule describes. The sibling-order hazard is answered instead at
expansion time, by `E-FLIST-DUP-CTX-CONFLICT` (§4).

**RULE D2 (the schema hash is an input too).** The structural `SCHEMA_HASH` of the serialized types
is stamped in both headers. A type-shape change flips it and invalidates every `.vu` and `.velab`
written before. It composes with the content digests: an artifact is fresh if and only if the
schema hash matches **and** the content digests match. The build fingerprint is stamped but is not
a staleness key.

**RULE V (live upstream re-hash).** `vrun` re-validates its upstream chain against the live sources
on every run, rather than trusting a recorded timestamp or a fast-path skip. Mtime is never a
sound freshness signal on its own; only the content hash is. When a live digest differs, `vrun`
refuses to simulate the stale snapshot. This is an intentional departure from the check-skipping
fast paths that commercial run flags offer. Implementation and diagnostics: §5.3.

**RULE API (make mis-binding structurally impossible).** The hashing functions take typed input
structs — a preprocess-inputs struct and an elaborate-inputs struct — and never raw argv. Logging,
packaging and runtime flags have no field in those structs, so they cannot physically reach a
digest. Adding a new preprocess or elaborate flag requires adding a field, which makes the binding
decision a compile-time obligation.

**RULE F (a filelist is transport; its contents are hashed).** The `-f` / `-F` flag itself is
bucket C — the file is never hashed, because how sources were grouped into filelists, or what a
`.f` was renamed to, must not invalidate an artifact. The expanded *contents* bucket exactly as if
they had been typed inline: a `+define+` three levels deep binds like a command-line one, a library
binding binds like a command-line one. There is no "came from a file" bucket.

### Flag buckets at HEAD

| Flag | Stage | Bucket | How it binds |
|---|---|---|---|
| `+define+` / `-D` / `--define` | vcmp, vita | A | in the compile digest preimage, as `name=value` lines in argv order |
| `+incdir+` / `-I` / `--incdir` | vcmp, vita | A | in the preimage as directory lines; every file the preprocessor actually opens is recorded with its own digest |
| `--work` / `--workdir` | vcmp | A | selects the library name and directory, which is the `library:unit` namespace; the manifest is itself hashed |
| `--top` | velab, vita | B | selects the elaboration root; not recorded in any digest (see the RULE B status above) |
| `-G` / `--param` | velab, vita | B | applied at elaborate; deliberately excluded from the digest |
| `-L` | velab | B | every consumed blob and every source and include behind it is recorded in trailer ⑨ |
| `-o` / `--out` | all | C | names the output |
| `--backend`, `--threads` / `-j`, `--timeout`, `+plusargs` | vita, vrun | C | runtime only; output must be byte-identical across backends and thread counts |
| `-l` / `--log`, `--log-append`, `-q` / `-v` / `-vv` / `--verbosity`, `-Wno-`, `-Werror[=]` | all | C | reporting only |
| `--obs-dir`, `--obs-procs[-time]`, `--probe`, `--probe-file`, `--hier-tree`, `--inst-paths` | vita | C | observability output only ([19-ai-agent-observability.md](19-ai-agent-observability.md)) |
| `--upstream` | vrun | C | selects *what* is re-checked; it is not itself hashed |
| `-f` / `-F`, `--dump-filelist` | all | C | transport and dry-run inspection |

### Specified, not implemented

These belong to the bucket model above and are stated here so a reader never mistakes them for a
shipped surface. Each is a present fact about HEAD, not a schedule.

| Flag | Intended bucket | Status at HEAD |
|---|---|---|
| `--std` / `-g<year>`, `-sv` | A (preimage, plus a global dialect digest) | not accepted; dialect is fixed |
| `-y <libdir>`, `-Y <ext>` / `+libext+`, `-v <libfile>` | A | not accepted; there is no source-library search |
| `--timescale`, `--timescale-policy` | A and B (RULE T) | not accepted; the global default resolves to the 1ns/1ns base |
| `-s` / `--top-module`, `-pvalue+` | B | not accepted; the spellings are `--top` and `-G` / `--param` |
| `-P<hier.path.param>=`, `-P <dir>`, `--lib-map` | B | not accepted; defparam-style overrides, precompiled-library search and an external logical-name map are absent |
| `--multi-driver` | B | not accepted; multi-driver resolution is always the IEEE 4-state merge and detection severity is fixed |
| `-E` (preprocess only), `--dump` (artifact text view) | outside the hash model | not accepted |
| `--rebuild`, `--clean` | — | no such flag; a stale chain is rebuilt by re-running the stages |
| `-sv_seed`, `-n` / `-N`, `--finish-at`, `-M` / `-m` | C | not accepted; the run-length cap is `--timeout` |

### Why the buckets matter

Leave `+define+WIDTH=8` out of the compile-side digest and the source bytes still match, so the
compile is skipped, so a `.vu` compiled with a different macro is reused, so a run intended for
`WIDTH=16` simulates 8 — silently, at exit 0. RULE A closes that class. RULE S closes the same
accident arriving through sibling order instead of through a flag.

---

## 3. Work libraries

Compiled units are addressed by the logical key `library:unit` rather than by path, as in the
commercial library-map flows and in GHDL's `--work` / `-P`. A logical name resolves to a directory.

```
<dir>/lib.toml                # canonical, machine-written manifest
<dir>/units/cu_<32 hex>.vu    # content-addressed compilation-unit blobs
```

A blob is byte-identical to what `vcmp -o` would write for the same input, and is named by the
first 32 hexadecimal characters of the blake3 digest of its own bytes. With `--work` and no `-o`,
`vcmp` writes only the library blob and no free-standing `.vu`.

### 3.1 The manifest

`lib.toml` has one canonical text form. It is machine-written, and the parser is strict: the form
below is the only one accepted, and it is also the byte domain of the manifest digest.

```toml
# vitamin work library manifest (canonical v1; machine-written by `vcmp --work`)
format_version = 1
name = "work"

[[cu]]
blob = "units/cu_<32 hex>.vu"
defines = []
incdirs = []
sources = [
  "<64 hex blake3><two spaces><path>",
]
includes = [
  "<64 hex blake3><two spaces><path>",
]
units = [
  "module top",
]
```

- The manifest's own `format_version` is **1**. It is independent of the container
  `format_version` (§7) and of the schema hash; the two numbers are unrelated.
- A unit line is `<kind> <name>`, where kind is `module`, `interface`, `package` or `class`.
- Spacing is literal. `sources= [` is rejected.
- `emit` is a fixed point: parsing and re-emitting a canonical manifest reproduces it byte for
  byte, which is what makes the manifest digest meaningful.
- Escaping covers backslash and double quote only. A recorded path or define containing a newline
  is rejected when it is recorded, not left to corrupt the next parse.
- Any deviation is `E-WORK-MANIFEST` (`VITA-E9005`) with a 1-based line number, exit 2.
- Paths are recorded as they were given to `vcmp`. Storing them relative to a detected build root
  would make a manifest digest identical across two checkouts of the same sources; at HEAD it is
  not, so a manifest built from absolute paths is checkout-specific.

Replacement rule: any existing compilation unit that shares at least one source path with a newly
compiled one is superseded, and the new unit takes the earliest such slot, so recompiling a source
updates the library in place. A duplicate unit name that survives that replacement is a real
redefinition — `E-DUP-UNIT` (`VITA-E2001`), exit 1, manifest untouched. A blob that no unit
references is collected on a best-effort basis.

### 3.2 What a snapshot records about its library

Trailer ⑨ carries the consumption record:

| Vector | Entry |
|---|---|
| `libs` | (logical name, directory as given, blake3 of the `lib.toml` bytes) per `-L` |
| `blobs` | (path as resolved at elaborate time, blake3 of the blob bytes) per consumed unit |
| `files` | (path, raw blake3) for every source and every include of every consumed unit |

Trailer ⑮ runs parallel to it (§1.7). Together they are what makes the RULE-V gate (§5.3) run
without any argv being re-supplied.

### 3.3 `velab -L` resolution

- `-L name[=dir]` is repeatable; `-L name` alone means the directory `./name`. There is no
  attached-value spelling, so `-Lfoo` is an unknown flag.
- Search order is `-L` order: the first library wins a duplicate unit name.
- At least one `--top` is required in library mode. A library's unrelated units must never become
  elaboration roots, so the loader walks the instantiation closure breadth-first from the named
  roots. A named item is emitted only from the compilation unit that the unit map resolves its
  name to, so a shadowed definition is skipped regardless of load order.
- A `--top` that no bound library defines is `E-ELAB-UNSUPPORTED` (`VITA-E3009`), exit 1.
- The global precision is the minimum over the loaded units, defaulting to the 1ns/1ns base.
- A positional `.vu` and `-L` libraries are mutually exclusive.

Status at HEAD: library mode drops the `.vu` source-map tail, so its elaborate-time diagnostics
carry no `file:line:col`. The merge splices items from several compilation units whose spans each
index their own expanded buffer from zero, so no single resolver can tell which unit a span
belongs to, and resolving through the wrong map would print a confidently wrong location. Plain
`velab <in.vu>` does locate.

### 3.4 Flag surface

| Flag | Stage | Meaning | Status |
|---|---|---|---|
| `--work <logical>[=<dir>]` | vcmp | logical work library (default name `work`) plus output directory | accepted |
| `--workdir <dir>` | vcmp | output directory when `--work` carries no `=<dir>`; alone it implies the name `work` | accepted |
| `-L <logical>[=<dir>]` | velab | bind a compiled library, repeatable | accepted |
| `--top <unit>` | velab, vita | elaboration root, repeatable; required in library mode | accepted |
| `-y`, `-Y` / `+libext+`, `-v <libfile>` | vcmp | source search directories and extensions for on-demand resolution | not accepted |
| `-P <dir>` | vcmp, velab | precompiled-library search directory | not accepted |
| `--lib-map <file>` | velab | external logical-name to directory map, with chaining and a search order | not accepted |

Invariants the design holds regardless of which of those exist: a consumer of an external library
hashes the consumed triples of that library, not only its own work library; the manifest content
digest invalidates on a unit being added, removed or renamed; and chaining between library maps is
a keyword inside the map file, never a command-line flag.

---

## 4. Filelists (`-f`, `-F`)

Large projects collect hundreds of source paths, include directories and defines into filelists
rather than typing them. A `.f` is tokenized like argv and spliced in place at its point of
reference, so every flag legal on the command line is legal inside a `.f`. All four applets accept
them, and expansion happens once, at argv level, before any per-applet parsing.

**Two flags, one difference — the base for relative paths.**

- `-f <file>` resolves relative paths inside that frame against the invocation working directory.
- `-F <file>` resolves them against the directory of the `.f` file itself, which makes a `-F` tree
  fully relocatable. This is why vendor IP ships with `-F`.
- The base is a property of how a frame was entered, not something inherited: each `-f` / `-F`
  token sets the base of the frame it opens. A command-line `-f` / `-F` target always resolves
  against the working directory; a nested target resolves against the enclosing frame's base.
- Absolute paths ignore the base. There is no `-c` synonym.

**Nesting.** A `-f` or `-F` line inside a `.f` splices another filelist at that position and
recurses to arbitrary depth. The flattened token stream is a depth-first pre-order traversal of
the inclusion tree, with the command line as frame 0. That is what gives a define three levels deep
a deterministic position in one flat stream.

```
expand(path, base_mode):
  canon = lexical_normalize(resolve(path, base_mode, invocation_cwd))   # symlinks are NOT resolved
  if canon ∈ active_stack OR phys_id(canon) ∈ active_phys:
      error E-FLIST-CYCLE (print the chain)
  push canon; push phys_id(canon)
  for tok in lex(read(canon)) in file order:
    if tok == `-f T`:   splice expand(T, cwd-relative)
    elif tok == `-F T`: splice expand(T, dir(canon)-relative)
    else:               resolve relative paths against this frame's base, emit a typed token
  pop canon; pop phys_id(canon)
```

**Cycle detection uses two keys.** Membership in the active stack is tested against the lexical
canonical path **and** the physical identity (device plus inode on Unix, the OS-canonical string
elsewhere). A symlink loop back onto a file's real path has a different lexical form, so a
lexical-only test would miss it and the run would die at the depth cap with a misleading
`E-FLIST-DEPTH` that prescribes the wrong fix. Physical identity is used for diagnosis only; it
never enters a hash or a manifest path, which stay lexical so that identical bytes give identical
digests on every platform.

**Canonicalization is locked for determinism, not for ergonomics.** After base resolution and
before dedup or hashing: paths are **not** case-folded — every platform treats a path as a
case-sensitive byte string, and a case collision surfaces as `E-FLIST-NOT-FOUND` — and symlinks
are **not** resolved, because identity, dedup and manifest paths use pure lexical `.` / `..`
normalization only. Break either and the same filelist over the same bytes yields a different
manifest digest per platform, which breaks RULE S and the byte-identity contract.

**Values are not paths.** Frame-base resolution applies to source positionals only. The token
following a value-taking flag is passed through byte-identical to its command-line form. The
canonical list of value-taking flags must match every value-taking arm of the argument parser:

```
-o  --out  --threads  -j  --backend  --timeout  -D  --define  -G  --param
-I  --incdir  -l  --log  --verbosity  --upstream  --work  --workdir
-L  --top  --obs-dir  --hier-tree  --inst-paths  --probe  --probe-file
```

`-f` and `-F` are deliberately absent from that list: the expander consumes their targets itself,
and those *are* paths that must resolve. Adding a new value-taking flag without adding it to this
list makes its value be rewritten as though it were a source path — a top name becomes an absolute
path and the run fails loudly, or an output path is silently redirected under the filelist's
directory.

**Syntax.** Comments are `//` to end of line, non-nesting `/* … */` (replaced by a single space so
they separate rather than join tokens), and a whole line whose trimmed form starts with `#`. A line
ending in `\` joins to the next. Whitespace and newlines separate tokens. Multi-value directives
join on `+` (`+incdir+a+b`, `+define+N=V+M`). Environment references are `$NAME`, `${NAME}` and
`$(NAME)`, applied to every token, to `-f` / `-F` targets, and to the value of a value-taking flag;
a lone `$` with no identifier body stays verbatim, for escaped-identifier paths.

**Wildcards are refused** (`E-FLIST-GLOB`): readdir order is not stable across platforms, which
would make the RULE S ordering nondeterministic. A generator that needs globbing emits an
explicitly ordered `.f`.

**Token classification inside a frame**, in order: `-f` / `-F` recurse; a value-taking flag and its
value pass through verbatim; `+define+` segments pass through verbatim, because they are macro text
and never paths; `+incdir+` segments each resolve against the frame base and re-join; any other
`-`-prefixed token passes through verbatim; anything else is a source path — glob-checked, then
resolved against the frame base.

### 4.1 Diagnostics

| Condition | Code | Mnemonic | Severity |
|---|---|---|---|
| a filelist reaches itself | `VITA-E8001` | `E-FLIST-CYCLE` | Error — prints the whole chain |
| nesting exceeds 256 levels | `VITA-E8002` | `E-FLIST-DEPTH` | Error — a backstop behind the cycle guard |
| a source appears twice under differing sticky context | `VITA-E8003` | `E-FLIST-DUP-CTX-CONFLICT` | Error |
| a token contains `*`, `?` or `[` | `VITA-E8004` | `E-FLIST-GLOB` | Error |
| an unreadable filelist or source, or a missing `-f` argument | `VITA-E8005` | `E-FLIST-NOT-FOUND` | Error |
| an undefined environment variable | `VITA-E8006` | `E-FLIST-UNDEF-ENV` | Error — never an empty-string substitution |
| a directive belonging to another stage | `VITA-E8007` | `E-FLIST-WRONG-STAGE` | Error |
| a `-f` line inside a `-F` frame | `VITA-W8008` | `W-FLIST-MIXED-BASE` | Warning |
| a single-value knob set in two places | `VITA-W8009` | `W-FLIST-OVERRIDE` | Warning |

`W-FLIST-MIXED-BASE` exists because re-anchoring a relocatable subtree onto the invocation working
directory is almost always a packaging mistake. The meaning is well defined, so it warns rather
than refuses.

**Wrong-stage is a hard error, never a silent no-op.** Each stage's expander parses the whole token
grammar, so the same `.f` parses everywhere, but a directive whose bucket does not belong to the
invoking stage is refused. Handing `velab` or `vrun` a filelist containing `+define+` is
`E-FLIST-WRONG-STAGE` (exit 3) with a message saying the directive is a compile-stage input —
those applets have no preprocess pass, so accepting it would mean discarding it in silence.

Status at HEAD: filelist expansion runs before the diagnostic gate is built, so filelist
diagnostics are ungated — `-Wno-W-FLIST-MIXED-BASE` does not suppress the warning, and a run that
dies during expansion prints no counts epilogue.

### 4.2 Conflicts and precedence

Multi-value directives accumulate: include directories form an ordered list, defines are last-wins
per name, sources append. A single-value knob set in more than one place is decided by the last
value in the flat stream, and command-line tokens are appended after every expansion, so the
command line overrides a `.f` while the rule stays one rule: last wins.

Every override records `W-FLIST-OVERRIDE` naming both values, both origins and the winner, so an
unintended collision is loud while the deliberate override workflow keeps working. The knobs that
record it are `-o` / `--out`, `--threads` / `-j`, `--backend`, `--timeout`, `--upstream`,
`--work`, `--workdir`, `-l` / `--log` and `--obs-dir`. Repeatable and accumulating flags do not,
and neither do the verbosity flags. Unlike the other filelist diagnostics these events are
replayed through the gated sink at pipeline start, so `-Wno-` and `-Werror=` apply to them and
they reach the counts epilogue.

A filelist's proper content is the source list and the include search path, plus defines and
library scoping. Putting a single-value elaborate knob in a `.f` is discouraged — build intent is
easier to trace on the command line — but it is not refused; a conflict surfaces as the warning
above.

### 4.3 Duplicate sources

The same canonical source appearing twice in an expansion dedups to its first occurrence. Flags
and their values are exempt; only bare positionals dedup, and identity is the physical id falling
back to the lexically normalized path.

Dedup alone would be unsound, because sticky-directive carryover (RULE S) means two occurrences of
one path sit in different inheritance contexts and are therefore not the same compilation input.
So whenever a duplicate was dropped, the pre-dedup stream is re-walked, tracking the sticky
`` `timescale `` state contributed by the kept files with a comment- and string-aware scanner. If
the dropped occurrence would have inherited a different directive from the kept one, that is the
hard error `E-FLIST-DUP-CTX-CONFLICT`, which names both inherited contexts (an absent context
renders as the base `1ns/1ns`).

### 4.4 `--dump-filelist`

A dry-run inspection of the effective inputs: it prints the post-expansion result and exits 0
without compiling, and it is checked first in every dispatcher so it short-circuits every
stage-specific rejection. Output goes to stdout, one line each, in this order:

```
source <path>          # each positional, in post-expansion argv order
define <NAME>          # a value-less define
define <NAME>=<VAL>    # a define with a value
incdir <dir>           # each -I / +incdir+ directory, in order
```

`W-FLIST-OVERRIDE` events are replayed through a gated sink first. There is no counts epilogue and
`--log` is not honoured here. The bucket is C.

---

## 5. The staleness gate

### 5.1 Gate order

The header is decoded alone; the body is untouched. Gates fire lowest-numbered first, so the most
fundamental mismatch is the one reported.

| Order | Condition | Code | Mnemonic | Message |
|---|---|---|---|---|
| 0 | magic missing or wrong, or the file is shorter than 8 bytes | `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | `bad or missing {velab\|VU} magic` |
| 0 | the header itself does not decode | `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | `undecodable {velab\|VU} header: {error}` |
| 1 | `format_version` differs from this build's | `VITA-E9001` | `E-ART-FORMAT-MISMATCH` | ``format_version={got} but this tool expects {want}; regenerate with `velab` `` |
| 2 | `tool_semver_major` differs | `VITA-E9004` | `E-ART-VERSION-GATE` | `produced by vitamin {got}.x, this tool is {want}.x; regenerate or install a matching vitamin` |
| 3 | `schema_hash` differs | `VITA-E9002` | `E-ART-SCHEMA-MISMATCH` | ``sim-ir type shape changed between builds; rerun `velab` `` |

`.vu` readers gate against `schema_hash::<hdl_ast::SourceUnit>()`; `.velab` readers gate against
`schema_hash::<sim_ir::SimIr>()`. Gate 3's message names sim-ir unconditionally, including when the
stale artifact is a `.vu` whose hdl-ast shape moved.

The fields that are **not** staleness keys: `composite_input_hash`, `consumed`,
`worklib_manifest_hash`, `uses_dump`, and every provenance field including git sha, dirty and
profile.

### 5.2 Body-decode failures

Past the header gate, every decode failure is also `E-ART-FORMAT-MISMATCH`, with a message naming
the exact segment: `undecodable .vu body`, `undecodable .vu timescale trailer`, `undecodable .vu
source-map trailer`, `undecodable .velab SimIr body`, and one message per `.velab` trailer — fork,
name, timescale, severity, radix, scope, assign-rank, queue-bound, work-consumed, net-dims,
final-procs, defer-marks, defer-acts, extra-sidecars and work-stamps. Naming the segment is what
turns a mid-file decode failure into an actionable report.

### 5.3 RULE V in practice — `E-ART-STALE-UPSTREAM` (`VITA-E9003`)

Two paths, both live, both content-hash only.

**Explicit: `vrun --upstream <file.vu>`.** Re-reads the live file, blake3-hashes it and compares
against the snapshot's recorded `composite_input_hash`. On a mismatch:
``{path}: digest changed since the .velab snapshot (rerun velab, or drop --upstream)``.

**Automatic: the work-library gate.** It runs on every `vrun` whose snapshot carries a non-empty
trailer ⑨. No flag enables it. It walks the recorded libs, blobs and files and checks each one:

| Entry | Changed | Unreadable |
|---|---|---|
| library manifest | ``work library `{name}`: {path} changed since the .velab snapshot (re-run velab)`` | ``work library `{name}`: {path}: {error} (re-run `vcmp --work` + velab)`` |
| unit blob | ``{path}: library blob changed since the .velab snapshot (re-run velab)`` | ``{path}: {error} (re-run `vcmp --work` + velab)`` |
| source or include | ``{path}: source changed since `vcmp --work` (re-run vcmp + velab)`` | ``{path}: {error} (re-run `vcmp --work` + velab)`` |

Editing only an included header trips the gate, because the include closure is recorded per
compilation unit alongside the sources. A stamp from trailer ⑮ may let an unchanged entry skip the
read and the hash; a stamp miss always re-hashes, so the content check is never bypassed by
mtime alone.

There is no `--rebuild`: a stale chain is repaired by re-running the stages the message names.

### 5.4 Exit codes

| Code | Constant | Meaning |
|---|---|---|
| 0 | `EXIT_OK` | clean run; also `--help`, `--version`, `explain`, `--dump-filelist` |
| 1 | `EXIT_USER_ERROR` | user or design error: lex, parse, elaborate, a runtime `$fatal`, or a `-Werror`-promoted warning on an otherwise clean run |
| 2 | `EXIT_STALE` | artifact or staleness rejection: bad magic, format, semver, schema, an undecodable body or trailer, a RULE-V mismatch, an invalid manifest |
| 3 | `EXIT_CLI_ERROR` | CLI or usage error: unknown flag, missing value, no sources, wrong positional count, unreadable input, unwritable output, a wrong-stage flag, a filelist error |

Exit 2 exists so that CI re-runs the upstream stages instead of debugging RTL. Every artifact
rejection — `.vu` and `.velab`, header and body, plus every RULE-V rejection and every manifest
rejection — returns it, through one function. A missing input file is not a staleness failure: it
is `VITA-E8005` and exit 3.

An artifact diagnostic is rendered without a location, since the failure is about the file as a
whole:

```
error[VITA-E9001] E-ART-FORMAT-MISMATCH: bad or missing velab magic
```

### 5.5 Remedy per gate

| Gate | Remedy |
|---|---|
| bad magic | the file is not a vita artifact, or is the wrong kind — regenerate with `vcmp` / `velab` |
| `format_version` | regenerate with `velab`; for a `.vu`, re-run `vcmp` and then `velab` |
| `tool_semver_major` | regenerate, or install a matching vitamin |
| `schema_hash` | re-run `velab` |
| RULE-V library or blob | re-run `velab`; re-run `vcmp --work` and `velab` when the path is unreadable |
| RULE-V source or include | re-run `vcmp` and `velab` |
| `--upstream` digest | re-run `velab`, or drop `--upstream` |
| manifest | the manifest is not in canonical form at the named line — let `vcmp --work` rewrite it |

### 5.6 Defensive fatals behind the gate

Because the trailers ride outside the schema gate, a hand-truncated `.velab` can pass the header
gate and reach the engine. Two engine guards re-use `E-ART-FORMAT-MISMATCH` as a runtime Fatal —
exit class 1, not 2, since the failure is discovered during simulation:

| Guard | Message |
|---|---|
| missing fork join-mode entry | `fork join-mode sidecar entry missing for (template={t}, join_bb={b}) — .velab trailer lost or stale; re-run velab` |
| missing declaration-initializer process | `declaration-initializer sidecar names process {pid}, which this IR does not have — .velab trailer lost or stale; re-run velab` |

A corrupted trailer must produce a clean diagnostic and a clean exit, never a panic or an abort.

### 5.7 Policy: refuse and rebuild

A `format_version`, `schema_hash` or semver-major mismatch is a hard error with an actionable
rebuild hint. There is no silent reuse and no migration machinery anywhere in the artifact crates:
because every artifact can always be regenerated from its sources, the policy is version-gate
rather than version-migrate. Migration becomes a question only if artifacts ever become a
distribution format.

---

## 6. Re-verification and one-shot equivalence

### 6.1 The `vrun` sequence

```
vrun <top>.velab:
  1. Decode the header alone: magic, format_version, tool_semver_major, schema_hash.
     A mismatch is a hard error with a rebuild hint — the body is never read.
  2. Decode the golden SimIr frame, then each trailer in order.
     A present-but-undecodable segment is a format mismatch naming that segment.
  3. If trailer ⑨ is non-empty, run the RULE-V gate over every recorded
     library manifest, unit blob, source and include:
       stamp hit (mtime and size match) → skip the read
       otherwise                        → read the bytes and blake3 them
     Any digest mismatch → E-ART-STALE-UPSTREAM, exit 2, with the fix in the message.
  4. If `--upstream <f.vu>` was given, re-hash that live file against the
     recorded composite_input_hash as well.
  5. All checks pass → hand the SimIr and the sidecars to the engine.
```

Mtime is a fast-path prefilter only. It is never the deciding signal.

The re-verification inputs come from the artifact itself, not from re-supplied argv. The header
carries irreversible digests and does not keep the original command line, so the recorded library
manifests are the primary source: each names its own sources, includes, defines and include
directories with their digests. A bare `vrun <top>.velab` therefore works as long as the recorded
library directories are still where the snapshot says they are.

### 6.2 What must be identical

The staged chain and the one-shot run must produce the same observable output for the same inputs:

| Observable | Requirement |
|---|---|
| `$display` transcript | byte-identical |
| waveform file | byte-identical |
| diagnostic lines, including `file:line:col` and the instance path | byte-identical |
| exit class | identical |

Equivalence is defined on observable output, not on internal representation. The one-shot path
never serializes: it builds the engine's options directly from the elaborator's in-memory sidecar
bundle. The staged path must therefore carry *every* engine-facing sidecar across the `.velab`
boundary. Anything dropped is the staged-drop hazard: the staged run silently produces a different
answer from the one-shot run on the same design, which is the exact failure the whole trailer
mechanism exists to prevent.

**Global precision equivalence.** The design-wide precision is the minimum over the effective
precision of every consumed unit. When no consumed unit specifies a timescale, the base `1ns/1ns`
applies and `W-PP-TIMESCALE-DEFAULT` (`VITA-W1017`) is emitted; the base is a constant, so it
breaks neither the recorded precision nor byte-identity. For the two paths to agree, the same argv
must produce the same set of consumed units and the same minimum over it. The consumption record
of §3.2 is the canonical definition of "the design", and both paths refer to it.

### 6.3 Sidecars carried on both paths

These are threaded identically by the one-shot driver and by the staged loader:

`fork_modes`, `net_names`, `proc_multipliers`, `proc_prec_mults`, `global_prec_exp`,
`timescale_unit`, `severities`, `stmt_locs`, `stmt_scopes`, `expr_scopes`, `proc_scopes`,
`proc_inst_scopes`, `radixes`, `assign_ranks`, `queue_bounds`, `net_dims`, `net_decl_ranges`,
`timeformat_stmts`, `handle_copy_stmts`, `queue_slice_stmts`, `file_directed_stmts`, `init_procs`,
`final_procs`, `clocking_inputs`, `clocking_commit`, `clocking_outputs`, `ca_delays`,
`defer_marks`, `defer_acts`, `func_table`, `func_names`, `task_calls_proc`, `task_calls_func`,
`two_state_nets`, `real_elem_dyn_nets`, `string_elem_dyn_nets`, `wired_and_nets`, `wired_or_nets`,
`class_handle_nets`, `class_new_sites`, `class_layouts`, `class_field_inits`, `class_rand`,
`class_constraints`, `class_dist`, `class_randc`, `randomize_with`, `class_vtable`, `class_calls`,
`class_field_widths`, `assert_fire`, `assert_ctl`.

### 6.4 One-shot only

The observability rail is a one-shot capability. The staged applets do not accept-and-drop its
flags; they refuse them, so the difference is loud rather than silent:

| Capability | One-shot | Staged |
|---|---|---|
| `--obs-dir` (`run.json`, `results.jsonl`, `coverage.json`) | supported | refused on `vcmp`, `velab`, `vrun`, exit 3 |
| `--probe` / `--probe-file` (`trace.jsonl`) | supported | refused, exit 3 |
| `--obs-procs` / `--obs-procs-time` (the `run.json` process objects) | supported | refused, exit 3 |
| `$vita_stage` (`stage.jsonl`) | supported | `velab` refuses a design that uses it, exit 3 |

Each refusal message names the stage and says where the flag belongs. `--hier-tree` and
`--inst-paths` are accepted by the staged applets and do nothing, which is the one place the rail
is quiet rather than loud.

The other per-stage refusals follow the same principle — a flag whose bucket does not match the
invoking stage is refused, never dropped: preprocess buckets on `velab` and `vrun`, runtime
plusargs on `vcmp` and `velab`, `--backend` on `vcmp` and `velab`, `-G` on `vcmp` and `vrun`,
`--work` / `--workdir` outside `vcmp`, and `-L` / `--top` outside their own stages.

### 6.5 Promoted warnings produce no artifact

`vcmp` and `velab` check for an error or fatal *before* writing anything, so a `-Werror`-promoted
warning fails the stage and leaves no artifact behind to be reused. `vrun` turns an otherwise clean
exit into exit 1 when a promoted warning fired, matching the one-shot behaviour
([13-diagnostics-and-logging.md](13-diagnostics-and-logging.md)).

### 6.6 Output naming and the clobber guard

| Applet | Default output |
|---|---|
| `vcmp` | the first source with its final extension replaced by `vu`; omitted entirely when `--work` is given without `-o` |
| `velab` (positional) | the input with its final extension replaced by `velab` |
| `velab -L` | `<first --top>.velab` |
| `vrun` | none; `-o` is a waveform override |

Only the final extension component is replaced, so `a.b.sv` becomes `a.b.vu`. Before writing, the
resolved output is compared against every positional input — by string equality, and by
canonicalizing both when both exist, so that `./a.sv` against `a.sv` and symlinked aliases are
caught. A match is refused: `output '<out>' would overwrite an input file`, exit 3.

---

## 7. `format_version` discipline

`CURRENT_FORMAT_VERSION` is **31**. It is the container format version, shared by `.vu` and
`.velab`, and it guards the on-disk wire layout: the header field layout, and everything in the
out-of-band trailer and tail segments that the schema hash cannot see. The schema hash covers only
the type shape of the golden frame.

**What a bump costs.** Every existing `.vu` and `.velab` refuses to load, with
`E-ART-FORMAT-MISMATCH` at the header gate, before any body byte is deserialized. The refusal is
the point: an artifact written under a different wire layout must meet a clean, actionable error
rather than a mid-file postcard failure or, worse, a successful mis-decode.

**When a change needs a bump.**

| Change | Bump? | Why |
|---|---|---|
| a field added, removed, reordered or retyped in any trailer or tail | yes | postcard is positional, so the bytes move |
| an element type change that alters encoding, such as `(u32,u32)` to `(i64,u32)` | yes | a stale artifact would mis-decode rather than fail |
| a new variant appended **last** to a trailer enum | yes | old values still decode, but an old binary must meet the header gate rather than the decoder |
| a header field added or reordered | yes | the header layout is exactly what the constant guards |
| a field renamed inside a trailer struct | no | postcard encodes fields positionally |
| widening a trailer `Vec<u32>` to `Vec<u64>` | no | varint encoding is identical below 2^32; the equivalence is pinned by a test |
| a frozen `sim-ir` type shape change | yes, plus a schema-hash re-pin | the golden root hash flips, and both goldens are regenerated |
| an engine-facing table synthesized from run options or from elaborate output | no | it rides out of band and never touches the golden shape |

**What is not versioned by it.** The work-library manifest carries its own `format_version = 1`,
unrelated to the container number. The `sim-ir` golden hash is unchanged across the recent
container versions, all of which are trailer or tail changes only; the two numbers are not the same
thing and must not be read as one.

The version-by-version record lives in the doc comment on `CURRENT_FORMAT_VERSION` in
`crates/vita-artifact/src/header.rs`, and in [../history/README.md](../history/README.md). This
document states the format as it is.

---

## 8. The schema-hash machinery

Three layers, none of which needs a build script or a code generator; `cargo` stays the only build
entry point. Full specification: [16-schema-hash-spec.md](16-schema-hash-spec.md); the frozen IR
backbone: [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md).

**Layer 1 — the structural schema hash (the runtime staleness and decode key).** A proc-macro
crate provides `#[derive(SchemaHash)]`. A proc-macro runs inside rustc, so it is fully
cargo-native. The derive walks the type's own body — field names, field types, variant shapes and
the serde attributes that affect the wire — into a canonical shape string, and registers children
by name rather than inlining them. A runtime registry walks the whole reachability closure, renders
it deterministically and blake3-hashes the result. The `.vu` root is `hdl_ast::SourceUnit`; the
`.velab` root is `sim_ir::SimIr`. Adding, removing, reordering or retyping a field or a variant
flips the hash, so a tool built against an incompatible type shape reports a clean "rebuilt with an
incompatible tool" error at decode time instead of mis-parsing in silence.

Determinism is a hard requirement of that layer: the registry uses ordered containers only, never
a hash-ordered one; the canonical string is sorted by fully-qualified type name; the line
terminator is a literal newline, never CRLF; and the canonical string and its blake3 must be
byte-identical on every platform and toolchain.

The canonical key is `module_path!()::Ident`, expanded in the module where the derive is written.
Moving a `SchemaHash` type into a submodule therefore changes its key, changes the canonical
string and flips the root hash, invalidating every artifact on disk. That is why the frozen sim-ir
types and every hdl-ast type live at their crate root.

Serde attributes participate in the shape because they change the wire bytes without changing the
Rust shape. They are collected, sorted by a fixed priority so that source order does not matter,
and rendered into the shape string.

**Layer 2 — the build fingerprint.** Tool version, git sha, dirty flag and profile are stamped into
every header for traceability. Not a staleness key, by design (§1.3).

**Layer 3 — the wire golden.** A `serde-reflection` registry of the frozen types is committed as a
golden file and re-traced by the test suite, so serde wire drift that the syn-level shape walk
cannot observe fails a test rather than corrupting artifacts. Layer 1 gates runtime staleness;
Layer 3 gates deliberate human review of a format change. Together they cover shape edits and wire
edits. The golden's newline is pinned explicitly, because the default pretty-printer picks the
platform newline and a byte-compared golden cannot survive that.

---

## 9. Runtime diagnostic locations

`sim-ir` is span-free by design (D4): it holds no `Span` type and no `span` field, and knows
neither Verilog syntax nor file paths. A runtime diagnostic — a `$fatal` at a particular line, a
`unique` violation, a deferred assert — must still point at source, so location travels on a
separate path, the way an RTLIL-style `src` attribute overlays location onto a neutral netlist.

How it works at HEAD:

- The `.vu` carries the source-map tail (§1.4), so `velab` rebuilds the same resolver the one-shot
  driver uses and staged elaborate diagnostics locate identically.
- Elaborate resolves each recorded statement to `{ file, line, col, byte_start, byte_end, instance }`
  **once**, and stores it in trailer ⑭ field `stmt_locs`. Because the resolution happens at
  elaborate time, the one-shot and staged runs are identical by construction rather than by
  parallel implementation.
- `stmt_scopes`, `expr_scopes` and `proc_inst_scopes` carry the `%m` scope chains that the same
  reports need.
- The engine consumes these when rendering a runtime diagnostic, so a runtime report gets the same
  caret rendering as a compile-time one ([13-diagnostics-and-logging.md](13-diagnostics-and-logging.md)).

The table is sparse by construction: entries exist for severity statements (`$fatal`, `$error`,
`$warning`, `$info`), `unique` and `priority` violations, deferred asserts, statements that index
an array net, and the file-directed memory tasks. Since the engine's IR cannot re-derive a location
at run time, this record is the only route by which a runtime diagnostic prints
`file:line:col [in instance]`.

Status at HEAD: there is no strip option and no load-time warning about a location-less snapshot.
Locations always ride the artifact. The one place they are absent is `velab -L` library mode
(§3.3), where the source-map tail is dropped; the loss is confined to elaborate-time diagnostics
and is refused a wrong answer rather than given a guessed one. The design intent — a separately
versioned side table, strippable for a release snapshot, with a one-time load warning when it is
absent — is not implemented.

---

## Sources

- [03-build-and-portability.md](03-build-and-portability.md) — cargo-only build, MSRV, platform matrix, workspace
- [04-architecture.md](04-architecture.md) — execution model, pipeline, crate roles, IR design principles
- [06-simulation-engine.md](06-simulation-engine.md) — the process execution model behind the frozen shape
- [08-timescale-and-timing.md](08-timescale-and-timing.md) — global precision minimum, directive carryover
- [09-testing-and-verification.md](09-testing-and-verification.md) — round-trip and equivalence tests
- [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) — gated sink, `-Wno-` / `-Werror`, runtime diagnostic rendering
- [15-error-code-reference.md](15-error-code-reference.md) — the E8xxx and E9xxx bands in full
- [16-schema-hash-spec.md](16-schema-hash-spec.md) · [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) — the hash mechanism and the frozen IR
- [19-ai-agent-observability.md](19-ai-agent-observability.md) — the one-shot observability rail
- [../manual/004_cli-reference.md](../manual/004_cli-reference.md) · [../manual/007_error-codes.md](../manual/007_error-codes.md) — the user-facing surface
- [../history/specs/2026-05-26-vitamin-rtl-simulator-design.md](../history/specs/2026-05-26-vitamin-rtl-simulator-design.md) — the originating design
- Prior art: Cadence `xmvlog` / `xmelab` / `xmsim` with `cds.lib`; Synopsys `vlogan` / `vcs` / `simv` with `synopsys_sim.setup`; Icarus `iverilog` → `.vvp` → `vvp`; GHDL `-a` / `-e` / `-r`; Yosys RTLIL `src` attributes; the rustc strict version hash as a version-gate model
