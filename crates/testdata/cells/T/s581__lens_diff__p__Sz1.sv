module top;
  logic [(1'b1 ? 0 : (4'bx100 ==? 4'b1?00)) : 0] v1;
  logic [(1'b0 && (4'bx100 ==? 4'b1?00)) : 0] v2;
  logic [(1'b1 || (4'bx100 ==? 4'b1?00)) : 0] v3;
  initial $display("@b %0d %0d %0d", $bits(v1), $bits(v2), $bits(v3));
endmodule
