package pa;
  localparam logic [39:0] PW = 40'h10_0000_000C;
  localparam logic [39:0] PF = 40'h0C;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = (PF ==? 4'b1?00);
endpackage
module top;
  localparam R = pb::RB;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
