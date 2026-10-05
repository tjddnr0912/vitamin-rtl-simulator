module top;
  logic clk, a, b; logic [1:0] r;
  initial clk = 1;
  assert property (@(posedge clk) 1'b0) else begin r = 0; if (a) r = 1; else unique if (b) r = 2; $display("act t=%0t r=%0d", $time, r); end
  initial begin a = 0; b = 1; #1 clk = 0; #1 b = 0; clk = 1; #1 $finish; end
endmodule
