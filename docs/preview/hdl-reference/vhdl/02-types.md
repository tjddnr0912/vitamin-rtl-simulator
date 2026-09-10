# 02 · VHDL Type System

Per IEEE 1076-2008 §5, including the `std` and `ieee` packages.

---

## Type classification

```
type
├── scalar
│   ├── integer                 — integer, natural, positive
│   ├── floating point (real)
│   ├── physical                — time
│   └── enumeration             — bit, boolean, character, severity_level, file_open_kind
├── composite
│   ├── array                   — bit_vector, string, ...
│   └── record
├── access                      — pointer, simulation only
└── file                        — simulation only
```

---

## Scalar Types

### Integer types

```vhdl
-- as declared in the standard package
type integer is range -(2**31-1) to 2**31-1;

subtype natural  is integer range 0 to integer'high;
subtype positive is integer range 1 to integer'high;
```

| Type | Range |
|------|------|
| `integer` | −(2³¹−1) … 2³¹−1 (an implementation may make it wider) |
| `natural` | 0 and up |
| `positive` | 1 and up |

### Floating-point type

```vhdl
type real is range -1.0E308 to 1.0E308;
```

Simulation and verification only; not synthesizable. The precision left after an operation is
implementation-defined.

### Physical types

An integer carrying a unit. Essential for modelling time.

```vhdl
-- the standard package definition of time (abridged)
type time is range -(2**63-1) to 2**63-1
  units
    fs;                 -- femtosecond (the primary unit)
    ps  = 1000 fs;
    ns  = 1000 ps;
    us  = 1000 ns;
    ms  = 1000 us;
    sec = 1000 ms;
    min = 60 sec;
    hr  = 60 min;
  end units;
```

In use:

```vhdl
constant CLK_PERIOD : time := 10 ns;
wait for 5 ns;
signal t_now : time := now;
```

### Enumeration types

```vhdl
type boolean        is (FALSE, TRUE);
type bit            is ('0', '1');
type severity_level is (NOTE, WARNING, ERROR, FAILURE);
type file_open_kind is (READ_MODE, WRITE_MODE, APPEND_MODE);
type file_open_status is (OPEN_OK, STATUS_ERROR, NAME_ERROR, MODE_ERROR);
```

`character` is an enumeration of the 256 ISO-8859-1 characters (since VHDL-1993).

**User-defined enumeration**:
```vhdl
type state_t is (IDLE, FETCH, DECODE, EXECUTE, WRITEBACK);
signal state : state_t := IDLE;
```

---

## Composite Types

### Arrays

**Constrained array**: the range is fixed at the declaration.

```vhdl
type byte_t      is array(7 downto 0) of bit;
type word_t      is array(15 downto 0) of bit;
type rom_256x8_t is array(0 to 255) of byte_t;
```

**Unconstrained array**: `range <>` — the range is settled by a port, a generic or a subtype.

```vhdl
-- as declared in the standard package
type bit_vector  is array(natural range <>) of bit;
type string      is array(positive range <>) of character;

-- the range given at the port declaration
port (data_in : in bit_vector(7 downto 0));

-- constrained by a subtype
subtype byte_vec is bit_vector(7 downto 0);
```

**Multidimensional arrays**:
```vhdl
type matrix_t is array(0 to 3, 0 to 3) of integer;
variable m : matrix_t;
m(0, 0) := 1;
```

**Array attributes**:
| Attribute | Meaning |
|------|------|
| `a'length` | number of elements |
| `a'left` | left index |
| `a'right` | right index |
| `a'high` | highest index |
| `a'low` | lowest index |
| `a'range` | the range (`low to high` or `high downto low`) |
| `a'reverse_range` | the range reversed |

### Records

A bundle of heterogeneous fields — the counterpart of a SystemVerilog `struct`.

```vhdl
type axi_t is record
  data    : std_logic_vector(31 downto 0);
  valid   : std_logic;
  ready   : std_logic;
  last    : std_logic;
end record;

signal axi_bus : axi_t;
axi_bus.valid <= '1';
```

---

## Access Types

A pointer into dynamically allocated memory. **Simulation only, not synthesizable.**

```vhdl
type node_t;
type link_ptr is access node_t;
type node_t is record
  val  : integer;
  nxt  : link_ptr;
end record;

variable head : link_ptr;
head := new node_t'(val => 0, nxt => null);
```

`deallocate(ptr)` releases it.

---

## File Types

Simulation I/O. **Not synthesizable.**

```vhdl
type text is file of string;   -- as declared in the standard package
file my_file : text open READ_MODE is "input.txt";
```

---

## The std_logic_1164 Package

`library ieee; use ieee.std_logic_1164.all;`

IEEE Std 1164 — the standard logic type of VHDL design.

### std_ulogic — a 9-value enumeration

```vhdl
type std_ulogic is ('U','X','0','1','Z','W','L','H','-');
```

