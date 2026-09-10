# `$strobe` / `$monitor` POSTPONED-REGION Implementation Spec (sim-engine)

> **Status:** implementation-ready. No frozen-IR change, no new diag codes.
> **Scope:** `crates/sim-engine` only. `SysTaskId::Strobe` and `SysTaskId::Monitor`
> already exist in the frozen IR (`crates/sim-ir/src/lib.rs:210-211`); this spec
> wires their *runtime* semantics into the scheduler's postponed region.
> **Author role:** scheduler architect. **Date:** 2026-06-04.

---

## 0. Executive summary

Today `builtins::dispatch` renders `$monitor`/`$strobe` *immediately, inline, like
`$display`* (`builtins.rs:43-50`). That is the v1 placeholder and is IEEE-wrong:
both tasks must fire in the **postponed region** — the read-only end-of-timestep
region that runs only after Active/Inactive/NBA reach quiescence (IEEE 1364-2005
§5.4). The deferral is what makes `$strobe(q)` after `q <= d` show the *post-NBA*
value.

The implementation has four moving parts, all inside `sim-engine`:

1. **New per-run state** on `SimState` (`state.rs`): a FIFO of pending strobe
   captures (cleared every timestep) + a single optional monitor record (global
   singleton) holding its last-evaluated 4-state **value** list for IEEE-correct
   (value-level, not rendered-string) change detection.
2. **`builtins.rs` rewrite of the `Monitor | Strobe` arm**: `$strobe` *registers*
   a capture (no print now); `$monitor` *replaces* the singleton and resets its
   last-value baseline (`last_vals = None`) to force a print at the next postponed
   flush.
3. **A `Scheduler::flush_postponed()` method** called at the exact settled point in
   `run()` — immediately after `break; // time-step stable`, before time advance.
   It drains strobes in call order, then renders the monitor and prints only on
   change.
4. **Reuse of `format_args_str(&Scheduler, ...)`** verbatim as the rendering
   surface — it already takes `&Scheduler` and re-evaluates ExprIds, so the same
   evaluation surface (`EvalCtx` built from `self.st`) is available at flush time.

Captures store `(Option<u32> fmt, Vec<u32> args)` — the *same ExprIds* the IR
holds — **not** pre-evaluated values. ExprIds index the immutable `ir.exprs` and
stay valid for the whole run, so re-evaluation at flush time is well-defined and
reads settled end-of-timestep net values.

---

## 1. New per-run state

### 1.1 Where it lives

On **`SimState<'a>`** (`crates/sim-engine/src/state.rs:27-50`), mirroring the
existing `dump_pending_path: Option<String>` per-run scratch field. This is the
correct home because:

* `format_args_str(sched, ...)` and `sched.eval(eid)` only need `&Scheduler` to
  reach `sched.st`; the flush helper evaluates + renders first (`&self`) then
  records the new value baseline (`&mut self.st`) — **render-then-record**, no
  borrow conflict.
* `builtins::dispatch` already writes per-run state via `sched.st.<field>`
  (e.g. `sched.st.dump_pending_path = Some(name);` at `builtins.rs:55`), so the
  `$strobe`/`$monitor` arms write `sched.st.postponed.<field>` identically.

### 1.2 The types (add to `state.rs`)

`MonitorState` holds `Vec<Value>`, so `state.rs` must reach the runtime value
type. `Value` is already defined in `crate::value` (`value.rs:34`,
`#[derive(Debug, Clone, PartialEq, Eq)]`); add `use crate::value::Value;` to the
top of `state.rs` if not already imported.

```rust
/// One captured `$strobe`/`$monitor` argument list. Stores ExprIds (not values)
/// so the args are RE-EVALUATED at postponed-flush time, sampling settled
/// end-of-timestep net values. ExprIds index the immutable `ir.exprs` and remain
/// valid for the whole run, so no value snapshot or scope context is needed:
/// `EvalCtx` is rebuilt from `Scheduler::st` (ir / nets / now / wt) at flush.
#[derive(Clone)]
pub(crate) struct FmtCapture {
    /// `SysTask.fmt`: Option<ExprId> → a `Const{val}` whose `val` is the
    /// format-string ConstId. `None` ⇒ bare-args (space-joined decimals).
    pub fmt: Option<u32>,
    /// `SysTask.args`: the argument ExprIds, evaluated lazily in `format_args_str`.
    pub args: Vec<u32>,
}

/// The single global `$monitor` record (IEEE 1364-2005: at most one active
/// monitor list in the entire simulation). A later `$monitor` REPLACES this.
pub(crate) struct MonitorState {
    pub cap: FmtCapture,
    /// Last evaluated 4-state VALUE list of `cap.args` (one `Value` per arg).
    /// `None` ⇒ never printed yet, so the next postponed flush prints
    /// unconditionally (establishment print). IEEE 1364-2005 §17.1 keys $monitor
    /// reprints off the *monitored expression VALUE* changing, NOT off the
    /// rendered string. `Value` derives `PartialEq`/`Eq` over the `(val, unk)`
    /// bit-planes, so equality is exactly 4-state-aware: an X/Z-collapsing format
    /// spec (`%d` rendering any-unknown to "x", `%h`/`%b` collapsing a
    /// partial-unknown group) can NEVER mask a genuine value transition the way a
    /// rendered-string diff would (e.g. `4'b00xx → 4'b0x00` under `%d`, both
    /// printing "x", is correctly detected as a change here).
    pub last_vals: Option<Vec<Value>>,
    /// `$monitoroff` clears this; `$monitoron` re-sets + resets `last_vals` to
    /// force a print. DEFERRED for the MVP (no SysTaskId bound) — field present,
    /// always `true`, so the flush logic is already on/off-aware when the tasks
    /// land.
    pub enabled: bool,
}

/// Per-timestep postponed-region queue + the global monitor singleton.
#[derive(Default)]
pub(crate) struct Postponed {
    /// FIFO of pending strobes for the CURRENT timestep. Drained-and-CLEARED at
    /// every postponed flush (one-shot-per-call semantics).
    pub strobes: Vec<FmtCapture>,
    /// The global monitor (replace-on-redefine). `None` until first `$monitor`.
    pub monitor: Option<MonitorState>,
}
```

