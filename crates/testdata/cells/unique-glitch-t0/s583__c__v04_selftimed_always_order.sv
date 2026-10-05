module top;
  logic clk;
  always begin $display("gen t=%0t", $time); clk = 0; #5 clk = 1; #5; end
  initial $display("init t=%0t clk=%b", $time, clk);
  initial #1 $finish;
endmodule
