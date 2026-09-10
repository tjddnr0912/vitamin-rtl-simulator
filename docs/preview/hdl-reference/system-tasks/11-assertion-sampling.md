# 11 · Assertion Sampling Functions

## Overview

This category covers the temporal sampling functions used by SVA (SystemVerilog Assertions)
concurrent assertions.
`$past`, `$rose`, `$fell`, `$stable`, `$changed` and `$sampled` query a signal's history relative
to a clock, and `$assertoff`, `$asserton`, `$assertkill` and `$assertcontrol` control at run time
whether assertions are active.

All of them are verification-only and not synthesizable; they are defined in IEEE 1800-2017 §16.

## vita support

This note describes the language, not the simulator. What vita accepts today is recorded in
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Clock-based sampling semantics

Understanding these functions starts with the region structure of the simulation scheduler.
The full scheduler model is covered in [06-simulation-engine.md](../../06-simulation-engine.md);
what follows summarises only the part that bears directly on assertions.

Within one timestep (one time slot) the regions run in this order:

```
Active → Inactive → NBA → ... → Observed → Reactive → ...
```

- **Active region**: blocking assignments, continuous assignments and gate logic run here.
- **NBA region**: nonblocking assignments (`<=`) take effect — a signal does not hold its final
  value until this stage completes.
- **Observed region**: concurrent assertions are evaluated. This runs after the NBA region, with
  the signals settled.
- **Reactive region**: where an assertion's pass/fail action block runs.

The Observed region is the key. When an assertion looks at a signal it sees an
"already settled value" that reflects every change made in Active, Inactive and NBA.
That value is the **sampled value**.

---

## Entries

### `$past(expr [, n [, enable_expr [, @clocking_event]]])`

- **Standard**: IEEE 1800-2017 §16.9.3
- **Meaning**: returns the sampled value from n clock cycles ago.
- `n`: how many cycles back. Defaults to 1. It must be a constant of at least 1.
- `enable_expr`: only a clock edge at which this condition holds counts as "one tick".
  On an edge where it is false the counter does not advance.
- `@clocking_event`: when it is not given, the assertion's own clocking event is inherited.

```sv
// the default — the value one clock ago
assert property (@(posedge clk)
  req |-> ##1 ack == $past(req));

// the value three clocks ago
assert property (@(posedge clk)
  out == $past(in, 3));

// enable_expr — count only while valid is 1
assert property (@(posedge clk)
  valid |-> out == $past(data, 2, valid));

// an explicit clock
assert property (
  out == $past(in, 1, , @(posedge clk)));
```

**n = 0 is not allowed**: IEEE 1800-2017 requires `n ≥ 1`.
To get the value of the current cycle use `$sampled(expr)`, or reference expr directly.

---

### `$rose(expr [, @clocking_event])` — a rising edge on the LSB

- **Standard**: IEEE 1800-2017 §16.9.2
- **Returns**: bit (true/false)
- **Meaning**: true when the LSB's sampled value at the previous clock edge was 0, x or z and its
  sampled value now is 1.

Formally:

```
$rose(expr) ≡ ($past(expr[0]) !== 1'b1) && (expr[0] === 1'b1)
```

Note that a transition from x or z up to 1 also returns true.
Holding at 1 (stable high) is false.

```sv
// check the property when ack goes 0→1 at posedge clk
assert property (@(posedge clk)
  $rose(ack) |-> ##1 done);

// on a multi-bit signal only the LSB is examined
logic [3:0] bus;
$rose(bus)  // detects only a (0/x/z)→1 transition on bus[0]; the upper bits are irrelevant
```

---

### `$fell(expr [, @clocking_event])` — a falling edge on the LSB

- **Standard**: IEEE 1800-2017 §16.9.2
- **Returns**: bit
- **Meaning**: true when the LSB's previous sampled value was 1, x or z and its sampled value now
  is 0.

Formally:

```
$fell(expr) ≡ ($past(expr[0]) !== 1'b0) && (expr[0] === 1'b0)
```

```sv
// detecting the release of an active-low reset
assert property (@(posedge clk)
  $fell(rst_n) |-> ##[1:5] fsm_idle);
```

---

### `$stable(expr [, @clocking_event])` — the value held

