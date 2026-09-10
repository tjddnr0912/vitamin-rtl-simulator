# 04 · SystemVerilog Interfaces

Per IEEE 1800-2017 §25. An interface encapsulates a bundle of signals shared between modules as
a single object; a modport states direction and a clocking block states timing. In a large
design this removes the repeated port lists and writes the timing contract with the verification
environment into the code itself.

---

## Basic declaration

An interface is declared as an `interface` ... `endinterface` block and, like a module, can take
external signals such as a clock through a port list.

```systemverilog
interface apb_if (input pclk);
    logic [31:0] paddr;
    logic [31:0] pwdata;
    logic [31:0] prdata;
    logic        penable;
    logic        pwrite;
    logic        psel;
endinterface
```

Receiving an interface on a module port:

```systemverilog
module apb_slave (apb_if.Slave bus);  // restricted to a modport
    always_ff @(posedge bus.pclk) begin
        if (bus.psel && bus.penable && !bus.pwrite)
            bus.prdata <= mem[bus.paddr];
    end
endmodule
```

Instantiating it in the top-level module:

```systemverilog
module tb_top;
    logic clk;
    apb_if dut_bus(.pclk(clk));           // interface instance

    apb_slave slave(.bus(dut_bus.Slave)); // connected through a modport view
endmodule
```

---

## Interface parameters (§25.3.3)

The same `#(parameter ...)` syntax as module parameters.

```systemverilog
interface myBus #(
    parameter int D_WIDTH = 32,
    parameter int A_WIDTH = 32
) (input clk);
    logic [D_WIDTH-1:0] data;
    logic [A_WIDTH-1:0] addr;
    logic               valid;
endinterface
```

Overridden at instantiation:

```systemverilog
myBus #(.D_WIDTH(64), .A_WIDTH(40)) wide_bus(.clk(clk));
```

---

## modport — a directional view (§25.5)

A modport defines a directional view of the signals inside an interface. The same interface can
be split into separate roles — master, slave, testbench, and so on.

```systemverilog
interface bus_if (input clk);
    logic [7:0] data;
    logic       valid;
    logic       ready;

    modport Master (
        output data, valid,
        input  ready, clk
    );

    modport Slave (
        input  data, valid, clk,
        output ready
    );

    modport Monitor (
        input data, valid, ready, clk  // read-only monitor
    );
endinterface
```

**Direction keywords**:

| Keyword | Meaning |
|--------|------|
| `input`  | a signal the user of this modport reads |
| `output` | a signal the user of this modport drives |
| `inout`  | bidirectional (a tri-state bus, for instance) |
| `import` | exposes a task or function of the interface so it can be called through this modport |
| `export` | the module connected through this modport supplies the body of the task or function |

### Importing and exporting tasks and functions in a modport

A task or function can be defined inside the interface and made reachable only from certain
views through a modport.

```systemverilog
interface bus_if (input clk);
    logic [7:0] data;
    logic       valid;

    // a task defined inside the interface
    task automatic wait_valid();
        @(posedge clk);
        while (!valid) @(posedge clk);
    endtask

    modport TB (
        output data,
        input  valid, clk,
        import wait_valid   // the task is callable through this modport
    );
endinterface

// caller side
module monitor(bus_if.TB bus);
    initial begin
        bus.wait_valid();   // reached through the modport import
        $display("data = %0h", bus.data);
    end
endmodule
```

A function brought in from a package can be imported into a modport as well:

```systemverilog
package util_pkg;
    function automatic logic [7:0] flip8(input logic [7:0] d);
        return {d[0],d[1],d[2],d[3],d[4],d[5],d[6],d[7]};
    endfunction
endpackage

interface proc_if;
    import util_pkg::*;
    modport mp (import flip8);  // expose a package function through the modport
endinterface
```

`export` runs the other way from `import` — the connected module supplies the body of the
function (the abstract-interface pattern, §25.9).

---

## clocking block (§14.12)

A clocking block gathers, in one place, the **sampling timing** (input skew) and the **driving
timing** (output skew) of signals relative to a clock event.
A testbench uses it to avoid timing races with the DUT.

