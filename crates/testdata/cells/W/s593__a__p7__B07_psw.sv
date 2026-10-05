`timescale 1ns/1ns
package pa;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PS ==? 4'sb1?00);
endpackage
module t;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (pb::RB)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
