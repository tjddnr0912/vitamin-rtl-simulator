module top #(parameter logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] P = 1'b1) (); initial $display("@P %0d %b", $bits(P), P); endmodule
