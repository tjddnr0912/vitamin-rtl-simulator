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
//! 5. **The digest must MOVE when the design changes.** A digest that survives a
//!    mutation of the RTL is not gating anything, and it looks exactly like one that
//!    is. `picorv32` failed this: its program was a loop of adds that never stored,
//!    so no computed value ever reached the bus the digest watches, and inverting
//!    the core's adder left the digest byte-identical. Every workload has since been
//!    checked by mutating one line of its upstream RTL — but note that a *symmetric*
//!    mutation can be dead for an honest reason (in the `verilog-ethernet` loopback,
//!    TX and RX share one `lfsr` instance, so changing the CRC polynomial changes
//!    both ends and cancels; the RX-side datapath mutation moves it).

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
