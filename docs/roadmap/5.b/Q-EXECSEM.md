(§5.b Q-EXECSEM · OPEN) (census of the operations each executor spells on its own, no code):
* scope: measure only, any track: which expression and statement operations are spelled in `sim-engine/src/eval/`, `native_eval/compile.rs`, `native/wprog.rs` and `jit.rs` (e.g. `UnOp::RedXor` is matched in `eval/eval_core.rs`, `native_eval/compile.rs` and `native/wprog.rs`)
* symptom: executor semantics live in 3–4 places, test-mitigated: exec/ + eval/ 6,755 lines, native_eval/ 2,121 plus the `backend.rs` VM, native/ 11,667, `jit.rs` 1,307
* evidence: d53774d7 (the VM drifted from the interpreter "in four silent ways")
* evidence: CONTRIBUTING.md:258
* evidence: `sim-engine/tests/backend_equiv.rs` is the sync
* fix: finish the Kernel pattern `sim-engine/src/native/kernel.rs:1-10` describes — statement and expression MEANING generic over `K: Kernel` (`compute_effect` / `apply_effect` already are), the executors differing only in storage and dispatch
* note: size: large, so the census comes first
* status: OPEN
* source: W/review/structure-2026-11-04/REPORT.txt S7 item 5
