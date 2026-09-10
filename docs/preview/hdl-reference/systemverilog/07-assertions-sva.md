# 07 · SystemVerilog Assertions (SVA)

Based on IEEE 1800-2017 §16.

> **Non-synthesizable only**: assertions are for simulation and formal verification. RTL code
> containing `assert`/`assume`/`cover` is either ignored by the synthesis tool or draws a
> warning. For the synthesizability summary see [09-synthesizability.md](09-synthesizability.md).

> This document describes the language. For which of these constructs vita itself accepts,
> refuses loudly, or supports only in part, see
> [docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## Immediate assertions (§16.4)

An immediate assertion sits inside a procedural block and evaluates values directly at the
current simulation time. It works the same way an `if/else` statement does — no clock is
involved.

```systemverilog
always_ff @(posedge clk) begin
    // immediate assert: evaluated at the current simulation time
    assert (data_valid || !enable)
        else $error("enable high but data_valid low");

    // the three forms: assert / assume / cover
    assert (cnt < MAX_CNT);          // on failure, $error (the default severity)
    assume (req == 1'b0);            // an environment assumption for formal verification
    cover  (state == IDLE);          // collects a coverage point
end
```

### assert / assume / cover semantics

| Keyword | Meaning | On failure |
|--------|------|---------|
| `assert` | The property must hold | $error (default), or the stated severity |
| `assume` | An environment constraint under formal verification. Under simulation it behaves like assert | $error |
| `cover` | Tracks whether the property ever became true (coverage) | — (cannot fail) |

### Severity system tasks

```systemverilog
assert (expr) else $fatal(1, "critical: %s", msg);   // ends the simulation
assert (expr) else $error("error: %0d", val);         // increments the error count
assert (expr) else $warning("warn");                  // a warning only
assert (expr) else $info("note");                     // informational
```

---

## Concurrent assertions (§16.14)

A concurrent assertion is evaluated **on every clock edge** and describes sequential behaviour
spanning several cycles. It can be placed directly in a module, an interface or a program
block, or inside an `always` block.

```systemverilog
// property + assert property declared separately (recommended)
property req_ack_handshake;
    @(posedge clk) disable iff (!rst_n)
    req |-> ##[1:3] ack;
endproperty

assert property (req_ack_handshake)
    else $error("ack did not arrive within 3 cycles of req");

// inline declaration
assert property (@(posedge clk) req |=> gnt)
    else $warning("gnt not granted in 1 cycle");

// cover — coverage collection
cover property (@(posedge clk) req ##2 ack);

// assume — an environment assumption for formal verification
assume property (@(posedge clk) $onehot(mode));
```

### Observed-region sampling

A concurrent assertion is evaluated in the SV scheduler's **Observed region**.

```
Active → NBA → Observed(assertion eval) → Reactive → Postponed
```

After the clock edge it samples the settled values, once the logic in the Active and NBA
regions has completed. This is what keeps a `glitch` from making the assertion misfire.

`$past(sig, N)`: returns the sampled value from N clocks earlier.

```systemverilog
// if req was true 2 cycles ago, ack must be true now
assert property (@(posedge clk)
    $past(req, 2) |-> ack);
```

---

## Sequence declarations (§16.9~16.10)

A sequence is the primitive block that describes a timing relationship in units of clock
cycles.

```systemverilog
sequence s_req_to_ack;
    req ##[1:3] ack;
endsequence

// a sequence taking arguments
sequence s_check_burst(logic start, logic done, int delay);
    start ##[1:delay] done;
endsequence
```

### Cycle delays ##N / ##[m:n]

| Syntax | Meaning |
|------|------|
| `seq1 ##1 seq2` | seq2 starts 1 cycle after seq1 ends |
| `seq1 ##0 seq2` | seq2 starts in the same cycle (overlapping) |
| `seq1 ##N seq2` | seq2 starts N cycles later |
| `seq1 ##[m:n] seq2` | seq2 starts somewhere between m and n cycles later |
| `seq1 ##[1:$] seq2` | seq2 follows at any time at least 1 cycle later |

```systemverilog
// exactly 2 cycles later
assert property (@(posedge clk) req |-> ##2 ack);

// within 1 to 5 cycles
assert property (@(posedge clk) req |-> ##[1:5] ack);

// eventually, at some point (at least 1 cycle later)
assert property (@(posedge clk) req |-> ##[1:$] ack);
```

### Repetition operators (§16.9.2~16.9.4)

#### Consecutive repetition [*N]

Matches repeatedly at 1-cycle intervals for N consecutive cycles. Applies to sequences as well.

```systemverilog
// sig true for 4 consecutive cycles
sig[*4]                    // = sig ##1 sig ##1 sig ##1 sig

// 2 to 5 consecutive cycles
sig[*2:5]

// zero or more times (an empty match is included)
sig[*]                     // = sig[*0:$]

// one or more times
sig[+]                     // = sig[*1:$]

// after req, data_valid holds for 4 consecutive cycles until ack arrives
assert property (@(posedge clk)
    req |-> data_valid[*4] ##1 ack);
```

#### Non-consecutive repetition [=N]

A boolean expression becomes true N times, not necessarily consecutively; any number of clocks
may fall in between. **Applies to boolean expressions only** (not to sequences).

```systemverilog
// data_rdy is true 3 times, non-consecutively, between start and end
assert property (@(posedge clk)
    start ##1 data_rdy[=3] ##1 end_flag);

// with a range
assert property (@(posedge clk)
    req |-> ack_pulse[=1:4] ##[0:1] done);
```

#### Goto repetition [->N]

Similar to non-consecutive repetition, except that **the sequence completes at the Nth and
last match**. **Applies to boolean expressions only.**

```systemverilog
// completes at the last of 4 rd_en matches, then checks intr_en
assert property (@(posedge clk)
    $rose(rdy) ##1 rd_en[->4] ##1 intr_en);

// [=N] vs [->N]:
// [=N]: extra clocks are allowed between the last match and the end condition
// [->N]: the next expression starts immediately after the last match
```

---

## Property declarations (§16.12~16.13)

A property is the higher-level construct that describes a temporal property. It is more
expressive than a sequence.

```systemverilog
property p_name [#(params)] [port_list] ;
    [disable iff (expr)]
    property_expr
endproperty
```

### Implication operators (§16.12.7)

| Operator | Name | Meaning |
|--------|------|------|
| `seq \|-> prop` | overlapping implication | prop is evaluated in the **same** cycle in which seq matches |
| `seq \|=> prop` | non-overlapping implication | prop is evaluated in the cycle **after** seq matches |

```systemverilog
// overlapping: in the cycle where req is true, ack must be true too
property p1;
    @(posedge clk) req |-> ack;
endproperty

// non-overlapping: ack is true the cycle after req (equivalent to |-> ##1)
property p2;
    @(posedge clk) req |=> ack;
endproperty

// combined: ack within 1 to 3 cycles after req
property p3;
    @(posedge clk) req |-> ##[1:3] ack;
endproperty
```

### throughout — a sequence operator (§16.9.9)

The condition must hold for the entire span of the sequence.

```systemverilog
// with req true, data_valid is held until ack_pulse arrives
property p_burst_valid;
    @(posedge clk)
    $rose(req) |-> (data_valid throughout ack_pulse[->1]);
endproperty
```

### until / until_with — property operators (§16.13.4)

| Operator | Form | Meaning |
|--------|------|------|
| `p1 until p2` | non-overlapping | p1 holds up to **just before** p2 becomes true (p2's cycle excluded) |
| `p1 until_with p2` | overlapping | p1 holds **including** the cycle in which p2 becomes true |

```systemverilog
// req is true right up to gnt (req is not required in the gnt cycle)
property p_req_until_gnt;
    @(posedge clk) req until gnt;
endproperty

// req is true including the gnt cycle
property p_req_until_with_gnt;
    @(posedge clk) req until_with gnt;
endproperty
```

**Note**: `until_with` is a property, so it cannot be joined directly with `##N`.

### implies (§16.12.9)

Implication at the property level. If the antecedent is false, the whole thing is vacuously
true.

```systemverilog
// if mode_A, then output_en must be true
property p_mode;
    @(posedge clk) (mode == MODE_A) implies output_en;
endproperty
```

Unlike `|->` (sequence implication), `implies` takes properties on both sides.

### always / s_eventually (§16.13.1~16.13.2)

```systemverilog
// always: p is true at every clock, indefinitely (a safety property)
property p_safety;
    @(posedge clk) always (fifo_count <= FIFO_DEPTH);
endproperty

// s_eventually: it must become true at some point (strong liveness)
property p_liveness;
    @(posedge clk) s_eventually (req_served);
endproperty

// always s_eventually: it must become true again and again (strong recurrence)
property p_recurrence;
    @(posedge clk) always s_eventually (heartbeat);
endproperty
```

| Operator | Kind | Meaning |
|--------|------|------|
| `always p` | safety | p is true at every point in time |
| `s_eventually p` | strong liveness | p must become true at some point |
| `eventually p` | weak liveness | p may become true at some point (vacuously true on a finite trace) |
| `always s_eventually p` | strong recurrence | p is true recurrently |

---

## Clocking blocks and assertions (§14.3, §16.14.6)

Using an interface's clocking block as the assertion clock builds the design-verification
timing contract into the code itself.

```systemverilog
interface bus_if (input clk);
    logic req, ack;

    clocking cb @(posedge clk);
        input  #1step req;   // sampled 1step earlier
        output #1 ack;
    endclocking

    // the clocking block used as the assertion clock
    property p_req_ack;
        @(cb) req |-> ##[1:4] ack;
    endproperty
    assert property (p_req_ack);
endinterface
```

`default clocking` lets each assertion omit its clock:

```systemverilog
module checker_blk (input clk, req, ack);
    default clocking main_clk @(posedge clk); endclocking

    // clock omitted — the default clocking is used
    assert property (req |-> ##[1:3] ack);
    assert property (ack |=> !ack);   // ack is a 1-cycle pulse
endmodule
```

---

## Action blocks (§16.14.7)

Define pass/fail callbacks that run according to the assertion's result.

```systemverilog
assert property (p_req_ack)
    $display("PASS: req->ack handshake OK at %0t", $time)  // pass action
    else $error("FAIL: req->ack timeout at %0t", $time);   // fail action

// pass action only (with the fail action omitted, the default $error applies)
// fail action only (an else clause alone means there is no pass action)
assert property (p_cnt_no_overflow)
    else $fatal(1, "Counter overflow detected!");
```

---

## disable iff — handling reset (§16.14.3)

```systemverilog
property p_with_reset;
    @(posedge clk) disable iff (!rst_n)
    req |-> ##[1:3] ack;
endproperty
```

`disable iff (cond)`: while cond is true, assertion evaluation is disabled. It is the idiom
that keeps assertions from misfiring while reset is asserted.

---

## Related documents

- [04-interfaces.md](04-interfaces.md) — clocking block declarations in detail
- [06-classes-oop.md](06-classes-oop.md) — UVM-style verification patterns (classes + randomization)
- [09-synthesizability.md](09-synthesizability.md) — the assertion ❌ non-synthesizable mapping
- `../system-tasks/11-assertion-sampling.md` — `$past`, `$rose`, `$fell`, `$stable`, `$sampled`

---

## Sources

- IEEE 1800-2017 §16 (Assertions), §14 (Clocking blocks)
- chipverify.com/systemverilog/systemverilog-assertions (WebFetch ✓)
- vlsi.pro/sva-sequences-repetition-operators/ — [*N] [=N] [->N] (WebFetch ✓)
- verificationguide.com/systemverilog/systemverilog-implication-operator/ — \|-> \|=> (WebFetch ✓)
- verificationacademy.com/forums/systemverilog/sva-throughout-vs-until — throughout/until_with (WebFetch ✓)
