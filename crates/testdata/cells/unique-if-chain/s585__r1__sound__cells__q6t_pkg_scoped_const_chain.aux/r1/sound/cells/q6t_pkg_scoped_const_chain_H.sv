package pk;
  localparam int MODE = 2;
  function void g();
    if (MODE == 0) begin end else unique if (MODE == 1) begin end
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g();
    return {x, z};
  endfunction
endpackage
module dut(input logic a, input logic b, output logic [1:0] y);
  assign y = pk::f(a, b);
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
