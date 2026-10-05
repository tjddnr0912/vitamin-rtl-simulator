module top; typedef logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] t_t; t_t v = '1; initial #1 $display("@typedef %0d %b", $bits(t_t), v); endmodule
