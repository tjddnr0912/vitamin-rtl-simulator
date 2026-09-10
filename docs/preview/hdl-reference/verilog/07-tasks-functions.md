# 07 · Verilog Tasks and Functions

Per IEEE 1364-2001/2005. Two ways to package reusable code: a task is a procedural
subroutine that is allowed to consume time, a function is a pure computation block that
returns a single value in zero time.

---

## Task vs Function — the core differences

| Property | `task` | `function` |
|------|--------|-----------|
| Execution time | may consume time (`#`, `@`, `wait` allowed) | must take zero simulation time |
| Port directions | `input` / `output` / `inout` all allowed | `input` only |
| Return value | none (results come back through output ports) | a single value (assigned to the variable named after the function) |
| Where it may be called | inside a procedural block (`initial`, `always`) | procedural blocks and continuous assignments (`assign`) |
| Minimum port count | zero or more | one or more (at least one `input`) |

A function containing a timing control is a compile error:

```verilog
// ❌ consuming time inside a function — illegal
function [7:0] bad_func(input [7:0] a);
    #10 bad_func = a;    // error: a function cannot use #delay
endfunction
```

---

## Task declaration

Both styles are valid.

### Traditional style (IEEE 1364-1995 compatible)

```verilog
task drive_bus;
    input  [7:0] addr;
    input  [7:0] wdata;
    output [7:0] rdata;
    output       ack;
    begin
        @(posedge clk);          // wait for a clock edge — allowed in a task only
        bus_addr  = addr;
        bus_wdata = wdata;
        @(posedge ack_signal);
        rdata = bus_rdata;
        ack   = 1;
    end
endtask
```

### ANSI port style (IEEE 1364-2001+)

```verilog
task drive_bus(
    input  [7:0] addr,
    input  [7:0] wdata,
    output [7:0] rdata,
    output       ack
);
    @(posedge clk);
    bus_addr  = addr;
    bus_wdata = wdata;
    @(posedge ack_signal);
    rdata = bus_rdata;
    ack   = 1;
endtask
```

### Calling it

```verilog
initial begin
    drive_bus(8'hA0, 8'hFF, read_data, ack_flag);
    $display("rdata=%0h ack=%b", read_data, ack_flag);
end
```

---

## Function declaration

### Traditional style

```verilog
function [7:0] add8;
    input [7:0] a;
    input [7:0] b;
    add8 = a + b;    // assigning to the variable named after the function = the return value
endfunction
```

### ANSI style

```verilog
function [7:0] add8(input [7:0] a, b);
    add8 = a + b;
endfunction
```

### Return value — assignment to the function name

In Verilog-2001 the only way to return a value is to **assign to an internal variable
that has the same name as the function**. The `return` keyword added by SystemVerilog
(IEEE 1800) is not available when parsing plain Verilog-2001:

```verilog
// ✅ plain Verilog-2001: assign to the name
function [31:0] max(input [31:0] a, b);
    max = (a > b) ? a : b;
endfunction

// ✅ SystemVerilog / IEEE 1800: the return keyword
function automatic [31:0] max(input [31:0] a, b);
    return (a > b) ? a : b;
endfunction
```

### Calling it

A function can be called from a continuous assignment and from a procedural block
alike:

```verilog
assign y = add8(sig_a, sig_b);

always @(*) begin
    result = max(data_a, data_b);
end
```

---

## automatic vs static

### static (the default)

The declared local variables are **shared across every call**. Concurrent calls
overwrite each other's values.

```verilog
task static_counter;
    integer count = 0;    // every call shares the same count
    count = count + 1;
    $display("count = %0d", count);
endtask
```

### automatic

Every call gets its own **stack frame**, allocated dynamically. Safe for concurrent
calls and for recursion.

```verilog
task automatic safe_counter;
    integer count = 0;    // a count private to this call
    count = count + 1;
    $display("count = %0d", count);
endtask
```

**Rule**: recursion in a task or function that is not declared `automatic` overwrites
the same variables and produces wrong results. If you need recursion, `automatic` is
mandatory.

---

## Recursion — automatic only

Recursion does not work without the `automatic` keyword:

```verilog
// ✅ automatic function — recursive factorial
function automatic integer factorial(input integer n);
    if (n <= 1)
        factorial = 1;
    else
        factorial = n * factorial(n - 1);
endfunction

// call
initial begin
    $display("4! = %0d", factorial(4));  // prints: 4! = 24
end
```

```verilog
// ✅ automatic task — recursive tree walk
task automatic traverse(input integer node);
    if (node == 0) return;          // SV return; in Verilog-2001 use disable
    $display("node %0d", node);
    traverse(node / 2);
endtask
```

Recursion without a base case runs forever and overflows the simulator's stack.

---

## inout ports

A task may have `inout` ports: it takes the value in, modifies it, and hands it back:

```verilog
task automatic swap(inout [7:0] a, b);
    logic [7:0] tmp;
    tmp = a;
    a   = b;
    b   = tmp;
endtask

initial begin
    logic [7:0] x = 8'hAA, y = 8'h55;
    swap(x, y);
    $display("x=%0h y=%0h", x, y);  // x=55 y=aa
end
```

---

## disable — leaving a task early

`disable task_name;` terminates a running task immediately. It is the equivalent of C's
`return` (Verilog-2001 has no `return`):

```verilog
task automatic find_first(input [7:0] data, output integer idx);
    integer i;
    idx = -1;
    for (i = 0; i < 8; i = i + 1) begin
        if (data[i]) begin
            idx = i;
            disable find_first;    // terminate immediately
        end
    end
endtask
```

---

## System tasks vs user tasks

| Kind | Form | Examples | Synthesis |
|------|------|------|------|
| System task | `$name(...)` | `$display`, `$finish`, `$random` | ❌ simulation only |
| User task | `task...endtask` | user-defined | conditional (if it consumes no time) |

System tasks are defined by the Verilog standard and implemented by the simulator. In
RTL code, wrap them in `` `ifdef `` conditional compilation to mark them as
simulation-only, or move them into a testbench.

```verilog
// ✅ conditionally compiled so the synthesis tool never sees it
`ifdef SIMULATION
initial begin
    $monitor("q = %b", q);
end
`endif
```

---

## Choosing between task and function

```
Do you need to consume time?
  └─ YES → task
  └─ NO  → Do you need more than one output?
              └─ YES → task (using output ports)
              └─ NO  → function
```

---

## Sources

- IEEE 1364-2001 §10.1–§10.3 (tasks and functions)
- IEEE 1800-2017 §13.2–§13.5 (tasks), §13.4 (functions), §13.4.4 (return)
- chipverify.com/verilog/verilog-task (verified by WebFetch ✓)
- chipverify.com/verilog/verilog-functions (verified by WebFetch ✓)
- vlsiverify.com/system-verilog/tasks/ (automatic vs static behaviour compared)
- fpgatutorial.com/verilog-function-and-task/ (guide to choosing between them)
