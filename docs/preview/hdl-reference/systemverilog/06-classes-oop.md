# 06 · SystemVerilog Classes and OOP

Based on IEEE 1800-2017 §8 (classes) and §18 (randomization).

> **Non-synthesizable only**: every feature covered in this document — classes, inheritance,
> virtual methods, parameterized classes, static members, randomization — is **100%
> non-synthesizable**. It belongs to simulation, verification and UVM testbenches only.
> Put a class in RTL design code and the synthesis tool raises an error.

---

## Basic class declaration (§8.3)

```systemverilog
class Packet;
    // instance variables (members)
    bit [31:0] addr;
    bit [31:0] data;
    int        id;

    // constructor — no return type, non-blocking
    function new(int init_id = 0);
        addr = 32'hDEAD_BEEF;
        data = 32'h0;
        id   = init_id;
    endfunction

    // method
    function void display();
        $display("[Packet] id=%0d addr=%08h data=%08h", id, addr, data);
    endfunction
endclass
```

Creating and using an object:

```systemverilog
Packet pkt;          // handle declaration — initial value is null
pkt = new(42);       // allocates the object on the heap, id=42
pkt.display();

Packet pkt2 = new;   // declare and construct at once, default arguments
```

A handle behaves much like a pointer. Release a handle with `pkt = null` and the garbage
collector reclaims objects that nothing references.

---

## The new() constructor (§8.7)

`new()` is a class's only constructor. It has no return type and cannot be overloaded. It may
take arguments, and default values make them optional.

```systemverilog
class Config;
    int width;
    int depth;

    function new(int w = 8, int d = 16);
        width = w;
        depth = d;
    endfunction
endclass

Config c1 = new;          // width=8, depth=16 (defaults)
Config c2 = new(32);      // width=32, depth=16
Config c3 = new(64, 128); // width=64, depth=128
```

### The this keyword (§8.11)

`this` refers to the current object instance. It disambiguates when a constructor argument and
a class member share a name.

```systemverilog
class Packet;
    int id;
    function new(int id);
        this.id = id;   // this.id = the class member, id = the argument
    endfunction
endclass
```

---

## Inheritance — extends / super.new() (§8.13)

```systemverilog
class BasePacket;
    int size;
    logic [7:0] kind;

    function new(int s = 64, logic [7:0] k = 8'h01);
        size = s;
        kind = k;
    endfunction

    function void show();
        $display("BasePacket: size=%0d kind=%02h", size, kind);
    endfunction
endclass

class ExtPacket extends BasePacket;
    logic [31:0] payload;

    function new(int s = 64, logic [31:0] p = 0);
        super.new(s);       // explicit parent constructor call — must be the first executable statement
        payload = p;
    endfunction

    function void show();
        super.show();       // call the parent method
        $display("ExtPacket: payload=%08h", payload);
    endfunction
endclass
```

**super.new() rules**:
- It must be the **first executable statement** of the child constructor.
- If the parent constructor requires arguments, they must be passed as `super.new(args)`.
- If the child constructor has no `super.new()`, the compiler inserts `super.new()` on the
  first line automatically (only when the parent constructor is callable with no arguments).

---

## Polymorphism — virtual methods (§8.20)

Override without `virtual` and the call is **statically dispatched** — resolved against the
handle's declared type. Add `virtual` and the call is **dynamically dispatched** — resolved
against the actual object type.

```systemverilog
class Shape;
    virtual function real area();
        return 0.0;
    endfunction
    virtual function void print();
        $display("Shape area = %0.2f", area());
    endfunction
endclass

class Circle extends Shape;
    real radius;
    function new(real r); radius = r; endfunction
    function real area();
        return 3.14159 * radius * radius;   // virtual override
    endfunction
endclass

class Rect extends Shape;
    real w, h;
    function new(real w, real h); this.w = w; this.h = h; endfunction
    function real area();
        return w * h;                        // virtual override
    endfunction
endclass

// polymorphism in use
Shape s;
Circle c = new(5.0);
Rect   r = new(4.0, 3.0);

s = c;  s.print();   // "Shape area = 78.54" — calls Circle::area()
s = r;  s.print();   // "Shape area = 12.00" — calls Rect::area()
```

