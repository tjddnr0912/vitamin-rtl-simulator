module top;
  localparam L1 = (4'bx100 ==? 4'b1?00) & 1'b0;
  localparam L2 = (4'bx100 != 4'b1100) & 1'b0;
  initial $display("@L %b %b", L1, L2);
endmodule
