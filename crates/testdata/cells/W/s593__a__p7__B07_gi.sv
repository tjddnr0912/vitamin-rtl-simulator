`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  if (pb::RB) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
