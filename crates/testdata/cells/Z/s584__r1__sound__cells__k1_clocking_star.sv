module top;
  logic clk, a;
  clocking cb @(*); endclocking
  always @(cb) $display("K t=%0t a=%b", $time, a);
  initial begin a = 1; #1 a = 0; #1 $finish; end
endmodule