`MonitorState` deliberately has **no** `#[derive(Default)]`; `Postponed` derives
`Default` because `Vec::default()` + `Option::default()` (= `None`) is exactly the
empty initial state.

### 1.3 The `SimState` field + initializer

Add one field to the `SimState` struct (after the status flags block, around
`state.rs:50`):

```rust
    // ── postponed region ($strobe FIFO + global $monitor singleton) ──
    pub postponed: Postponed,
```

And one line in `SimState::new` (in the struct literal, `state.rs:78-94`,
alongside `dump_pending_path: None,`):

```rust
            postponed: Postponed::default(),
```

That is the entire state-plumbing diff: one struct field + one initializer line +
the three type defs. No constructor argument, no `SimOpts` change.

---

## 2. `builtins.rs` change

Replace the placeholder `Monitor | Strobe` arm (`builtins.rs:43-50`) with two
distinct arms. Neither prints now; both *capture ExprIds*.

### 2.1 Before (verbatim, `builtins.rs:43-50`)

```rust
        // v1: $monitor/$strobe render immediately like $display (Monitor-region
        // scheduling deferred).
        SysTaskId::Monitor | SysTaskId::Strobe => {
            let mut s = format_args_str(sched, fmt, args);
            s.push('\n');
            write_out(sched.st, &s);
            Ctl::Continue
        }
```

### 2.2 After

```rust
        // $strobe: REGISTER a postponed capture (does NOT print now). It is
        // rendered with settled end-of-timestep values at `flush_postponed`,
        // then cleared (one-shot per call). Multiple strobes in one step print
        // in call order (FIFO push).
        SysTaskId::Strobe => {
            sched.st.postponed.strobes.push(FmtCapture {
                fmt,
                args: args.to_vec(),
            });
            Ctl::Continue
        }
        // $monitor: REPLACE the global singleton (IEEE: at most one active
        // monitor in the whole sim). `last_vals = None` forces an establishment
        // print at the next postponed flush of THIS timestep, seeding the
        // baseline value list.
        SysTaskId::Monitor => {
            sched.st.postponed.monitor = Some(MonitorState {
                cap: FmtCapture {
                    fmt,
                    args: args.to_vec(),
                },
                last_vals: None,
                enabled: true,
            });
            Ctl::Continue
        }
```

Add the imports at the top of `builtins.rs` (next to the existing
`use crate::state::{vcd_var_type, SimState};`):

```rust
use crate::state::{vcd_var_type, FmtCapture, MonitorState, SimState};
```

### 2.3 Why ExprIds, and confirmation the EvalCtx is reconstructable at flush

* `args.to_vec()` clones the `&[u32]` slice the executor already holds
  (`exec.rs:46-47` passes `&args` from the cloned `Stmt::SysTask { which, fmt,
  args }`). `fmt: Option<u32>` is `Copy`. No value evaluation happens here.
* At flush time the rendering surface is `format_args_str(self, cap.fmt,
  &cap.args)` where `self: &Scheduler`. `format_args_str` builds the evaluation
  context exactly as `$display` does: `sched.eval(eid)` (`builtins.rs:193,239,312`)
  internally constructs `EvalCtx { ir: self.st.ir, nets: self.st, now:
  self.st.now, wt: &self.st.wt }` (`sched.rs:366-373`). Because the flush runs
  *after* the settle break, `self.st.now` is the just-settled time and every net
  holds its settled value, so the net reader (`impl NetReader for SimState`,
  `state.rs:261-267`) returns end-of-timestep values. **No scope/process context
  is needed** — vita's expression eval is global-net-table-based, not
  per-process-frame.

---

## 3. Scheduler change: `flush_postponed()` + the call site

### 3.1 Exact insertion point in `run()`

The inner drain loop (`sched.rs:178-236`) exits only via `break; //
time-step stable` (`sched.rs:235`), reached when cont-assigns are at a fixpoint and
Active, Inactive, **and** NBA are all empty — i.e. the timestep is fully settled
and time has not yet advanced. Insert the flush **between line 236 (`}` closing the
inner loop) and line 238 (the `// Advance time` comment)**.

