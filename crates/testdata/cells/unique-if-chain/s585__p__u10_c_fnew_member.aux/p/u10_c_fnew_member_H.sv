class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = 0;
    if (x) r = 1; else unique if (z) r = 2;
  endfunction
endclass
class C;
  D d;
  function logic [1:0] f(input logic x, input logic z);
    d = new(x, z);
    return d.r;
  endfunction
endclass
module dut(input logic a, input logic b, output logic [1:0] y);
  C obj; initial obj = new;
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
