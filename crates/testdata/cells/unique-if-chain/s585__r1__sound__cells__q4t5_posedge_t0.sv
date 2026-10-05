module top;
  logic clk; logic a, b; logic [1:0] r;
  initial clk = 1;
  always @(posedge clk) begin r = 0; unique if (a) r = 1; else if (b) r = 2; end
  initial begin a = 0; b = 1; #1 clk = 0; #1 b = 0; clk = 1; #1 $display("t=%0t r=%0d", $time, r); #1 $finish; end
endmodule
