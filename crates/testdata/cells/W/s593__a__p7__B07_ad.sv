`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  logic [3:0] arr [0:(pb::RB)+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
