module top;
  logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] v;
  initial begin v = '1; $display("@bits %0d v=%b", $bits(v), v); end
endmodule
