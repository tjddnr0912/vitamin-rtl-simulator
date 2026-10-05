package p; localparam int X = 1; endpackage
module top;
  import p::*;
  if (1) begin : b
    initial #1 $display("@l2 X=%0d", X);
    localparam int X = 2;
    initial #1 $display("@l4 X=%0d", X);
  end
  initial #5 $finish;
endmodule
