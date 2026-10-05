module top;
  logic s, u;
  always_comb s = 1'b1;
  always @(s) $display("R t=%0t s=%b u=%b", $time, s, u);
  initial begin #0 u = 1; end
  initial #5 $finish;
endmodule