#### Before (verbatim, `sched.rs:234-239`)

```rust
                continue;
                }
                break; // time-step stable
            }

            // Advance time to the next scheduled tick.
            let next = match self.wheel.keys().next().copied() {
```

#### After

```rust
                continue;
                }
                break; // time-step stable
            }

            // POSTPONED REGION (IEEE 1364-2005 §5.4): now == settled time, all
            // region buckets (Active/Inactive/NBA) empty, cont-assigns at
            // fixpoint, time NOT yet advanced. Reads settled `cur` net values.
            // Drain strobes (call order) then the monitor (print-on-change).
            self.flush_postponed();

            // Advance time to the next scheduled tick.
            let next = match self.wheel.keys().next().copied() {
```

### 3.2 The method (add to `impl Scheduler`, near `apply_nba`, `sched.rs:271`)

```rust
    // ── postponed region ($strobe FIFO drain + $monitor change-detect) ─────

    /// IEEE 1364-2005 §5.4 postponed region. Called at the settled point of each
    /// timestep (Active/Inactive/NBA empty, cont-assigns at fixpoint, `now` =
    /// settled time, time NOT yet advanced). Read-only w.r.t. net state: it only
    /// EVALUATES ExprIds (reading settled `cur` net values via `NetReader`) and
    /// writes to `self.st.out`. NOTE: this flush has nothing to do with
    /// `prev`/`snapshot_prev` — those are edge-detection state that `run()` rolls
    /// at the *start of the next* timestep (after time advance); the postponed
    /// render reads only the settled `cur` values and is independent of edge state.
    ///
    /// ORDER (frozen, documented for the golden gate): all strobes first in call
    /// order, then the single monitor line. IEEE leaves this tie-break
    /// implementation-defined; vita freezes strobes-then-monitor for byte-stable
    /// 3-OS golden output.
    fn flush_postponed(&mut self) {
        // (a) STROBES — drain the FIFO in call order, render NOW (settled values),
        //     print, then CLEAR (one-shot per call: a strobe never repeats next
        //     step unless its statement re-executes and re-registers).
        if !self.st.postponed.strobes.is_empty() {
            // `mem::take` first to end the immutable borrow that `format_args_str`
            // needs against `&self` (mirrors `apply_nba`'s `mem::take(&mut nba)`).
            let batch = std::mem::take(&mut self.st.postponed.strobes);
            for cap in &batch {
                let mut line = format_args_str(self, cap.fmt, &cap.args);
                line.push('\n');
                write_out(self.st, &line);
            }
            // `batch` dropped here; `self.st.postponed.strobes` is now empty.
        }

        // (b) MONITOR — IEEE 1364-2005 §17.1: reprint whenever any monitored
        //     expression VALUE changes (4-state-aware), NOT when the rendered
        //     string changes. We therefore evaluate the arg ExprIds to a
        //     `Vec<Value>` and compare against the stored baseline with
        //     `Value`'s derived `PartialEq` (exact `(val, unk)` bit-plane
        //     equality). Only when the value list differs (or was never seeded:
        //     establishment / replace) do we render + print and re-seed.
        //
        //     Borrow shape: hoist EVERYTHING out of the `&self.st.postponed`
        //     borrow into locals (copy `enabled`, copy `fmt`, clone `args`,
        //     `take` the previous `last_vals`) and DROP that borrow before any
        //     `&self`-eval / `&mut self.st`-write. No NLL dependence on a binding
        //     surviving across the render — render-then-record, zero overlap.
        //     `Option::take` is used so the old baseline is moved out (not
        //     cloned) and the slot is rewritten unconditionally below.
        let mon = match self.st.postponed.monitor.as_mut() {
            Some(m) if m.enabled => {
                let fmt = m.cap.fmt;
                let args = m.cap.args.clone();
                let prev = m.last_vals.take(); // moves baseline out; slot now None
                Some((fmt, args, prev))
            }
            // disabled (`$monitoroff`) or no monitor established → nothing to do.
            _ => None,
        };
        if let Some((fmt, args, prev)) = mon {
            // No-arg monitor (`$monitor;` → fmt=None, args=[]) prints nothing —
            // not even a bare newline. Guarded so a future bare-`$monitor` lowering
            // cannot silently inject a lone "\n" into golden RTL output (see §7.4).
            if fmt.is_none() && args.is_empty() {
                // Seed an empty baseline (`[]` == `[]` keeps it silent forever)
                // and emit NO line; a zero-expression monitor has no value to
                // track. Early-return is safe: strobes (a) already drained and the
                // monitor block is the final action of `flush_postponed`.
                if let Some(m) = self.st.postponed.monitor.as_mut() {
                    m.last_vals = Some(Vec::new());
                }
                return;
            }
            // Evaluate every monitored expression to a settled 4-state Value.
            // `self.eval` builds `EvalCtx` from `self.st`, reading settled `cur`.
            let cur_vals: Vec<Value> = args.iter().map(|&eid| self.eval(eid)).collect();
            let changed = match &prev {
                None => true,                 // establishment / replace → print
                Some(old) => *old != cur_vals, // 4-state value-level change
            };
            if changed {
                let mut line = format_args_str(self, fmt, &args);
                line.push('\n');
                write_out(self.st, &line);
            }
            // Re-seed the baseline with the freshly-evaluated values regardless of
            // whether we printed: an unchanged step must keep the same baseline,
            // and a printed step adopts the new one.
            if let Some(m) = self.st.postponed.monitor.as_mut() {
                m.last_vals = Some(cur_vals);
            }
        }
    }
```

