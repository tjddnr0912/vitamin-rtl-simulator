//! corpus-runner — the **workload corpus**: real RTL, fetched at a pinned commit,
//! run under `vita`, checked against a digest an external oracle produced.
//!
//! This is a dev/test tool (`publish = false`), not a shipped binary.
//!
//! # Why a workload corpus exists
//!
//! Every performance decision this project has made rests on the designs it could
//! measure, and for a long time that was two: `picorv32` (third-party) and
//! `bench/keccak` (first-party, written *for* the measurement). A ceiling computed
//! from one design is a property of that design. The corpus exists so that the next
//! "this is worth N weeks" claim is priced against RTL nobody here wrote.
//!
//! # The contract every workload obeys
//!
//! 1. **Permissive licence.** MIT / BSD-2 / BSD-3 / ISC / Apache-2.0 only. The RTL is
//!    never redistributed — `bench/*` is gitignored and [`fetch`](crate::plan_fetch)
//!    clones it at the pinned SHA on the machine that runs it.
//! 2. **An oracle ran it first.** A workload with no oracle is not admitted, however
//!    interesting it looks. `iverilog` is the reference; `verilator` is a second
//!    opinion on 2-state arithmetic only.
//! 3. **One digest line, accumulated over the whole run.** Not final state — a
//!    final-state comparison is blind to any divergence the design later overwrites.
//!    The pinned [`Workload::digest`] is what the oracle printed, so `run` is a
//!    differential gate on a machine that has no oracle installed (CI has none).
//! 4. **Deterministic and self-terminating.** Explicit `$finish`, fixed seeds, and a
//!    watchdog, so a workload can never hang the harness.

#![forbid(unsafe_code)]

mod corpus;
mod fetch;
mod run;

pub use corpus::{coverage, Expect, Origin, Shape, Workload, CORPUS};
pub use fetch::{plan_fetch, FetchStep};
pub use run::{
    grade, job_for, measure, prepare_iverilog, resolve_bench_root, Grade, Job, Measurement,
    Outcome, Tool,
};
