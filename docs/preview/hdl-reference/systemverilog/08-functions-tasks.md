# 08 · SystemVerilog Function and Task Extensions

Based on IEEE 1800-2017 §13. This document covers the function and task features
SystemVerilog adds over Verilog-2005. For the Verilog basics see
`../verilog/07-tasks-functions.md`.

> This document describes the language. For which of these constructs vita itself accepts,
> refuses loudly, or supports only in part, see
> [docs/manual/003_language-reference.md](../../../manual/003_language-reference.md).

---

## The main additions over Verilog

| Feature | Verilog-2005 | SystemVerilog |
|------|-------------|---------------|
| void-returning function | ❌ (a return value is mandatory) | ✅ `function void f(...)` |
| ref arguments | ❌ | ✅ `ref type var` |
| const ref arguments | ❌ | ✅ `const ref type var` |
| `let` declaration | ❌ | ✅ compile-time inline |
| `return;` (void) | ❌ | ✅ |
| automatic by default | ❌ (static by default) | automatic by default in a class or program |
| ANSI port style | ❌ | ✅ |

---

## let declarations — compile-time inline (§11.13)

`let` gives a name to an expression inside a module, package or block: a **compile-time inline
substitution**. Unlike a `define` macro it has a **local scope** and is type-safe.

```systemverilog
// basic form: no arguments
let max_addr = (1 << ADDR_W) - 1;

// form with arguments
let compare(a, b) = (a == b) ? "Pass" : "Fail";
let in_range(x, lo, hi) = (x >= lo) && (x <= hi);

// use
$display("max_addr = %0h", max_addr);
$display("%s", compare(exp_data, act_data));
assert (in_range(addr, BASE_ADDR, BASE_ADDR + SIZE - 1));
```

### let vs `define

| Property | `let` | `` `define `` |
|------|-------|--------------|
| Scope | Local to the block, module or package that declares it | The whole file (global) |
| Type checking | ✅ (argument types are inferred) | ❌ |
| Multiple declarations | The same name may be used once per scope | The last declaration overwrites |
| Purpose | Expression reuse, SVA helpers | Textual substitution |

It is frequently used inside assertions:

```systemverilog
// let as an SVA helper
let addr_aligned = (addr[1:0] == 2'b00);
assert property (@(posedge clk) wr_en |-> addr_aligned);
```

---

## void functions (§13.4)

A function with no return value. `return;` exits it early. In Verilog a function had to return
exactly one value; SV lets you declare a function purely for its side effects.

```systemverilog
function void print_state(input logic [1:0] state);
    case (state)
        2'b00: $display("IDLE");
        2'b01: $display("BUSY");
        2'b10: $display("DONE");
        default: begin
            $warning("Unknown state: %02b", state);
            return;    // early exit from a void function
        end
    endcase
endfunction
```

A void function may be called without taking a return value:

```systemverilog
print_state(curr_state);   // return value ignored — legal in SV
```

> **Difference from Verilog**: a Verilog function always has a return value and the result of the call must be used. To call a non-void SV function as if it were void, use the `void'(func_call)` cast.

```systemverilog
void'(some_func_with_return());   // return value explicitly discarded
```

---

## ref arguments — pass by reference (§13.5.2)

An argument declared `ref` passes a reference to the original variable. A change made inside
the function or task is visible to the caller immediately.

```systemverilog
// swap implemented with ref
task automatic swap(ref logic [7:0] a, ref logic [7:0] b);
    logic [7:0] tmp;
    tmp = a;
    a   = b;
    b   = tmp;
endtask

// call
logic [7:0] x = 8'hAA, y = 8'h55;
swap(x, y);   // x = 0x55, y = 0xAA
```

### const ref — a read-only reference

`const ref` passes a reference but forbids modification inside the subroutine. It is used as a
**performance optimization** to pass a large array or structure without copying it by value.

```systemverilog
function automatic logic [31:0] calc_checksum(
    const ref logic [7:0] data [],   // a dynamic array passed without a copy
    input int size
);
    logic [31:0] sum = 0;
    for (int i = 0; i < size; i++)
        sum += data[i];
    return sum;
endfunction
```

### Restrictions on ref arguments

- A `ref` argument may be used **only in an `automatic` subroutine**.
- Using a ref argument in a subroutine with `static` lifetime is a compile error.

```systemverilog
function static void bad_ref(ref int x);   // ❌ compile error
    x = 0;
endfunction

function automatic void ok_ref(ref int x);  // ✅
    x = 0;
endfunction
```

---

## automatic vs static — the lifetime context (§13.4.2)

In Verilog every subroutine has `static` lifetime by default. In SV the default depends on the
context.

| Context | Default lifetime |
|----------|--------------|
| task/function inside a `module` | **static** (Verilog compatibility) |
| a `class` method | **automatic** (§8.6) |
| inside a `program` block | **automatic** |
| a `package` function/task | static (declaring it explicitly is recommended) |
| explicit declaration | overridden with the `automatic`/`static` keyword |

```systemverilog
// inside a module — static is the default, so automatic must be stated
module my_mod;
    task automatic reentrant_task(input int n);
        // recursion is possible; each call gets its own stack frame
        if (n > 0) reentrant_task(n - 1);
    endtask
endmodule

// inside a package — stating automatic is recommended
package util_pkg;
    function automatic int abs_val(input int x);
        return (x >= 0) ? x : -x;
    endfunction
endpackage
```

### When automatic is required

1. **Recursive calls** — under static lifetime the shared local variables misbehave
2. **ref arguments** — a compile error under static lifetime
3. **Parallel task instances** — running a task several times concurrently inside a fork-join
4. **Class methods** — already automatic (no need to state it)

---

## Function and task declaration style — ANSI ports (§13.4~13.5)

Besides the traditional Verilog form, SV supports the more compact ANSI C style declaration.

```systemverilog
// ANSI style (recommended)
function automatic logic [31:0] adder(
    input  logic [31:0] a,
    input  logic [31:0] b,
    output logic        carry
);
    {carry, adder} = {1'b0, a} + {1'b0, b};
endfunction

// task in ANSI style
task automatic wait_for_ack(
    input  logic       clk,
    input  logic       req,
    output logic       ack,
    input  int         timeout_cycles = 100
);
    int cnt = 0;
    while (!ack) begin
        @(posedge clk);
        if (++cnt >= timeout_cycles) begin
            $error("wait_for_ack timeout");
            return;
        end
    end
endtask
```

---

## Related documents

- `../verilog/07-tasks-functions.md` — Verilog function/task basics
- `../system-tasks/` — SV system functions: `$urandom`, `$bits`, `$cast`, `$past`
- `../system-tasks/11-assertion-sampling.md` — `$past`, `$rose`, `$fell`, `$stable`, `$sampled` in detail
- [07-assertions-sva.md](07-assertions-sva.md) — using `let` in SVA
- [05-packages.md](05-packages.md) — the function automatic pattern inside a package
- [06-classes-oop.md](06-classes-oop.md) — class methods (automatic by default)

---

## Sources

- IEEE 1800-2017 §13 (Tasks and functions), §11.13 (let expression)
- asic4u.wordpress.com/2015/12/26/the-let-construct/ — let declarations (WebFetch ✓)
- chipverify.com/systemverilog/systemverilog-functions — ref/void/automatic (WebFetch ✓ partial)
- fpgatutorial.com/systemverilog-functions/ — ANSI style, automatic contexts
