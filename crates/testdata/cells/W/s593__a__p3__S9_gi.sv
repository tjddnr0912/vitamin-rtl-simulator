`timescale 1ns/1ns
module t;
  localparam int I = 12;
  if (I inside {4'b1?00, 4'b0011}) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
