module top;
  logic s, y;
  always_comb begin y = s; $display("C t=%0t s=%b", $time, s); end
  initial begin s = 1'b1; $fatal(1, "boom"); end
  final $display("F t=%0t y=%b", $time, y);
endmodule
