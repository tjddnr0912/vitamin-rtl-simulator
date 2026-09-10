# 03 · Verilog Expressions and Operators

Per IEEE 1364-2005 §4.1–§4.4. An expression is built from operands and operators;
the order of evaluation follows operator precedence and associativity.

---

## Full operator precedence table

Level 1 is the highest (evaluated first). Operators of equal precedence evaluate
left to right, except `**`, which is right to left.

| Precedence | Operator | Kind | Associativity |
|---------|--------|------|------|
| 1 (highest) | `( )` `[ ]` | parentheses, bit/part select | — |
| 2 | `!` `~` unary `+` unary `-` `&` `~&` `\|` `~\|` `^` `~^` | unary / reduction | right → left |
| 3 | `**` | power | right → left |
| 4 | `*` `/` `%` | multiply, divide, modulus | left → right |
| 5 | `+` `-` | binary add/subtract | left → right |
| 6 | `<<` `>>` `<<<` `>>>` | logical/arithmetic shift | left → right |
| 7 | `<` `<=` `>` `>=` | relational | left → right |
| 8 | `==` `!=` `===` `!==` | equality | left → right |
| 9 | `&` | binary bitwise AND | left → right |
| 10 | `^` `~^` / `^~` | binary bitwise XOR / XNOR | left → right |
| 11 | `\|` | binary bitwise OR | left → right |
| 12 | `&&` | logical AND | left → right |
| 13 | `\|\|` | logical OR | left → right |
| 14 (lowest) | `? :` | conditional (ternary) | right → left |

### Precedence in practice

```verilog
// a + b << 2 does NOT parse as a + (b << 2) — shift does not outrank addition
// In fact: + is level 5 and << is level 6, so + binds first → (a + b) << 2
// → if a + (b << 2) is what you meant, the parentheses are mandatory
assign y = a + (b << 2);

// & versus == is the other classic trap: a & b == c parses as a & (b == c)
// because == outranks &
assign z = (a & b) == c;   // parenthesized to state the intent
```

---

## Operators by category

### Arithmetic operators

| Operator | Meaning | Notes |
|--------|------|------|
| `+` | addition | |
| `-` | subtraction, unary negation | |
| `*` | multiplication | |
| `/` | integer division (fraction discarded) | divisor 0 → `x` |
| `%` | modulus | divisor 0 → `x` |
| `**` | power (1364-2001+) | base 0 with a negative exponent → `x` |

### Bitwise operators

Bit-by-bit operations; the result width is that of the wider operand.

| Operator | Meaning |
|--------|------|
| `~` | bitwise NOT |
| `&` | bitwise AND |
| `\|` | bitwise OR |
| `^` | bitwise XOR |
| `~^` / `^~` | bitwise XNOR |

### Logical operators

The result is always 1 bit (`1'b0` or `1'b1`). An operand of 0 is false, anything
non-zero is true.

| Operator | Meaning |
|--------|------|
| `!` | logical NOT |
| `&&` | logical AND |
| `\|\|` | logical OR |

### Reduction operators

Unary operators that combine every bit of a vector into a 1-bit result.

| Operator | Meaning | Example (4-bit input `4'b1010`) |
|--------|------|-----------------------------|
| `&a` | AND of all bits | `1 & 0 & 1 & 0` = `0` |
| `~&a` | NAND of all bits | `~(1&0&1&0)` = `1` |
| `\|a` | OR of all bits | `1\|0\|1\|0` = `1` |
| `~\|a` | NOR of all bits | `~(1\|0\|1\|0)` = `0` |
| `^a` | even parity (XOR) | `1^0^1^0` = `0` |
| `~^a` | odd parity (XNOR) | `~(1^0^1^0)` = `1` |

```verilog
wire [7:0] data;
wire parity = ^data;    // even parity over 8 bits
wire all_one = &data;   // are all bits 1?
wire any_bit = |data;   // is it non-zero? (boolean conversion)
```

### Shift operators

| Operator | Kind | Vacated bits filled with |
|--------|------|-------------|
| `<<` | logical left shift | 0 |
| `>>` | logical right shift | 0 |
| `<<<` | arithmetic left shift (1364-2001+) | 0 |
| `>>>` | arithmetic right shift (1364-2001+) | the MSB if signed, 0 if unsigned |

```verilog
reg [7:0] a = 8'sb1111_0000;  // -16
reg [7:0] b;

b = a >> 2;    // logical: fills 0 → 8'b0011_1100 = +60
b = a >>> 2;   // arithmetic: fills the sign bit (1) → 8'b1111_1100 = -4
```

### Comparison operators

The result is 1 bit: true = 1, false = 0, undetermined = x.

| Operator | Meaning | With x/z operands |
|--------|------|------------|
| `<` | less than | `x` |
| `<=` | less than or equal | `x` |
| `>` | greater than | `x` |
| `>=` | greater than or equal | `x` |
| `==` | logical equality | `x` |
| `!=` | logical inequality | `x` |
| `===` | case equality | 0 or 1 (x/z compared exactly, as values) |
| `!==` | case inequality | 0 or 1 (x/z compared exactly, as values) |

