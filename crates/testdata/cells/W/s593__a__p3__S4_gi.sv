`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  if (U ==? (4'b1?00)) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
