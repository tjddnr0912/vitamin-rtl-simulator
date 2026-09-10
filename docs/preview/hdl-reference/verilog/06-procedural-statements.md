# 06 · Verilog Procedural Statements

Per IEEE 1364-2001/2005. Covers the constructs that control execution flow inside
`initial` / `always` blocks: conditionals, case, loops, fork-join, and timing control
(delay/event).

---

## if / else

```verilog
// basic if-else
always @(*) begin
    if (sel == 2'b00)
        y = a;
    else if (sel == 2'b01)
        y = b;
    else if (sel == 2'b10)
        y = c;
    else
        y = d;
end
```

**Priority-encoder semantics**: in an `else if` chain, the conditions higher up take
priority over the ones below. Write `if-else if` when a priority encoder is what you
mean; use `case` when the selection is parallel.

**Avoiding latches**: in a combinational `always`, an `if` with no `else`, or a set of
branches that does not cover every condition, makes the synthesis tool infer a latch.

```verilog
// ❌ infers a latch — no else
always @(*) begin
    if (en) q = d;   // nothing assigns q when en=0 → latch
end

// ✅ no latch — else added
always @(*) begin
    if (en) q = d;
    else    q = q_default;
end
```

---

## case / casez / casex

### case

Full 4-state comparison (identical to `===`). `x` and `z` values compare exactly.

```verilog
always @(*) begin
    case (opcode)
        2'b00: result = a + b;
        2'b01: result = a - b;
        2'b10: result = a & b;
        2'b11: result = a | b;
        default: result = '0;
    endcase
end
```

### casez

Treats `z` or `?` in a case item as don't-care. `x`/`z` on the data side (the case
expression) still take part in the comparison as they are.

```verilog
// priority encoder — casez with ? patterns
always @(*) begin
    casez (req)          // req[3:0]
        4'b1???: grant = 4'b1000;   // bit3 has priority
        4'b01??: grant = 4'b0100;
        4'b001?: grant = 4'b0010;
        4'b0001: grant = 4'b0001;
        default: grant = 4'b0000;
    endcase
end
```

### casex ⚠️ — do not use in RTL

`casex` treats `x`/`z`/`?` as don't-care in the case item **and** in the case
expression. An X value propagated through the simulation can activate a branch that was
never intended.

```verilog
// ❌ casex hazard
reg [1:0] sig = 2'bxx;  // value is x before initialisation

always @(*) begin
    casex (sig)
        2'b1?: y = a;   // x matches x as don't-care → this branch can activate
        2'b0?: y = b;
        default: y = c;
    endcase
end
// simulation gives y=a, the synthesised hardware gives y=c → mismatch
```

**Recommendation**: when you need don't-care, use `casez` with `?`. The rule is to
replace `casex` with `casez` throughout a codebase.

### case comparison summary

| Construct | z/? in case item | x in case item | x/z in expression |
|------|----------------|--------------|-----------------|
| `case` | compared as is | compared as is | compared as is |
| `casez` | don't-care | compared as is | compared as is |
| `casex` | don't-care | don't-care | don't-care ⚠️ |

---

## full_case / parallel_case pragmas ⚠️

### Definitions

- `full case`: every possible value of the case expression matches one of the case
  items → synthesis infers no default latch
- `parallel case`: no combination matches two or more case items at once
  → synthesis implements parallel logic (no priority)

### The trap — simulation ignores them completely

```verilog
// ❌ pragma use — hazardous
always @(*) begin
    case (state) // synthesis full_case parallel_case
        2'b00: next = 2'b01;
        2'b01: next = 2'b10;
        2'b10: next = 2'b00;
        // 2'b11 is missing — the pragma tells synthesis to build no latch
        // the simulator ignores the comment → next holds on 2'b11 (latch behaviour)
    endcase
end
```

A simulator treats `// synthesis ...` as an **ordinary comment**. Only the synthesis
tool interprets it. The result is a **simulation/synthesis mismatch**: the
pre-synthesis simulation and the post-synthesis gate netlist behave differently.

### The correct alternative

```verilog
// ✅ full case: add a default
always @(*) begin
    case (state)
        2'b00: next = 2'b01;
        2'b01: next = 2'b10;
        2'b10: next = 2'b00;
        default: next = 2'b00;   // the simulator handles it too
    endcase
end

// ✅ if parallel case is what you mean: still use case, but guarantee in the
//    design itself that no two items can be true at the same time
```

**Conclusion**: the `full_case` / `parallel_case` pragmas are the anti-pattern Cliff
Cummings (SNUG 1999) named "the Evil Twins of Verilog Synthesis". Do not use them in
new code.

---

## Loops

### for

A Verilog `for` is a static repetition — the loop body is unrolled at elaboration time.

```verilog
integer i;
always @(*) begin
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i])
            count = count + 1;
    end
end
```

### while

Repeats while the condition holds. To be synthesizable, the iteration count must be
statically determined.

```verilog
integer cnt;
always @(*) begin
    cnt = 0;
    while (cnt < 8) begin
        result[cnt] = data[cnt] ^ key[cnt];
        cnt = cnt + 1;
    end
end
```

### repeat

Repeats a given number of times.

