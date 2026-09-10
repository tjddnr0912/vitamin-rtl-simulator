# 03 · SystemVerilog Procedural Statements

Per IEEE 1800-2017 §9/§12/§13. SV adds three variants of Verilog's `always`/`initial` that state
the design intent (`always_comb/ff/latch`), adds case/if qualifiers, extends the loop forms, and
improves how arguments are passed to functions and tasks.

---

## always_comb / always_ff / always_latch

Verilog expressed combinational, sequential and latch logic all through the one `always @(*)`;
SV separates them. A tool warns or errors when the stated intent and the actual code disagree.

### always_comb (§9.2.2.2)

For combinational logic only.

```systemverilog
always_comb begin
    y = a & b;
    case (sel)
        2'b00: out = x;
        2'b01: out = y;
        default: out = '0;
    endcase
end
```

**Semantics**:
- Implicit sensitivity list: every signal **read** inside the block is included automatically.
  Signals assigned on the LHS are excluded.
- Runs once automatically at time 0 (`always @*` waits for the first change event).
- A single driver per signal is enforced at compile time — another `always` or an `assign`
  writing the same variable is a compile error.
- Blocking timing controls (`#delay`, the event `@`) are not allowed.

**Note**: signals read inside a function the block calls are not part of the sensitivity list.
Mind that a change to a signal inside the function does not retrigger the block.

### always_ff (§9.2.2.4)

For clock-edge registers (flip-flops) only.

```systemverilog
always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) q <= '0;
    else        q <= d;
end
```

**Semantics**:
- Exactly one event control `@(...)` is allowed. No further event or delay inside the block.
- Nonblocking assignment (`<=`) is the recommended form (tools warn on blocking assignment).
- Synthesis tools infer flip-flops.

### always_latch (§9.2.2.3)

For level-sensitive latches only.

```systemverilog
always_latch begin
    if (en) q <= d;   // transparent only while en=1; holds its value when en=0
end
```

**Semantics**:
- The same implicit sensitivity rule as `always_comb`.
- An incomplete conditional branch (latch behaviour) is taken to be intended — it suppresses the
  synthesis tool's latch warning.
- If the latch is not intended, `always_comb` or `always_ff` is the right choice.

---

## unique / priority case·if

Verilog's `case`/`if` were open to different readings in simulation and in synthesis. Stating
the intent with one of the two SV qualifiers buys a runtime check and a synthesis hint at once.

### unique case / unique if

```systemverilog
unique case (opcode)
    4'hA: y = a + b;
    4'hB: y = a - b;
    4'hC: y = a & b;
    default: y = '0;
endcase
```

**Runtime check**: exactly one branch must match.
- A warning if no branch matches (when there is no default).
- A warning if two or more branches match at once.

**Synthesis hint**: all branches are mutually exclusive — the tool can simplify without a
priority encoder.

### priority case / priority if

```systemverilog
priority casez (addr)
    8'b1???_????: region = BOOT;
    8'b01??_????: region = ROM;
    default:      region = RAM;
endcase
```

**Runtime check**: the first matching branch runs. A warning if no branch matches.

**Synthesis hint**: priority runs top to bottom — the tool infers a priority encoder.

The `unique0`/`priority0` variants suppress the warning when nothing matches (SV §12.4.2).

---

## foreach

A dedicated loop for walking a whole array. For a multi-dimensional array the indices are listed
comma-separated.

```systemverilog
// one dimension
int arr [8];
foreach (arr[i])
    arr[i] = i * 2;

// multiple dimensions
int mat [4][4];
foreach (mat[r, c])
    mat[r][c] = r * 4 + c;

// dynamic array
int dyn[];
dyn = new[5];
foreach (dyn[k])
    dyn[k] = k;

// associative array
int aa[string];
foreach (aa[k])
    $display("%s = %0d", k, aa[k]);
```

---

## do-while

A loop guaranteed to run at least once.

```systemverilog
int i = 0;
do begin
    $display("i = %0d", i);
    i++;
end while (i < 5);
```

Verilog's `while` tests the condition first, so the body never runs if the condition starts out
false. `do-while` fits the cases where the body has to run at least once.

---

## void function + return

A function with no return value. Unlike a Verilog task it cannot use timing controls, and
`return` can leave it early.

```systemverilog
function void check_range(int val, int lo, int hi);
    if (val < lo || val > hi) begin
        $display("out of range: %0d", val);
        return;           // leaves the function here
    end
    $display("ok: %0d", val);
endfunction
```

- Give a return type — `function int ...` — when a value is needed.
- Return a value with `return expr;`. `return expr;` inside a `void function` is a compile error.

---

## ref / const ref arguments

Verilog functions and tasks passed arguments by value only. SV adds pass by reference.

### ref — pass by reference

Modifies the caller's variable directly. Passing a large array costs nothing to copy. Only
allowed in an `automatic` function or task.

```systemverilog
function automatic void zero_out(ref int arr[]);
    foreach (arr[i]) arr[i] = 0;
endfunction

int data[] = new[1024];
zero_out(data);   // zeroes the caller's own data in place
```

### const ref — read-only reference

Passed by reference but not modifiable; writing to it is a compile error. Use it to pass a large
read-only argument safely.

```systemverilog
function automatic int sum_all(const ref int arr[]);
    int total = 0;
    foreach (arr[i]) total += arr[i];
    // arr[0] = 0;   // compile error
    return total;
endfunction
```

### Comparison

| Passing mode | Keyword | Modifies the caller | Copy cost | Use for |
|--------------|---------|---------------------|-----------|---------|
| By value | (none) | no | O(n) | small scalars |
| By reference | `ref` | yes | O(1) | large arrays, multiple outputs |
| Read-only reference | `const ref` | no | O(1) | large read-only arguments |

---

## Sources

- IEEE 1800-2017 §9 (Processes), §12 (Procedural programming statements), §13 (Tasks and functions)
- verilogpro.com/systemverilog-always_comb-always_ff/
- verilogpro.com/systemverilog-unique-priority/
- vlsiworlds.com/system-verilog/argument-passing-and-the-const-keyword-in-systemverilog/
- chipverify.com/systemverilog/systemverilog-functions
- verificationguide.com/systemverilog/systemverilog-task-function-argument-passing/
