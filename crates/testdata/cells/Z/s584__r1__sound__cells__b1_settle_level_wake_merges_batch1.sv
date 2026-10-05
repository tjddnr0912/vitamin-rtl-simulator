module top;
  wire w; assign w = 1'b1;
  logic s, t, u; wire v; assign v = u;
  always @(w) s = 1;
  always @(s) $display("R t=%0t v=%b u=%b", $time, v, u);
  always @(t) u = 1;
  initial t = 1;
  initial #5 $finish;
endmodule
