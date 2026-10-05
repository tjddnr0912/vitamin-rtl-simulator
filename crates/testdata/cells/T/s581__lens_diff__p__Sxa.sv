module top;
  logic [((4'bx100 ==? 4'b1?00) && 1'b0) : 0] v;
  logic arr [((4'bx100 ==? 4'b1?00) && 1'b0) + 1];
  initial $display("@bits %0d size %0d", $bits(v), $size(arr));
  initial $display("@rt %b", ((4'bx100 ==? 4'b1?00) && 1'b0));
endmodule
