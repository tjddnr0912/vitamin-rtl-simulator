`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  case (1'b1)
    (pb::RB): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
