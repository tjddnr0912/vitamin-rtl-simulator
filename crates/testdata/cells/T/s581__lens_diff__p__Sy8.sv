module top; logic [3:0] v = 4'b1010; initial $display("@bsel %b %0d", v[((4'bx100 ==? 4'b1?00) & 1'b0)], $bits({((4'bx100 ==? 4'b1?00) & 1'b0) {1'b1}})); endmodule
