module top;
  logic [((4'bx100 ==? 4'b1?00) + 1'b0):0] x1;
  logic [{1'b0, (4'bx100 ==? 4'b1?00)}:0] x2;
  logic [~(4'bx100 ==? 4'b1?00):0] x3;
  logic [!(4'bx100 ==? 4'b1?00):0] x4;
  logic [((4'bx100 ==? 4'b1?00) ^ 1'b1):0] x5;
  logic [$signed(4'bx100 ==? 4'b1?00):0] x6;
  logic [1'(4'bx100 ==? 4'b1?00):0] x7;
  logic [((4'bx100 ==? 4'b1?00) == 1'b1):0] x8;
  logic [((4'bx100 ==? 4'b1?00) * 2):0] x9;
  logic [(-(4'bx100 ==? 4'b1?00)):0] x10;
  logic [((4'bx100 ==? 4'b1?00) ? 1 : 0):0] x11;
  logic [(4'bx100 !=? 4'b1?00):0] x12;
  logic [(4'bx100 ==? 4'b1?00):0] x13;
  initial #1 $display("@ %0d %0d %0d %0d %0d %0d %0d %0d %0d %0d %0d %0d %0d", $bits(x1), $bits(x2), $bits(x3), $bits(x4), $bits(x5), $bits(x6), $bits(x7), $bits(x8), $bits(x9), $bits(x10), $bits(x11), $bits(x12), $bits(x13));
endmodule
