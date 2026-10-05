module top;
  logic [((4'bx100 ==? 4'b1?00) + 1'b0):0] x1;
  logic [~(4'bx100 ==? 4'b1?00):0] x3;
  logic [((4'bx100 ==? 4'b1?00) ^ 1'b1):0] x5;
  logic [((4'bx100 ==? 4'b1?00) * 2):0] x9;
  logic [(-(4'bx100 ==? 4'b1?00)):0] x10;
  logic [((4'bx100 ==? 4'b1?00) ? 1 : 0):0] x11;
  initial #1 $display("@ x1=%0d x3=%0d x5=%0d x9=%0d x10=%0d x11=%0d", $bits(x1), $bits(x3), $bits(x5), $bits(x9), $bits(x10), $bits(x11));
endmodule