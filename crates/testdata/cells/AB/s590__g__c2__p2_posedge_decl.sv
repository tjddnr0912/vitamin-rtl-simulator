module top;
  logic a = 0, b = 1; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  always @(posedge y[0]) $display("P t=%0t y=%b", $time, y);
  always @(negedge y[0]) $display("N t=%0t y=%b", $time, y);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
