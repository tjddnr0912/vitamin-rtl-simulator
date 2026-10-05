`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  if ($signed(B2) ==? 4'sb1?00) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
