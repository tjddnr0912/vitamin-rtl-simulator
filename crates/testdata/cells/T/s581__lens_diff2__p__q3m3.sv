module top;
  logic [((4'bx100 ==? 4'b1?00) << 1):0] v11;
  logic [((4'bx100 ==? 4'b1?00) ? 1'b1 : 1'b1):0] v15;
  initial #1 $display("@ v11=%0d v15=%0d", $bits(v11), $bits(v15));
endmodule