**Virtual method rules**:
- Once the parent declares a method `virtual`, repeating the `virtual` keyword on the child's
  override is optional (it is inherited).
- If a non-`virtual` method is overridden in a child, calling through a parent handle runs the
  parent method.

---

## Parameterized classes (§8.25)

A generic class is defined with integer parameters or type parameters.

### Integer parameters

```systemverilog
class FIFO #(int DEPTH = 8, int WIDTH = 8);
    logic [WIDTH-1:0] mem [DEPTH];
    int               head = 0;
    int               tail = 0;
    int               count = 0;

    function void push(logic [WIDTH-1:0] val);
        if (count < DEPTH) begin
            mem[tail] = val;
            tail = (tail + 1) % DEPTH;
            count++;
        end
    endfunction

    function logic [WIDTH-1:0] pop();
        if (count > 0) begin
            pop = mem[head];
            head = (head + 1) % DEPTH;
            count--;
        end
    endfunction
endclass

// instantiation
FIFO #(16, 32) deep_fifo;
FIFO #(.DEPTH(4)) nibble_fifo;   // WIDTH keeps its default of 8
```

### Type parameters

```systemverilog
class Stack #(type T = int);
    T    items [];
    int  sp = 0;

    function void push(T val);
        items = new[sp + 1](items);
        items[sp++] = val;
    endfunction

    function T pop();
        return items[--sp];
    endfunction
endclass

// instantiated with several types
Stack #(bit [3:0]) nibble_stack = new;
Stack #(real)      float_stack  = new;
Stack              int_stack    = new;   // T = int (the default)

// an alias via typedef
typedef Stack #(logic [7:0]) ByteStack;
ByteStack bstack = new;
```

---

## static members (§8.9)

A `static` variable is shared by every instance of the class. A `static` method can be called
directly through the class name, with no instance.

```systemverilog
class Packet;
    static int obj_count = 0;   // shared by every instance
    int id;

    function new();
        obj_count++;
        id = obj_count;
    endfunction

    // static method — cannot reach a non-static member (id)
    static function int get_count();
        return obj_count;
        // id = 1;  // compile error: cannot access a non-static member
    endfunction

    function void display();
        $display("id=%0d / total=%0d", id, obj_count);
    endfunction
endclass

// use
Packet p1 = new;
Packet p2 = new;
Packet p3 = new;

$display("total = %0d", Packet::get_count());  // 3
p1.display();  // id=1 / total=3
```

---

## The scope resolution operator :: (§23.8)

`::` is used to reference an item inside a class scope from outside the class, or to reach the
specialized scope of a parameterized class.

```systemverilog
// calling a static method
Packet::get_count();

// direct access to a static variable
$display("count = %0d", Packet::obj_count);

// parameterized class: the specialization must be named
FIFO #(16, 32)::DEPTH;        // ✓ reaches the specialized class scope
// FIFO::DEPTH;               // ✗ unspecialized scope — compile warning or error

// calling a parent method from a child class
// (super is the :: variant that names the parent scope from inside)
super.show();
```

---

## Randomization — rand / randc / constraint (§18)

> **Non-synthesizable**: the whole randomization feature set is non-synthesizable. Simulation
> only.

### rand and randc

Declare a class member `rand` or `randc` and it is randomized automatically on a
`randomize()` call.

```systemverilog
class Transaction;
    rand  bit [7:0]  opcode;   // uniform random on every call
    randc bit [2:0]  tag;      // cycles through 0..7, visiting all before repeating
    rand  int        length;
    rand  logic [31:0] addr;
endclass
```

| Declaration | Behaviour |
|------|------|
| `rand`  | An independent uniformly distributed random value on every `randomize()` call |
| `randc` | Repeats a value only after cycling through every possible value (like a deck of cards) |
| (none)  | Excluded from `randomize()` |

### constraint blocks (§18.5)

A constraint block is declared inside the class and states the conditions the random variables
must satisfy. Several constraint blocks are implicitly ANDed together.