- **Standard**: IEEE 1800-2017 §16.9.2
- **Returns**: bit
- **Meaning**: true when the sampled value at the previous clock edge equals the sampled value at
  the current one. The comparison is 4-state, so x→x and z→z also count as stable.

Formally:

```
$stable(expr) ≡ ($past(expr) === expr)
```

```sv
// handshake: addr must hold steady while req is asserted
assert property (@(posedge clk)
  req && !$rose(req) |-> $stable(addr));
```

---

### `$changed(expr [, @clocking_event])` — the value changed

- **Standard**: IEEE 1800-2017 §16.9.2
- **Returns**: bit
- **Meaning**: true when the sampled value at the previous clock edge differs from the sampled
  value at the current one. The logical opposite of `$stable`.

Formally:

```
$changed(expr) ≡ !$stable(expr) ≡ ($past(expr) !== expr)
```

```sv
// detecting a state-machine transition
assert property (@(posedge clk)
  $changed(state) |-> valid_transition(state));
```

---

### `$sampled(expr)` — the settled sampled value inside an action block

- **Standard**: IEEE 1800-2017 §16.9.1
- **Meaning**: returns the sampled value as of the Observed region in which the assertion was
  evaluated.

**When to use it**: inside an assertion's property body every signal already reads as its sampled
value, so `$sampled` adds nothing there. Where `$sampled` earns its keep is the **action block**
(the pass/fail block).

An action block runs in the Reactive region. By then the Active region may have moved the signals
again, so referencing a signal directly from Reactive — `$display("val=%0d", sig)` — can print a
value other than the one the assertion was evaluated against.

`$sampled(sig)` asks explicitly for the "settled" sampled value from the Observed region and so
avoids that mismatch.

```sv
// printing the exact signal values from an action block
assert property (@(posedge clk) req |-> ack)
else $error("req=%0b ack=%0b at time %0t",
            $sampled(req), $sampled(ack), $time);

// inside a property body $sampled is unnecessary (it is redundant)
assert property (@(posedge clk)
  $sampled(req) |-> $sampled(ack));  // no need to write it this way
```

---

## Assertion runtime control

### `$assertoff` / `$asserton` / `$assertkill`

- **Standard**: IEEE 1800-2017 §20.12
- **Purpose**: suppressing the noise over an interval where assertion violations are expected, such
  as a reset window or an initialisation sequence.

```sv
// signatures
$assertoff  [(levels [, list_of_scopes])];
$asserton   [(levels [, list_of_scopes])];
$assertkill [(levels [, list_of_scopes])];
```

- `levels`: 0 = affect the whole hierarchy. 1 = the named scope only. n = down n levels.
  When omitted it behaves as 0 (everything).
- With no scope argument the effect applies to the whole design.

| Task | Behaviour |
|--------|------|
| `$assertoff` | Lets assertions already in flight run to completion, then disables new ones |
| `$asserton` | Re-enables disabled assertions |
| `$assertkill` | Terminates immediately, in-flight assertions included |

```sv
initial begin
  // suppress assertions across the reset window
  $assertoff(0);          // disable everything

  rst_n = 0;
  #100;
  rst_n = 1;
  #20;

  $asserton(0);           // re-enable everything
end
```

---

### `$assertcontrol` — fine-grained runtime control

Use it where the control needed is finer than `$assertoff`, `$asserton` and `$assertkill` cover.
It can act separately on each kind of assertion (concurrent, immediate and so on) and each kind of
directive (assert, cover, assume).

```sv
// signature (IEEE 1800-2017 §20.12)
$assertcontrol(control_type
               [, [assertion_type]
               [, [directive_type]
               [, [levels]
               [, list_of_modules_or_assertions]]]]])
```

#### control_type values

| Value | Name | Meaning |
|----|------|------|
| 1 | Lock | Locks the assertion state (later assertoff/on calls are ignored) |
| 2 | Unlock | Releases the lock |
| 3 | On | Enable (equivalent to `$asserton`) |
| 4 | Off | Disable (equivalent to `$assertoff`) |
| 5 | Kill | Kill immediately (equivalent to `$assertkill`) |
| 6 | PassOn | Enable execution of the pass action block |
| 7 | PassOff | Disable execution of the pass action block |
| 8 | FailOn | Enable execution of the fail action block |
| 9 | FailOff | Disable execution of the fail action block |
| 10 | NonvacuousOn | Enable only passes that are not vacuous successes |
| 11 | VacuousOff | Disable vacuous passes |