Because `===` and `!==` compare x and z as values in their own right, they are
simulation-only constructs (not synthesizable — a synthesis tool will error or warn
if they appear in RTL).

### Concatenation and replication

```verilog
{a, b}          // concatenate the bits of a and b
{a, 4'b0000}    // append four 0 bits after a
{4{a[1:0]}}     // a[1:0] repeated 4 times (8 bits)
{2{a}, 3{b}}    // mixed
```

An unsized constant (`'0`, `'1`, `'x`, `'z`) expands to the context width inside a
concatenation (a 1364-2001+ feature — some tools do not support it).

### The conditional (ternary) operator

```verilog
y = condition ? true_expr : false_expr;

// examples
assign mux_out = sel ? a : b;
assign safe_div = (divisor != 0) ? (dividend / divisor) : 0;
```

When `condition` is `x` or `z`, the two arms are merged bit by bit:
- where the two arms agree on a bit position, that value is kept
- where they differ, the bit is `x`

---

## Signed arithmetic rules (IEEE 1364-2001+)

### The basic principle

An expression's result is **signed only if every operand is signed**. If even one
operand is unsigned, the whole expression is converted to unsigned before the
operation.

```verilog
reg signed [7:0] a = -8;   // 8'sb1111_1000
reg        [7:0] b = 200;  // 8'b1100_1000

// a + b: b is unsigned → the whole operation is unsigned
// a is read as 256-8=248 → 248+200 = 448 (truncated to 8 bits: 192)
```

### Declaring something signed

```verilog
reg  signed [7:0]  acc;         // signed reg
wire signed [15:0] offset;      // signed wire (1364-2001+)
parameter signed [7:0] BIAS = -10;  // signed parameter

// signed literals
4'sd5     // 4-bit signed +5
8'sb1000  // 8-bit signed -128
```

### $signed / $unsigned conversion

```verilog
$signed(a)    // use a as signed in the operation (bit pattern unchanged)
$unsigned(a)  // use a as unsigned (bit pattern unchanged)
```

```verilog
reg [7:0] u = 200;          // unsigned
integer result;
result = $signed(u) + 1;    // 200 read as signed (= −56) → −55
```

### Extension rules

When operands in an expression differ in width, the narrower one is widened to match:
- unsigned → zero extension
- signed → sign extension (the MSB repeats)

---

## x/z propagation rules

The result of an expression containing x (unknown) or z (high-Z) during simulation:

| Operation | Behavior |
|------|------|
| arithmetic (`+`, `-`, `*`, `/`, `%`, `**`) | any x or z operand → the entire result is x |
| `/`, `%` with divisor 0 | result x |
| relational (`<`, `<=`, `>`, `>=`) | any x or z operand → a 1-bit x result |
| logical equality (`==`, `!=`) | any x or z operand → a 1-bit x result |
| case equality (`===`, `!==`) | compares x/z as values → always 0 or 1 |
| logical (`&&`, `\|\|`, `!`) | an x operand → x, unless logic already decides it (`1 \|\| x = 1`) |
| bitwise AND (`&`) | `0 & x = 0`, `1 & x = x`, `x & x = x`; `z` is treated as `x` |
| bitwise OR (`\|`) | `1 \| x = 1`, `0 \| x = x`, `x \| x = x` |
| bitwise XOR (`^`) | any x or z operand → that bit is x |
| shift (`<<`, `>>`, `<<<`, `>>>`) | an x or z in the shift amount (right operand) → the entire result is x |
| conditional (`? :`) | an x or z condition → the arms merge bitwise (differing positions become x) |
| reduction | an x in the input is treated as x for that bit and affects the result |

### Bitwise AND truth table with x/z (1 bit)

| a \ b | 0 | 1 | x | z |
|-------|---|---|---|---|
| 0 | 0 | 0 | **0** | **0** |
| 1 | 0 | 1 | **x** | **x** |
| x | **0** | **x** | x | x |
| z | **0** | **x** | x | x |

`0 & x = 0` because 0 is FALSE, which already determines the AND (an exception to x
propagation).

### Bitwise OR truth table with x/z (1 bit)

| a \ b | 0 | 1 | x | z |
|-------|---|---|---|---|
| 0 | 0 | 1 | **x** | **x** |
| 1 | 1 | 1 | **1** | **1** |
| x | **x** | **1** | x | x |
| z | **x** | **1** | x | x |

`1 | x = 1` because 1 is TRUE, which already determines the OR (an exception to x
propagation).

---

## Sources

- IEEE 1364-2005 §4.1–§4.4 (Expressions)
- IEEE 1800-2017 §11 (Operators — Verilog-compat)
- chipverify.com/verilog/verilog-operators
- vlsiverify.com/verilog/verilog-operators/
- hdlworks.com/hdl_corner/verilog_ref/items/SignedArithmetic.htm
- staff.washington.edu/kd1uj/BEE271/Lectures/ (Signed numbers lecture)
