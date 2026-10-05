module top;
  typedef logic [1 + (4'bx100 ==? 4'b1?00):0] t_t;
  t_t v;
  initial $display("G9 %0d", $bits(v));
endmodule
