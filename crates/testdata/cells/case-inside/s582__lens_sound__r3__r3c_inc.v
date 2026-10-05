module top;
  `include "r3c_inc.vh"
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd2;
    case (x) inside[1:0]: m = 1; default: m = 0; endcase
    $display("r3c include m=%0d", m); #1 $finish; end
endmodule