```systemverilog
class Packet;
    rand bit [7:0]  opcode;
    rand bit [31:0] addr;
    rand int        length;

    constraint opcode_c {
        opcode inside {8'h01, 8'h02, 8'h04, 8'h08};  // a specific value set
    }

    constraint addr_c {
        addr[1:0] == 2'b00;     // 4-byte aligned
        addr inside {[32'h1000 : 32'hFFFF]};  // range limit
    }

    constraint length_c {
        length > 0;
        length <= 256;
        length % 4 == 0;        // a multiple of 4
    }

    // correlated constraint
    constraint correlated_c {
        if (opcode == 8'h04)    // if opcode is 0x04
            length == 64;       //  then length must be 64
    }
endclass
```

### The randomize() method

```systemverilog
Packet pkt = new;

// plain call — always check the return value
if (!pkt.randomize())
    $fatal(1, "Randomization failed!");

// assert form (recommended)
assert(pkt.randomize()) else $fatal(1, "Randomization failed!");
```

### Inline constraints with `with` (§18.7)

Adds a one-off constraint at the `randomize()` call. It is ANDed with the existing constraints
of the class.

```systemverilog
// a one-off constraint pinning addr to a specific value
assert(pkt.randomize() with { addr == 32'hA000; });

// a more involved inline constraint
assert(pkt.randomize() with {
    opcode == 8'h01;
    length inside {[64:128]};
});
```

### pre_randomize / post_randomize (§18.8)

Hook methods called automatically before and after `randomize()`. Use `pre_randomize()` to set
constraints up and `post_randomize()` to compute derived fields.

```systemverilog
class Packet;
    rand  bit [31:0] addr;
    rand  int        length;
    bit   [31:0]     end_addr;   // derived field (not randomized)

    function void pre_randomize();
        // just before randomize() — enable/disable constraints, etc.
        $display("before randomize");
    endfunction

    function void post_randomize();
        // just after randomize() — compute derived fields
        end_addr = addr + length - 1;
    endfunction
endclass
```

### Constraint inheritance and overriding (§18.9)

A child class inherits its parent's constraints. A constraint block of the same name overrides
the parent's.

```systemverilog
class ExtPacket extends Packet;
    // override the parent's addr_c with a wider range
    constraint addr_c {
        addr inside {[32'h0 : 32'hFFFF_FFFF]};  // the full range is allowed
    }
endclass
```

Disabling a constraint block:

```systemverilog
pkt.opcode_c.constraint_mode(0);  // disable one constraint
pkt.randomize();
pkt.opcode_c.constraint_mode(1);  // enable it again
```

---

## Class-based verification patterns

The basic UVM-style structure:

```systemverilog
// transaction class
class APBTxn;
    rand bit [31:0] addr;
    rand bit [31:0] data;
    rand bit        write;

    constraint addr_aligned { addr[1:0] == 2'b00; }
endclass

// driver class (uses a virtual interface — see 04-interfaces.md)
class APBDriver;
    virtual apb_if.TB vif;

    function new(virtual apb_if.TB handle);
        vif = handle;
    endfunction

    task automatic drive(APBTxn txn);
        @(vif.cb);
        vif.cb.paddr   <= txn.addr;
        vif.cb.pwdata  <= txn.data;
        vif.cb.pwrite  <= txn.write;
        vif.cb.psel    <= 1;
        vif.cb.penable <= 1;
        @(vif.cb);
        vif.cb.psel    <= 0;
        vif.cb.penable <= 0;
    endtask
endclass

// test sequence
initial begin
    APBTxn txn;
    APBDriver drv = new(dut_bus.TB);

    repeat (10) begin
        txn = new;
        assert(txn.randomize()) else $fatal(1, "rand failed");
        drv.drive(txn);
    end
end
```

---

## Sources

- IEEE 1800-2017 §8 (Classes), §18 (Constrained random value generation), §23.8 (Scope resolution)
- chipverify.com/systemverilog/systemverilog-class-constructor
- chipverify.com/systemverilog/systemverilog-constraints
- chipverify.com/systemverilog/systemverilog-randomization-methods
- chipverify.com/systemverilog/systemverilog-static-variables-functions
- chipverify.com/systemverilog/systemverilog-parameterized-classes
- verificationguide.com/systemverilog/systemverilog-constraints/
- dvcon-proceedings.org/the-top-most-common-systemverilog-constrained-random-gotchas.pdf
