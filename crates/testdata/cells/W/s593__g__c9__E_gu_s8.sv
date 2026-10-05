`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  if (X ==? 4'b1?00) begin : g initial $display("GI=then"); end else begin : h initial $display("GI=else"); end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
