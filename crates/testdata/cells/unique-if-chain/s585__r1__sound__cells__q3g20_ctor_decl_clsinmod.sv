module dut(input logic a, input logic b, output logic [1:0] y);
  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  class C;
    logic [1:0] v;
    function new(input logic x, input logic z);
      g(x, z);
      v = {x, z};
    endfunction
  endclass
  C obj = new(a, b);
  assign y = obj.v;
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
