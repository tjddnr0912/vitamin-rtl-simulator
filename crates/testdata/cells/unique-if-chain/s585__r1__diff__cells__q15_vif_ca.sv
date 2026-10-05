interface ifc;
  logic s;
  function void ifv(input logic x, input logic z);
    unique if (x) s = 1;
    else if (z) s = 0;
  endfunction
  task it(input logic x, input logic z);
    unique if (x) s = 1;
    else if (z) s = 0;
  endtask
endinterface
class C;
  virtual ifc vif;
  function new(virtual ifc v); vif = v; endfunction
  function logic [1:0] f(input logic x, input logic z);
    vif.ifv(x, z);
    return {x, z};
  endfunction
endclass
module dut(input logic a, input logic b, output logic [1:0] y);
  ifc u_if();
  C obj; initial obj = new(u_if);
  assign y = obj.f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
