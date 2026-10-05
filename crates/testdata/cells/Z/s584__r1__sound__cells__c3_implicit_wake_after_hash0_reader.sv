module top;
  logic s, g; wire h; assign h = g;
  always_comb s = 1'b1;
  always @(s) g = s;
  initial begin #0 $display("P t=%0t h=%b g=%b s=%b", $time, h, g, s); end
  initial #5 $finish;
endmodule
