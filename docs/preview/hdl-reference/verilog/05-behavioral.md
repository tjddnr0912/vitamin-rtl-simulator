# 05 · Verilog Behavioral Statements

Per IEEE 1364-2001/2005. `initial` and `always` are Verilog's two procedural
execution contexts. Inside them, the choice of assignment operator — blocking (`=`)
vs. nonblocking (`<=`) — decides both the synthesis result and the simulation
accuracy.

---

## initial vs. always

| Aspect | `initial` | `always` |
|------|----------|--------|
| Times executed | once at simulation t=0, then finishes on its own | repeats forever (an infinite loop) |
| Main use | testbench initialization, stimulus generation | describing RTL logic |
| Synthesizable | ❌ not synthesizable | ✅ (when the conditions are met) |
| Several of them | allowed — every initial starts in parallel at t=0 | allowed — every always runs in parallel |

```verilog
// initial — testbench initialization
initial begin
    clk = 0;
    rst = 1;
    #20 rst = 0;
    #200 $finish;
end

// always — clock generation
always #5 clk = ~clk;   // a 10 time-unit period
```

---

## The sensitivity list of an always block

The sensitivity list triggers the always block whenever one of the signals listed
inside `@(...)` changes.

### Level sensitive (combinational logic)

```verilog
// explicit list — the Verilog-1995 style
always @(a or b or sel) begin
    y = sel ? a : b;
end

// implicit "everything" (@*) — recommended since Verilog-2001
always @(*) begin
    y = sel ? a : b;    // a, b and sel are added automatically
end
```

`@(*)` puts every signal **read** inside the block into the sensitivity list
automatically. Leave a signal out of an explicit list and simulation behaves like a
latch while synthesis builds a multiplexer — a **simulation/synthesis mismatch**.

### Edge sensitive (sequential logic)

```verilog
// synchronous reset
always @(posedge clk) begin
    if (rst)
        q <= 0;
    else
        q <= d;
end

// asynchronous reset (active-low, negedge)
always @(posedge clk or negedge rst_n) begin
    if (!rst_n)
        q <= 0;
    else
        q <= d;
end
```

| Keyword | Trigger condition |
|--------|-----------|
| `posedge sig` | a 0→1 transition of sig (includes x/z→1 and 0→x) |
| `negedge sig` | a 1→0 transition of sig (includes x/z→0 and 1→x) |
| `sig` (no edge) | any value change of sig |

---

## blocking (=) vs. nonblocking (<=)

### Blocking assignment (=)

Evaluates the RHS, updates the LHS immediately, then moves to the next statement.
It executes in order, in the scheduler's **Active region**.

```verilog
always @(*) begin
    // combinational logic — use blocking
    a = b & c;
    y = a | d;   // a already holds b&c here
end
```

### Nonblocking assignment (<=)

Samples (evaluates) the RHS in the **Active region** and schedules the LHS update
for the **NBA region**. Every nonblocking update of that time step is then applied
together in the NBA region.

```verilog
always @(posedge clk) begin
    // sequential logic — use nonblocking
    b <= a;   // Active: sample a. NBA: b ← a_old
    c <= b;   // Active: sample b (still its old value). NBA: c ← b_old
end
```

For the details of the NBA region and the stratified event queue, see
[06-simulation-engine.md](../../06-simulation-engine.md).

### The canonical shift register

```verilog
// ✅ a correct 4-bit shift register — nonblocking
module shift4 (
    input        clk,
    input        d,
    output reg   q3
);
    reg q0, q1, q2;

    always @(posedge clk) begin
        q0 <= d;    // NBA: q0 ← d
        q1 <= q0;   // NBA: q1 ← the old q0
        q2 <= q1;   // NBA: q2 ← the old q1
        q3 <= q2;   // NBA: q3 ← the old q2
    end
    // result: one bit shifts right on every clock
endmodule
```

```verilog
// ❌ a broken shift register — blocking
always @(posedge clk) begin
    q0 = d;     // q0 = d immediately
    q1 = q0;   // q0 is already d → q1 becomes d too
    q2 = q1;   // likewise d
    q3 = q2;   // so q3 = d as well (no shift; all four hold d)
end
```

### Choosing the assignment operator

| Situation | Operator | Why |
|------|--------|------|
| combinational logic (`always @(*)`) | `=` blocking | expresses the sequential RHS dependency |
| sequential logic (`always @(posedge clk)`) | `<=` nonblocking | the NBA region separation guarantees flip-flop behavior |
| mixing both in one always | ❌ forbidden | a source of simulation/synthesis mismatch |
| testbench stimulus | `=` blocking | guarantees a deterministic stimulus order |

---

## Points to watch

**A variable assigned inside an always block must be a reg.** (Unlike
SystemVerilog's `logic`, a Verilog `wire` can only be driven by a continuous
assignment.)

```verilog
wire  y_wire;
reg   y_reg;

assign y_wire = a & b;       // ✅ wire → continuous assign
always @(*) y_reg = a & b;  // ✅ reg → blocking inside an always
```

**Nonblocking assignments in an initial block**: legal syntax, but they go through
NBA region scheduling, so blocking assignments make testbench stimulus more
predictable.

---

## Sources

- IEEE 1364-2001 §9.7–§9.8 (procedural blocks), §9.10 (sensitivity list)
- IEEE 1800-2017 §9.2 (initial/always), §9.4 (event control), §10.4 (blocking), §10.4.2 (non-blocking)
- chipverify.com/verilog/verilog-always-block (sensitivity list — verified by WebFetch)
- chipverify.com/verilog/verilog-blocking-non-blocking-statements (NBA mechanism — verified by WebFetch)
- analogcircuitdesign.com/verilog-blocking-and-non-blocking/ (timing semantics)
- eclipse.umbc.edu/robucci/cmpe316/lectures/L04__VerilogIntroII/ (initial vs. always)
- 06-simulation-engine.md (NBA region details, stratified event queue)
