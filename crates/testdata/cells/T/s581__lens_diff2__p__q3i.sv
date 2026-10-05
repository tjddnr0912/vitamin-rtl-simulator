module top;
  logic [(4'bx100 inside {4'b1?00, 4'bx100}):0] i1;
  logic [(4'bx100 inside {4'bx100, 4'b1?00}):0] i2;
  logic [(4'b0100 inside {4'b1?00, 4'bx100}):0] i3;
  logic [((4'bx100 ==? 4'b1?00) || (4'b0100 ==? 4'b0100)):0] i6;
  logic [!(4'bx100 !=? 4'bx100):0] i8;
  logic [((4'bx100 ==? 4'b1?00) === (4'bx100 ==? 4'b1?00)):0] i10;
  logic [(4'bx100 inside {4'b1?00, 4'b0100, 4'bx100}):0] i11;
  logic [((4'bx100 ==? 4'b1?00) == (4'bx100 ==? 4'b1?00) || 1'b1):0] i12;
  initial #1 $display("@ i1=%0d i2=%0d i3=%0d i6=%0d i8=%0d i10=%0d i11=%0d i12=%0d", $bits(i1), $bits(i2), $bits(i3), $bits(i6), $bits(i8), $bits(i10), $bits(i11), $bits(i12));
endmodule
