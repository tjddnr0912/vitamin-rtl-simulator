module top;
  logic clk; int n = 0;
  initial clk = 0;
  always #5 clk = ~clk;
  always @(posedge clk) n++;
  initial begin #22 $display("t=%0t n=%0d clk=%b", $time, n, clk); $finish; end
  initial #100 $finish;
endmodule
