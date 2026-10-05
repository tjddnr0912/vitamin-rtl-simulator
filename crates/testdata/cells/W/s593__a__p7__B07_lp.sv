`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  localparam L = (pb::RB);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