`format_args_str` and `write_out` are `fn`s in `builtins.rs`; make them reachable
from `sched.rs`. Both are currently module-private (`fn`, no `pub`). Change their
signatures to `pub(crate) fn` in `builtins.rs`:

```rust
pub(crate) fn format_args_str(sched: &Scheduler, fmt: Option<u32>, args: &[u32]) -> String { ... }
pub(crate) fn write_out(st: &mut SimState, text: &str) { ... }
```

and import them at the top of `sched.rs` (next to `use crate::exec::...`):

```rust
use crate::builtins::{format_args_str, write_out};
```

`Value` is already in scope in `sched.rs` (`use crate::value::Value;` at
`sched.rs:13`), so the value-list change detection needs no new import there.

### 3.3 Borrow shape (why this compiles)

* **Strobes:** `std::mem::take(&mut self.st.postponed.strobes)` moves the `Vec`
  out, *ending* the `&mut self.st` borrow. The loop then takes `format_args_str(self,
  ...)` (`&self`, immutable) to render, and `write_out(self.st, ...)` (`&mut
  self.st`) to print — sequentially, never simultaneously. This is the identical
  pattern to `apply_nba` (`sched.rs:272`: `let mut batch =
  std::mem::take(&mut self.nba);`).
* **Monitor:** hoist **every** read out of the `monitor` borrow in one tight
  scope — copy `enabled` (the match guard), copy `cap.fmt` (`Copy`), clone
  `cap.args` (`Vec<u32>`), and `take()` the previous `last_vals` (`Option<Vec<
  Value>>`, moved out, not cloned) — then the `&mut self.st.postponed.monitor`
  borrow ends (the `match` block closes, dropping `m`). Only *after* that do we
  `self.eval(eid)` / `format_args_str(self, ...)` (`&self`) and `write_out(self.st,
  ...)` (`&mut self.st`), then re-acquire a fresh `self.st.postponed.monitor.
  as_mut()` to store the new baseline. Nothing from the original borrow survives
  across the render, so there is **no NLL dependence** on a binding's last-use
  point — unlike a shape that keeps an `mon` binding live across the render, a
  future edit that reads the monitor after rendering (e.g. logging `enabled`)
  cannot resurrect a conflicting borrow. This matches the `mem::take`-style
  "render-then-record, no overlap" discipline the strobe arm already follows.

### 3.4 `$finish` / `$stop` in the same timestep — decision

**Decision: a postponed flush at the settle break is automatically SKIPPED on
`$finish`/`$stop` in the SAME timestep. This DIVERGES from reference simulators
(Icarus/VCS) and is a deliberate MVP scoping choice for implementation simplicity
and determinism, documented as the §7.3 deferral. We do NOT add an extra flush at
the Finish/Stop return sites.**

Rationale and exact mechanics:

* `$finish`/`$stop` make `builtins::dispatch` return `Ctl::Finish`/`Ctl::Stop`
  (`builtins.rs:51-52`); the executor maps these to `Step::Finish`/`Step::Stop`
  and returns from `run_process` (`exec.rs:48-49`); the scheduler's drain loop
  sets `self.st.finished = true` and **`return`s straight out of `run()`**
  (`sched.rs:190-197`) — bypassing the `break` at line 235 and therefore the
  `flush_postponed()` call entirely.
* **IEEE-accurate framing (important — the earlier "no settle reached" rationale
  overstated the justification):** IEEE 1364-2005 §5.4/§17 actually run the
  postponed region of the *current* timestep before terminating on `$finish` —
  the active-region `$finish` cancels *future* timesteps, not the current
  timestep's postponed region. So a conforming simulator (Icarus/VCS) WOULD drain
  the current postponed region and **print** the same-step strobe/changed monitor
  before exiting. vita's MVP intentionally does NOT, for one reason only:
  implementation simplicity and determinism (the engine already abandons the rest
  of the region pass on `$finish` — remaining Active processes and the NBA batch
  are dropped — and we keep that single early-return path rather than splicing a
  conditional drain into it). This is a known, documented **portability
  divergence**, not an IEEE-mandated behavior; it is the §7.3 deferral, and a
  future revision can adopt the strict drain (see footnote below).
* `$stop` is treated as `$finish` for the MVP (no interactivity); same skip and
  same divergence note.

