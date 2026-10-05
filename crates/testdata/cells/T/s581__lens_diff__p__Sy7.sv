module top; localparam logic [7:0] V = 8'hA5; initial $display("@psel %b", V[((4'bx100 ==? 4'b1?00) & 1'b0) +: 2]); endmodule
