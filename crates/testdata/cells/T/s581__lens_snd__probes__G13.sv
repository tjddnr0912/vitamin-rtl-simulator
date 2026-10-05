module top;
  localparam logic [1 + (4'bx100 ==? 4'b1?00):0] LP = 0;
  initial $display("G13 %0d", $bits(LP));
endmodule
