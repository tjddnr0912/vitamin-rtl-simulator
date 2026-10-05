module top;
  int y;
  initial begin unique case (y) 0: y = 1; default: y = 2; endcase end
  initial $display("unterminated);
endmodule
