# Contributing to vitamin

Thanks for your interest in **vitamin**, an open-source Rust RTL simulator. This
guide covers the toolchain, the determinism rules that keep builds reproducible,
and the contribution workflow.

## Toolchain & build

- **Rust 1.82** is the MSRV, pinned by `rust-toolchain.toml` (rustup picks it up
  automatically — no manual `rustup override` needed).
- **Edition 2021** (not 2024 — that requires rustc ≥ 1.85 and would break the MSRV).
- Always build and test with `--locked` for 3-OS reproducibility:

  ```sh
  cargo build  --workspace --locked
  cargo test   --workspace --locked
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo fmt --all -- --check
  ```

- `blake3` is pinned to `=1.8.2` (1.8.3+ moves to edition 2024). Keep the pin.

## Determinism rules (load-bearing)

vitamin guarantees byte-identical output across Linux/macOS, enforced by a frozen
IR and golden-hash gates. Respect these or CI will (correctly) reject the change:

- The `sim-ir` serialized types are **frozen** by a structural `SchemaHash` gate.
  Adding, removing, or reordering a field flips the golden root hash and
  invalidates every `.velab` artifact. Change a frozen type only deliberately,
  with a `format_version` bump and the golden hash re-pinned in the same commit.
- No `usize`/`isize`/`f32`/`f64` in frozen types; collections are `BTree`-only and
  span-free (these are the cross-OS determinism requirements).
- `sim-ir` cross-type fields use the fully-qualified `sim_ir::Foo` spelling; a
  guard test (`tests/body_refs.rs`) rejects bare references.

## Tests

- TDD is the norm: write the failing test, make it pass, run the full gate, commit.
- The differential harness (`crates/sim-engine/tests/differential.rs`) diffs
  representative designs against Icarus Verilog (`iverilog` + `vvp`); it skips
  gracefully when those tools are not on `PATH`.
- Add a regression test for every fix and a focused test for every feature.

## Pull requests

- Keep the full gate green (build / test / clippy / fmt, all `--locked`).
- Commit messages use the prefix convention: `Add` / `Fix` / `Update` / `Refactor`
  followed by a concise subject.
- Keep changes scoped; do not commit secrets or local `.env` files.

## Platforms

Linux and macOS are supported and CI-tested (Ubuntu, macOS, RHEL9). Windows is not
currently a target — contributions adding Windows support are welcome but out of
the current Phase-1 scope.

## Where things live

- `crates/` — the 17-crate workspace (front-end, elaborate, engine, artifacts).
- `docs/manual/` — the user-facing manual (start at `000_introduction.md`).
- `docs/preview/` — the authoritative design SPEC (single source of truth).
- `docs/REMAINING_WORK.md` — the living engineering tracker (Phase-2 items live here).
