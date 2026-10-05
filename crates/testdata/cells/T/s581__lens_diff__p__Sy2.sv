module m (input logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] a); initial #1 $display("@port %0d %b", $bits(a), a); endmodule
module top; m u (.a(1'b1)); endmodule
