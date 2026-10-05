module top;
  int y;
  initial begin unique case (y) 0: y = 1; default: y = 2; endcase end
  initial y = 3 § 4;
endmodule
