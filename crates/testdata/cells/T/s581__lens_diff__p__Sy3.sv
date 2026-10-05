module top; wire [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] w = '1; initial #1 $display("@wire %0d %b", $bits(w), w); endmodule