> **If a future revision wants IEEE-strict same-step drain on `$finish`,** the
> safe shape is a `postponed_drained_this_step: bool` guard on `SimState`,
> cleared on time advance and set inside `flush_postponed`, with the drain gated
> on `!postponed_drained_this_step` — rather than sprinkling unconditional
> `self.flush_postponed();` calls at return sites (a naive "before each return"
> edit double-flushes or drains a prior step's region). The exact origin sites for
> a same-step `$finish`/`$stop` are precisely **two**:
>   * `FinishReason::Finish` at `sched.rs:192`,
>   * `FinishReason::Stop` at `sched.rs:196`.
>
> A guarded `self.flush_postponed();` immediately before *each of those two* would
> drain the current postponed region. It must **NOT** be added at:
>   * the finished-flag early returns `if self.st.finished { return
>     self.finish_kind(); }` at `sched.rs:174` (outer-loop top) and `sched.rs:187`
>     (active-batch loop) — these re-surface a *prior* timestep's already-handled
>     termination, so flushing there double-emits or emits stale output;
>   * `FinishReason::Error` (`:201`) or any `FinishReason::DeltaLimit`
>     (`:181,210,220,231`) — a fatal/oscillator state is not a clean settle.
>
> The guard approach is preferred precisely because it is robust to this site
> enumeration drifting as the loop evolves. This is explicitly out of scope for
> the MVP and noted in §7.3 DEFERRED.

The canonical "post-NBA strobe" cases (Example 1, tests 5.1) all use
`#1 $finish;` *after* the strobe's timestep, so the strobe's own timestep settles
normally and flushes — the skip only affects strobes textually co-located with
`$finish` in the *same* active region with no intervening delay.

---

## 4. Rendering reuse

`flush_postponed` reuses **`format_args_str` verbatim** — the exact same engine
`$display` uses. No second formatter, no radix divergence.

* Signature (unchanged except visibility): `pub(crate) fn format_args_str(sched:
  &Scheduler, fmt: Option<u32>, args: &[u32]) -> String` (`builtins.rs:234`).
* The call from `flush_postponed`: `format_args_str(self, cap.fmt, &cap.args)`
  where `self: &Scheduler`. The borrow is immutable, satisfying the `&Scheduler`
  parameter directly. It internally calls `sched.eval(eid)` →
  `EvalCtx { ir, nets: self.st, now: self.st.now, wt }` (`sched.rs:366-373`),
  and `expr_const_string(sched.st, fmt_eid)` for the template (`builtins.rs:245`)
  — the identical evaluation surface as `$display`, so `%d/%h/%b/%o/%c/%s/%t/%m`
  and X/Z formatting all behave exactly as in `$display`.
* The radix-suffixed variants (`$strobeb/o/h`, `$monitorb/o/h`) are NOT separate
  `SysTaskId`s in the frozen IR — they collapse onto `Strobe`/`Monitor` with the
  format string carrying the radix spec, so one formatter already covers them.

---

## 5. Tests (add to `crates/sim-engine/tests/end_to_end.rs`)

All tests use the existing `build()` + `simulate_capture()` harness shape
(`end_to_end.rs:25-46,165-171`). `simulate_capture` returns `(SimResult, String)`
where the String is concatenated RTL output (`$display`/`$write`/`$strobe`/
`$monitor` all route through `write_out` → `st.out` → `LogSink::RtlOutput`).

> **Driving distinct timesteps:** `$monitor` change-detection needs several
> timesteps. The idiom (already used across the suite, e.g.
> `nba_shifts_one_stage_per_clock`) is an `initial` block with `#N` delays. Each
> `#N` advances time and forces a fresh settle → a fresh postponed flush.

### 5.1 `$strobe` shows post-NBA value where `$display` shows pre-value (canonical)

```rust
#[test]
fn strobe_shows_post_nba_value_vs_display_pre() {
    // q starts 0, d=1. On the posedge: $display(q) prints pre-update 0; the NBA
    // q<=d schedules q→1 (applied in NBA region); $strobe(q) defers to the
    // postponed region and samples the settled post-NBA value 1.
    let src = "module m; reg clk; reg d; reg q; \
               always @(posedge clk) begin \
                 $display(\"disp %b\", q); q <= d; $strobe(\"strb %b\", q); \
               end \
               initial begin clk=0; d=1; q=0; \
                 #5 clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // $display fires in the active region (q still 0). $strobe fires in the
    // postponed region of the SAME timestep, after NBA set q=1.
    assert_eq!(out, "disp 0\nstrb 1\n");
}
```

### 5.2 Two strobes print in call order within one step (post-NBA values)

```rust
#[test]
fn two_strobes_print_in_call_order() {
    // In one posedge step: register $strobe(a) then $strobe(b). a is NBA-updated
    // to 9 this step. Postponed FIFO drains in call order: a-line (settled 9)
    // before b-line (2).
    let src = "module m; reg clk; reg [3:0] a; reg [3:0] b; \
               always @(posedge clk) begin \
                 $strobe(\"a=%0d\", a); $strobe(\"b=%0d\", b); a <= 4'd9; \
               end \
               initial begin clk=0; a=4'd1; b=4'd2; \
                 #5 clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Both strobes sample at end-of-timestep regardless of enqueue position:
    // a shows its settled post-NBA value 9; order is call order (a then b).
    assert_eq!(out, "a=9\nb=2\n");
}
```

### 5.3 `$strobe` is one-shot (does not repeat the next, unchanged step)

```rust
#[test]
fn strobe_is_one_shot_per_call() {
    // $strobe runs once inside the posedge body. The next timestep (a later #5
    // with NO posedge) must NOT reprint it: the FIFO was cleared at flush.
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"s=%0d\", a); \
               initial begin clk=0; a=4'd4; \
                 #5 clk=1; \
                 #5 a=4'd7; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Exactly one strobe line (from the single posedge); the later a=7 step does
    // not reprint because the strobe FIFO is cleared every flush.
    assert_eq!(out, "s=4\n");
}
```

### 5.4 `$monitor` prints once on establishment

```rust
#[test]
fn monitor_prints_once_on_establish() {
    // Establish $monitor on flag (=0). It prints once in the postponed region of
    // the establishing timestep (establishment-prints-immediately rule).
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor(\"flag=%b\", flag); \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "flag=0\n");
}
```

### 5.5 `$monitor` prints only on change across three steps (no duplicate)

```rust
#[test]
fn monitor_prints_only_on_change() {
    // Establish at t=0 (flag=0 → print). t=10 flag→1 (print). t=20 flag unchanged
    // (NO print). t=30 flag→0 (print). Three lines, the unchanged step is silent.
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor(\"flag=%b\", flag); \
                 #10 flag=1; \
                 #10 flag=1; \
                 #10 flag=0; \
                 #10 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // establish(0) → 1 → [unchanged, silent] → 0
    assert_eq!(out, "flag=0\nflag=1\nflag=0\n");
}
```

### 5.6 `$monitor` change detection is X-aware

```rust
#[test]
fn monitor_detects_x_transition() {
    // flag starts X (uninitialized 1-bit reg). Establish prints "flag=x". Then
    // flag→0 is a value change (X→0) and prints "flag=0". 4-state-aware equality:
    // a defined↔X transition counts as a change.
    let src = "module m; reg flag; \
               initial begin \
                 $monitor(\"flag=%b\", flag); \
                 #5 flag=0; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // %b of an X 1-bit reg renders 'x' (see fmt_radix X handling).
    assert_eq!(out, "flag=x\nflag=0\n");
}
```

### 5.7 A second `$monitor` replaces the first (global singleton)

```rust
#[test]
fn second_monitor_replaces_first() {
    // Monitor a (=0) at t=0; a→1 at t=10. At t=20 a SECOND $monitor on b (=7)
    // replaces the first. a→2 at t=30 is now invisible. b→8 at t=40 prints.
    let src = "module m; reg [3:0] a; reg [3:0] b; \
               initial begin a=4'd0; b=4'd7; \
                 $monitor(\"a=%0d\", a); \
                 #10 a=4'd1; \
                 #10 $monitor(\"b=%0d\", b); \
                 #10 a=4'd2; \
                 #10 b=4'd8; \
                 #10 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a establish(0) → a(1) → b establish(7) → [a→2 invisible] → b(8)
    assert_eq!(out, "a=0\na=1\nb=7\nb=8\n");
}
```

### 5.8 strobe + monitor ordering within one step (strobes-then-monitor)

```rust
#[test]
fn strobe_then_monitor_ordering_in_one_step() {
    // In a single timestep both a $strobe fires and the monitor changes. Frozen
    // tie-break: strobe line FIRST, then the monitor line.
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"S=%0d\", a); \
               initial begin clk=0; a=4'd0; \
                 $monitor(\"M=%0d\", a); \
                 #5 a=4'd5; clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // t=0 postponed: monitor establish prints M=0 (no strobe yet).
    // t=5 postponed: a changed 0→5 AND a strobe fired this step → strobe first
    // (S=5), then monitor (M=5).
    assert_eq!(out, "M=0\nS=5\nM=5\n");
}
```

### 5.9 `$finish` in the same active region skips the same-step postponed output

```rust
#[test]
fn strobe_then_finish_same_step_is_skipped() {
    // $strobe then $finish in the SAME active region with no intervening delay:
    // the engine returns on $finish before the settle break, so the postponed
    // flush never runs for this step → the strobe prints nothing.
    //
    // PORTABILITY NOTE: this DIVERGES from reference simulators. IEEE 1364-2005
    // §5.4/§17 drain the CURRENT timestep's postponed region before terminating
    // on $finish, so Icarus/VCS would print "s=3\n" here. vita's MVP skips it for
    // implementation simplicity/determinism (documented §3.4 + §7.3). The expected
    // target for a future IEEE-strict revision is therefore `"s=3\n"`; this test
    // pins the deliberate MVP behavior (empty output) so the divergence is golden,
    // not accidental.
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd3; $strobe(\"s=%0d\", a); $finish; end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // MVP: "" (skip). IEEE-strict / Icarus / VCS reference target: "s=3\n".
    assert_eq!(out, "", "no postponed flush after same-step $finish (MVP divergence)");
}
```

### 5.10 `$strobe` interleaves correctly with `$display` (deferral proof, no NBA)

```rust
#[test]
fn strobe_defers_past_later_blocking_writes() {
    // Within one initial block: $strobe(a) is registered while a=1, then a
    // blocking a=2 runs, then $display(a) prints 2. The strobe, deferred to the
    // postponed region, samples the FINAL settled a=2 — proving the strobe reads
    // end-of-timestep state, not the call-site value, even with blocking writes.
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd1; $strobe(\"s=%0d\", a); a=4'd2; \
                 $display(\"d=%0d\", a); #1 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // $display prints d=2 immediately (active region). The strobe flushes at the
    // settle of t=0 (before the #1 advances time) sampling a=2.
    assert_eq!(out, "d=2\ns=2\n");
}
```

### 5.11 Determinism: identical SimIr → identical strobe+monitor output twice

```rust
#[test]
fn strobe_monitor_deterministic_repeat() {
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"s=%0d\", a); \
               initial begin clk=0; a=4'd0; \
                 $monitor(\"m=%0d\", a); \
                 #5 a=4'd1; clk=1; \
                 #5 clk=0; #5 a=4'd2; clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_r1, o1) = simulate_capture(&ir, SimOpts::default());
    let (_r2, o2) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(o1, o2, "same SimIr → byte-identical strobe+monitor output");
}
```

### 5.12 `$monitor` re-establish on the SAME signal resets the baseline (re-print)

```rust
#[test]
fn monitor_reestablish_same_signal_reprints() {
    // Monitor a (=5) → establish prints. a unchanged. Re-issue $monitor on the
    // SAME a: replace semantics reset `last`, so it prints again at that step
    // even though a's value did not change.
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd5; \
                 $monitor(\"a=%0d\", a); \
                 #5 $monitor(\"a=%0d\", a); \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // First establish prints a=5; re-establish resets last_vals=None → prints
    // a=5 again.
    assert_eq!(out, "a=5\na=5\n");
}
```

### 5.13 No-arg `$monitor;` emits nothing (no bare newline) — golden-pinned

```rust
#[test]
fn no_arg_monitor_emits_nothing() {
    // A bare `$monitor;` (fmt=None, args=[]) has zero monitored expressions. The
    // flush guard skips it entirely — it must NOT inject a lone "\n" into RTL
    // output at the establishing timestep (or any later step). This pins the
    // deliberate decision from §7.4 / the flush no-arg guard so the output is
    // golden-checked, not emergent.
    //
    // NOTE: depends on elaborate lowering a bare `$monitor;` to a Monitor node
    // with no args. If the front end does not yet emit such a node, gate this test
    // behind the same support; the assertion (empty output) is the contract.
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor; \
                 #5 flag=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Zero-expression monitor: no establishment line, no per-step line.
    assert_eq!(out, "", "no-arg $monitor emits no bytes, not even a newline");
}
```

### 5.14 Monitor reprints on an X→X value transition the rendered string hides

```rust
#[test]
fn monitor_reprints_on_unknown_to_unknown_value_change() {
    // IEEE-correctness regression for value-level (not rendered-string) change
    // detection. q is a 4-bit reg. Under `%d`, EVERY value containing any X
    // renders to literal "x" (builtins fmt_dec returns "x" on any_unknown). So a
    // rendered-string diff would suppress the second print. Value-level 4-state
    // equality detects 4'b00xx → 4'b0x00 as a genuine change → reprint.
    //
    //   t=0  establish: q = 4'b00xx → "x"   (print)
    //   t=5  q = 4'b0x00           → "x"   (DIFFERENT value, same string → MUST print)
    //   t=10 q = 4'b0x00           → "x"   (unchanged value+string → silent)
    let src = "module m; reg [3:0] q; \
               initial begin q=4'b00xx; \
                 $monitor(\"q=%d\", q); \
                 #5 q=4'b0x00; \
                 #5 q=4'b0x00; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Three lines? No — two: establish + the X→X value change. The third step is
    // a true no-op (identical (val,unk) planes) and stays silent. All three
    // render to "q=x"; only value-level equality distinguishes them.
    assert_eq!(out, "q=x\nq=x\n");
}
```

> **Test count:** 14 tests (5.1–5.14), covering every required case: post-NBA
> strobe vs display pre-value (5.1), two strobes in order (5.2), one-shot (5.3),
> monitor establish (5.4), monitor change-only across 3+ steps (5.5), X-aware
> change (5.6), second-monitor-replaces (5.7), strobe+monitor ordering (5.8),
> $finish-same-step MVP-skip + reference target (5.9), deferral-past-blocking
> (5.10), determinism (5.11), re-establish baseline reset (5.12), no-arg monitor
> emits nothing (5.13), and value-level X→X change detection that a rendered-string
> diff would miss (5.14).

---

## 6. Invariants: no IR change, no diag codes

* **Frozen IR untouched.** `SysTaskId::Strobe` (`crates/sim-ir/src/lib.rs:211`)
  and `SysTaskId::Monitor` (`:210`) already exist; `Stmt::SysTask { which, fmt,
  args }` (`:239`) is unchanged. No field is added/removed/reordered in any
  `#[derive(SchemaHash)]` type, so the sim-ir root hash does NOT flip and every
  `.velab`/`.vu` stays valid. `format_version` is NOT bumped. The new
  `FmtCapture`/`MonitorState`/`Postponed` types live in `sim-engine` (a runtime
  crate, never serialized, no `SchemaHash` derive) — they are pure per-run scratch
  on `SimState`, exactly like `dump_pending_path`.
* **No new diag codes.** The MsgCode bijection (doc-15, 36 codes, gated by
  `tests/` bijection) is untouched: `$strobe`/`$monitor` emit RTL output through
  the existing `write_out` → `st.out` path, never a `Diagnostic`. No `MsgCode`
  variant is added. `cargo test --workspace --locked` keeps passing the bijection
  gate.
* **Verification gate:** `cargo build --workspace --locked`, `cargo test
  --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D
  warnings`, `cargo fmt --all -- --check` must all pass. The 14 new tests + the
  existing 30+ `end_to_end` tests are the regression net.

---

## 7. DEFERRED (explicit, out of scope for this MVP)

1. **`$monitoron` / `$monitoroff`** — no `SysTaskId` variant exists in the frozen
   IR (only `Monitor`). The `enabled: bool` field on `MonitorState` is present and
   the flush already honors it (`if mon.enabled`), so when the tasks land they
   are a thin layer: `$monitoroff` sets `enabled=false`; `$monitoron` sets
   `enabled=true` + `last_vals=None` (immediate re-print + rebaseline). Adding
   them WOULD touch the frozen IR (`SysTaskId` is `#[derive(SchemaHash)]`) → a
   `format_version` bump, hence deferred to a deliberate IR revision.
2. **File variants `$fstrobe` / `$fmonitor`** — require an fd/MCD subsystem
   (`$fopen`/`$fclose`) that does not exist. Region semantics are identical; a
   thin layer once file I/O lands. No `SysTaskId` variant exists.
3. **IEEE-strict same-step `$finish` postponed drain** — the MVP skips the
   same-step postponed flush on `$finish`/`$stop` (§3.4). This is a deliberate
   **divergence from reference simulators** (IEEE/Icarus/VCS drain the *current*
   timestep's postponed region before terminating; vita does not, for
   implementation simplicity/determinism). The strict alternative — a
   `postponed_drained_this_step` guard + a guarded `flush_postponed()` before the
   two origin sites `FinishReason::Finish` (`sched.rs:192`) and
   `FinishReason::Stop` (`sched.rs:196`), and explicitly NOT at the finished-flag
   early returns (`:174`,`:187`) nor `Error`/`DeltaLimit` — is documented in the
   §3.4 footnote but not implemented. Test 5.9 pins the MVP behavior and records
   the IEEE-strict reference target (`"s=3\n"`).
4. **`$strobe` / `$monitor` with NO args (bare `$monitor;`)** — RESOLVED for
   `$monitor` by an explicit flush guard: when `cap.fmt.is_none() &&
   cap.args.is_empty()` the monitor branch emits **nothing** (not even a bare
   newline) and seeds `last_vals = Some(vec![])`. This is golden-pinned by test
   5.13, so a future bare-`$monitor` lowering cannot silently inject a lone `"\n"`
   into deterministic RTL output. (Bare `$strobe;` is not specially guarded — a
   no-arg strobe is an explicit one-shot user statement, so emitting its empty
   line is the expected call-site behavior; only the *re-evaluated-every-step*
   monitor singleton needed the guard. If a bare-`$strobe;` policy is ever wanted,
   apply the same `fmt.is_none() && args.is_empty()` test in the strobe loop.)
5. **Implicit time-prefix change suppression** — only explicit user argument
   expressions drive monitor change detection (now value-level, via the evaluated
   `Vec<Value>` baseline). The
   `$time`-prefix special-case (so an implicit time stamp does not force a print
   every step) is deferred; vita's `$monitor` does not auto-prepend `$time`, so
   this is a non-issue until/unless that prefix is added.

---

## 8. Diff manifest (files touched, all in `crates/sim-engine`)

| File | Change |
|---|---|
| `src/state.rs` | + `FmtCapture`, `MonitorState` (holds `Vec<Value>` baseline), `Postponed` types; + `use crate::value::Value;` if not present; + `postponed: Postponed` field on `SimState`; + `postponed: Postponed::default()` in `SimState::new`. |
| `src/builtins.rs` | rewrite `Monitor \| Strobe` arm → two arms (register strobe / replace monitor); `format_args_str`/`write_out` → `pub(crate)`; + import `FmtCapture, MonitorState`. |
| `src/sched.rs` | + `flush_postponed()` method; + call site after `break; // time-step stable`; + import `format_args_str, write_out`. |
| `tests/end_to_end.rs` | + 14 tests (§5.1–5.14). |

No change to `src/exec.rs`, `src/lib.rs`, `src/eval.rs`, `src/value.rs`,
`src/width.rs`, or any crate outside `sim-engine`. No `Cargo.toml` change.
