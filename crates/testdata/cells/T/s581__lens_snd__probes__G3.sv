module top;
  logic [7:0] x = 8'hA5;
  initial begin $display("G3 %b", (2 + (4'bx100 ==? 4'b1?00))'(x)); end
endmodule