```verilog
// testbench: wait for 8 clock pulses
initial begin
    repeat (8) @(posedge clk);
    $display("8 clocks elapsed");
end
```

### forever

Loops indefinitely. Used where `always` is not available (inside `initial`, inside a
task). It **must contain a delay or an event control** — without one the simulation
hangs.

```verilog
initial begin
    forever begin
        @(posedge clk);
        // check on every rising clock edge
        if (dut_error) $error("DUT error detected at %0t", $time);
    end
end
```

---

## disable

`disable` terminates a named block or a task early. It is the equivalent of C's
`break` / `return`.

```verilog
// find the position of the first 1 bit
integer idx;
always @(*) begin : search_loop
    idx = -1;
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i]) begin
            idx = i;
            disable search_loop;   // leave the loop once found
        end
    end
end
```

Calling `disable task_name;` with a task name terminates that task immediately.

---

## fork-join

`fork-join` runs several statements **in parallel**. It is used inside `initial` /
`always` blocks.

### Verilog: fork-join (wait for all)

The only form IEEE 1364 provides. The parent process blocks until every child thread
has finished.

```verilog
initial begin
    fork
        #10 a = 1;   // thread 1: a=1 at t=10
        #20 b = 1;   // thread 2: b=1 at t=20
        #30 c = 1;   // thread 3: c=1 at t=30
    join
    // reached at t=30, once all threads are done
    $display("all done at t=%0t", $time);
end
```

### SystemVerilog extensions: fork-join_any / fork-join_none

Not in IEEE 1364 (Verilog); added by IEEE 1800 (SystemVerilog). Available to a
testbench written against the SystemVerilog language.

```verilog
// fork-join_any: the main thread resumes when the first thread completes
//                the remaining threads keep running in the background
initial begin
    fork
        #10 a = 1;
        #20 b = 1;
        #30 c = 1;
    join_any
    // reached at t=10 (only a=1 has completed) — threads b and c are still running
end

// fork-join_none: the main thread resumes immediately after spawning (fire-and-forget)
initial begin
    fork
        monitor_bus();    // keeps running in the background
        drive_stimulus(); // keeps running in the background
    join_none
    // reached immediately — both threads are running in the background
end
```

### disable fork

Terminates every active thread spawned in the current scope, immediately.

```verilog
initial begin
    fork
        #100 a = 1;
        #200 b = 1;
    join_any
    // after the first one finishes, kill the rest
    disable fork;
end
```

### fork-join variants compared

| Construct | Standard | Resumes when | Remaining threads |
|------|------|---------|------------|
| `fork...join` | IEEE 1364 | all complete | — |
| `fork...join_any` | IEEE 1800 (SV) | the first completes | keep running in background |
| `fork...join_none` | IEEE 1800 (SV) | never waits (immediate) | keep running in background |

---

## Timing Control (Delay / Event Control)

### delay control (#)

```verilog
#10 a = 1;          // a=1 after 10 time units
#0  b = 2;          // runs last at the same time step (Inactive region)

// intra-assignment delay: the RHS is evaluated immediately, the LHS updates after d
data = #5 bus_in;   // sample the current bus_in → store into data 5 later
```

A `#0` delay is used to avoid race conditions between statements executing in the same
Active region, but overusing it makes code hard to follow.

### event control (@)

```verilog
@(posedge clk)           // wait for a rising edge
@(negedge rst_n)         // wait for a falling edge
@(a or b or c)           // resume when any of a, b, c changes
@(*)                     // resume when any signal read inside changes (Verilog-2001)
@(posedge clk or posedge rst) // multiple edges

// named event: a user-defined event trigger
event ev_done;
-> ev_done;          // trigger the event
@(ev_done);          // wait for the event
```

### wait (level-sensitive)

```verilog
wait (ready == 1);    // block until ready becomes 1 (level-sensitive)
wait (count == 10);
```

`@(posedge)` resumes once, on an edge; `wait` passes straight through if the condition
is already true.

### Timing control compared

| Construct | Mechanism | Resumes on |
|------|------|---------|
| `#d` | delay based | d time units having elapsed |
| `@(posedge/negedge)` | edge detection | an edge on the signal |
| `@(signal)` | change detection | a change in the signal's value |
| `wait(cond)` | level detection | cond becoming true (immediately if already true) |

---

## Sources

- IEEE 1364-2001 §9.4–§9.6 (if, case, loop), §9.7 (timing control), §9.8 (fork-join)
- IEEE 1800-2017 §12 (procedural statements), §9.3 (fork-join_any/none)
- verilogpro.com/verilog-case-casez-casex/ (casex hazard, casez recommended — verified by WebFetch)
- eclipse.umbc.edu/robucci/cmpe316/lectures/L08__Full_and_Parallel_Case/ (full/parallel_case pragmas)
- verificationacademy.com: full_case vs parallel_case vs casex vs casez forum thread
- Cliff Cummings, "full_case parallel_case, the Evil Twins of Verilog Synthesis", SNUG 1999
- vlsiverify.com/verilog/procedural-timing-control/ (delay/event control — verified by WebFetch)
- chipverify.com/systemverilog/systemverilog-fork-join (the three fork-join forms — verified by WebFetch)
- theoctetinstitute.com/content/verilog/loops/ (for/while/repeat/forever)
