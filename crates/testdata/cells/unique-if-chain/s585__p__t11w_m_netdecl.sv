module dut(input logic a, input logic b, output wire [1:0] y);
  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g(x, z);
    return {x, z};
  endfunction
  wire [1:0] w = f(a, b);
  assign y = w;
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
