module top;
  logic k = 1'b1;
  logic s, g;
  always_comb s = k;
  always @(s) g = s;
  initial begin #0 $display("P t=%0t g=%b s=%b", $time, g, s); end
  initial #5 $finish;
endmodule