### Declaration

```systemverilog
clocking cb @(posedge clk);
    default input #1step output #1;  // input: sampled just before the clock, output: driven 1 ns later
    input  data, valid;
    output ready;
endclocking
```

`default input <skew> output <skew>` sets the default skew applied to every signal. A signal
that needs a different skew can override it on its own line.

### input skew vs output skew

| Item | Direction | Meaning |
|------|------|------|
| `input  #T` | T **before** the clock | samples the signal T before the clock edge |
| `input  #1step` | immediately before the clock | samples in the simulation step just before the clock edge (the recommended default) |
| `output #T` | T **after** the clock | drives the signal T after the clock edge |
| `output #0` | with the clock | drives as soon as nonblocking assignments have settled |

`#1step` samples in the simulation time step immediately before the clock edge, which suits
setup/hold simulation.

### A clocking block and a modport together in an interface

```systemverilog
interface apb_if (input pclk);
    logic [31:0] paddr;
    logic [31:0] pwdata;
    logic [31:0] prdata;
    logic        psel;
    logic        penable;
    logic        pwrite;

    clocking cb @(posedge pclk);
        default input #1step output #1;
        input  prdata;
        output paddr, pwdata, psel, penable, pwrite;
    endclocking

    modport TB  (clocking cb, input pclk);     // testbench: access through the clocking block
    modport DUT (input  paddr, pwdata, psel,   // DUT: direct signal access
                         penable, pwrite,
                 output prdata);
endinterface
```

Including `clocking cb` in a modport forces whoever uses that modport to reach the signals only
through the clocking block, so the timing discipline is enforced.

Signal access through a clocking block is written `vif.cb.signal` or
`vif.cb.signal <= value`.

---

## Virtual interfaces (§25.9)

An interface instance is fixed statically in the module hierarchy.
To reach interface signals from a class-based testbench, which is dynamic, use a
`virtual interface` (a handle).

### Declaring and passing one

```systemverilog
// a virtual interface member inside a class
class Driver;
    virtual apb_if.TB vif;   // a virtual interface handle naming a modport

    function new(virtual apb_if.TB handle);
        vif = handle;        // bind the handle to the real interface instance
    endfunction

    task automatic write(logic [31:0] addr, data);
        @(vif.cb);           // wait for the clocking block event
        vif.cb.paddr   <= addr;
        vif.cb.pwdata  <= data;
        vif.cb.pwrite  <= 1;
        vif.cb.psel    <= 1;
        vif.cb.penable <= 1;
        @(vif.cb);
        vif.cb.psel    <= 0;
        vif.cb.penable <= 0;
    endtask
endclass

// top-level testbench module
module tb_top;
    logic clk = 0;
    always #5 clk = ~clk;

    apb_if dut_bus(.pclk(clk));    // static interface instance

    Driver drv;
    initial begin
        drv = new(dut_bus.TB);     // bind the real instance to the virtual interface
        drv.write(32'h1000, 32'hDEAD);
    end
endmodule
```

**How it works**:

1. `apb_if dut_bus` is created statically in `tb_top`.
2. `new(dut_bus.TB)` — the TB modport handle of the real instance is passed to the Driver constructor.
3. `vif` points at `dut_bus` like a pointer. Several objects can share the same interface.
4. Signals are driven through the clocking block, as in `vif.cb.paddr <= addr`.

**When the module hierarchy is deep**: the standard patterns are `config_db` (UVM) or passing
the handle down hierarchically from the top. `$cast` or a direct port connection is also used.

---

## Sources

- IEEE 1800-2017 §25 (Interfaces), §14.12 (Clocking blocks), §26.3 (Package import)
- chipverify.com/systemverilog/systemverilog-interface
- chipverify.com/systemverilog/systemverilog-modport
- vlsiverify.com/system-verilog/systemverilog-clocking-block/
- vlsiworlds.com/system-verilog/clocking-blocks-and-modports/
- verificationacademy.com/forums (modport import/export tasks, citing IEEE §26.3)
- medium.com/@vimala.learnvlsi (virtual interface in class example)
