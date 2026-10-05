`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  logic [(pb::RB)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