#### assertion_type bitmask (values may be OR-ed)

| Value | Meaning |
|----|------|
| 1 | Concurrent assertions |
| 2 | Simple Immediate assertions |
| 4 | Observed Deferred Immediate assertions |
| 8 | Final Deferred Immediate assertions |
| 16 | `expect` statements |
| 32 | `unique` conditionals |
| 64 | `unique0` conditionals |
| 128 | `priority` conditionals |

#### directive_type bitmask (values may be OR-ed)

| Value | Meaning |
|----|------|
| 1 | the `assert` directive |
| 2 | the `cover` directive |
| 4 | the `assume` directive |

```sv
// e.g. turn off only the assert directive on simple immediate assertions
$assertcontrol(4, 2, 1);

// e.g. disable the pass action block of concurrent assertions
$assertcontrol(7, 1, 1);

// e.g. turn off the cover directive on every assertion type
$assertcontrol(4, 255, 2);
```

`$assertoff`, `$asserton` and `$assertkill` can be read as simple wrappers around
`$assertcontrol(4, ...)`, `$assertcontrol(3, ...)` and `$assertcontrol(5, ...)` respectively.

---

## The functions side by side

| Function | Scope | Underlying idea |
|------|----------|----------|
| `$rose(e)` | LSB rising transition | a comparison against `$past(e[0])` |
| `$fell(e)` | LSB falling transition | a comparison against `$past(e[0])` |
| `$stable(e)` | the value held | `$past(e) === e` |
| `$changed(e)` | the value changed | `!$stable(e)` |
| `$past(e, n)` | the value n clocks ago | sampled-value history |
| `$sampled(e)` | the current sampled value | a safe reference inside an action block |

---

## Icarus / Verilator support

| Function | Icarus Verilog | Verilator |
|------|---------------|-----------|
| `$past`, `$rose`, `$fell`, `$stable`, `$changed` | ❌ rejected (13.0, see ⚠️ below) | supported (needs `--assert`) |
| `$sampled` | ❌ rejected (13.0) | supported |
| `$assertoff`, `$asserton`, `$assertkill` | partial | supported |
| `$assertcontrol` | limited | limited |

> ⚠️ **Measured live against iverilog 13.0:** iverilog 13.0 **rejects** concurrent assertions along
> with `$past`, `$rose`, `$fell` and `$stable` ("not supported" / "not defined"), so it is not an
> oracle for these functions.
> Verilator ignores assertions altogether unless the `--assert` flag is given.

---

## Synthesizability

❌ None of these functions is synthesizable — they are verification-only.
An assertion that references `$past` can still be used for equivalence checking with a formal
verification tool (SymbiYosys and the like), but it never appears in a synthesized netlist.

---

## Sources

- IEEE 1800-2017 §16.9 (sampled value functions), §16.12 (assertion control)
- IEEE 1800-2017 §20.12 ($assertoff/$asserton/$assertkill/$assertcontrol)
- research-log: [system-tasks-random-assertion-2026-05-28.md](../../../history/research-log/system-tasks-random-assertion-2026-05-28.md)
- Scheduler region structure: [06-simulation-engine.md](../../06-simulation-engine.md)
- [circuitcove.com — Assertion Control](https://circuitcove.com/system-tasks-assertion/) (WebFetch ✓)
- [circuitcove.com — Sampled Value Functions](https://circuitcove.com/system-tasks-sampled/) (WebFetch ✓)
- [vlsiverify.com — Sample Value Functions](https://vlsiverify.com/system-verilog/assertions/sample-value-functions/) (WebFetch ✓)
- [verificationguide.com — SVA Built-in Methods](https://verificationguide.com/systemverilog/systemverilog-sva-built-in-methods/) (WebFetch ✓)
- [accellera.org sv-bc — $assertcontrol discussion](https://www.accellera.org/images/eda/sv-bc/10917.html) (WebFetch ✓)
