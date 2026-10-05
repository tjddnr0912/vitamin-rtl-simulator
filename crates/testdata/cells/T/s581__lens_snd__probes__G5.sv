module top;
  logic [1 + (4'bx100 ==? 4'b1?00):0] v;
  initial begin $display("G5 %0d", $bits(v)); end
endmodule
