module top;
  logic [(4'b1100 ==? {2'b1?, 2'b00}):0] v;
  initial $display("MD1 %0d", $bits(v));
endmodule