| Value | Name | Meaning in simulation |
|----|------|----------------|
| `'U'` | Uninitialized | the state before initialization; the default at the start of simulation |
| `'X'` | Forcing Unknown | a strong-driver conflict, or an undetermined value |
| `'0'` | Forcing 0 | a strong logic 0 (tied to ground) |
| `'1'` | Forcing 1 | a strong logic 1 (tied to VCC) |
| `'Z'` | High Impedance | tri-state: the driver is off |
| `'W'` | Weak Unknown | a weak-driver conflict |
| `'L'` | Weak 0 | a pull-down resistor |
| `'H'` | Weak 1 | a pull-up resistor |
| `'-'` | Don't Care | a hint for synthesis optimization; in simulation it behaves as `'X'` |

The values a synthesis tool actually recognizes are `'0'`, `'1'`, `'Z'` and `'-'`.

### std_logic — the resolved subtype

```vhdl
function resolved(s : std_ulogic_vector) return std_ulogic;
subtype std_logic is resolved std_ulogic;
```

`std_ulogic` permits a single driver only. `std_logic` supports multiple drivers (a shared
bus) through the `resolved` function, which returns the representative value from a 9×9
decision table.

The main cases of the resolution table:

| Driver A | Driver B | Result |
|-----------|-----------|------|
| `'0'` | `'0'` | `'0'` |
| `'1'` | `'1'` | `'1'` |
| `'0'` | `'1'` | `'X'` |
| `'0'` | `'Z'` | `'0'` |
| `'1'` | `'Z'` | `'1'` |
| `'Z'` | `'Z'` | `'Z'` |
| `'H'` | `'0'` | `'0'` |
| `'L'` | `'1'` | `'1'` |
| `'U'` | (any) | `'U'` |
| `'-'` | (any) | `'X'` |

### Array types

```vhdl
type std_ulogic_vector is array (natural range <>) of std_ulogic;
-- VHDL-2008: std_logic_vector is a subtype of std_ulogic_vector
subtype std_logic_vector is (resolved) std_ulogic_vector;
```

Before VHDL-2008, `std_logic_vector` and `std_ulogic_vector` were distinct types, so assigning
one to the other required an explicit type conversion (`std_logic_vector(...)`). From 2008 on
they are related as subtype and base type, so the conversion is implicit.

### New in VHDL-2008

```vhdl
-- reduction operators
and_reduce(v)   -- AND of all elements
or_reduce(v)    -- OR of all elements
xor_reduce(v)   -- odd parity

-- matching comparison (honours the don't-care '-')
a ?= b    -- matching equality
a ?/= b   -- matching inequality

-- conversion functions
to_string(v)    -- "10110..."
to_hstring(v)   -- "FF"
to_ostring(v)   -- "377"
to_bstring(v)   -- same as to_string
```

---

## The numeric_std Package

`library ieee; use ieee.numeric_std.all;`

IEEE Std 1076.3 — synthesizable integer arithmetic.

### Type declarations

```vhdl
type unsigned is array (natural range <>) of std_logic;
type signed   is array (natural range <>) of std_logic;
```

| Type | Interpretation | Range (n bits) |
|------|------|-------------|
| `unsigned` | unsigned integer | 0 … 2ⁿ−1 |
| `signed` | two's complement | −2ⁿ⁻¹ … 2ⁿ⁻¹−1 |

### The main operations

```vhdl
-- arithmetic
a + b     -- same type, same length
a - b
a * b     -- the result width is the sum of the operand widths
abs a     -- signed only

-- comparison (returns std_logic)
a < b   a <= b   a > b   a >= b   a = b   a /= b

-- bit manipulation
shift_left(a, n)    shift_right(a, n)
rotate_left(a, n)   rotate_right(a, n)

-- width conversion
resize(a, new_size)   -- signed: sign-extend; unsigned: zero-extend

-- type conversion
to_integer(u)              -- unsigned/signed → integer
to_unsigned(n, size)       -- integer → unsigned (size bits)
to_signed(n, size)         -- integer → signed (size bits)
std_logic_vector(u)        -- unsigned → slv (type conversion)
unsigned(slv)              -- slv → unsigned
signed(slv)                -- slv → signed
```

### Mixed-type operations are illegal

`unsigned` and `signed` cannot be combined directly; an explicit conversion is required.

```vhdl
-- error
result <= u_val + s_val;

-- correct
result <= u_val + unsigned(resize(s_val, u_val'length));
```

---

## Subtypes

A subtype adds a constraint to an existing type, or just names it.

```vhdl
subtype byte_t     is integer range 0 to 255;
subtype nibble_vec is std_logic_vector(3 downto 0);
```

A subtype is treated as the same type as its parent, so it can be assigned without a
conversion.

---

## Type Conversion

Explicit conversion between closely related types:

```vhdl
integer(r)              -- real → integer (rounds)
real(i)                 -- integer → real
std_logic_vector(u)     -- unsigned → slv
unsigned(slv)           -- slv → unsigned
to_integer(u)           -- unsigned → integer (numeric_std)
to_unsigned(i, n)       -- integer → unsigned, n bits
```

---

## Sources

- IEEE 1076-2008 §5 (Types)
- IEEE Std 1164 (std_logic_1164) — hdlworks.com ✓, hdlfactory.com ✓
- IEEE Std 1076.3 (numeric_std) — hdlfactory.com ✓
