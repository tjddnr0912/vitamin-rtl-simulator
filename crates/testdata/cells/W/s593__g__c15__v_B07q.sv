package pa;
  localparam logic [39:0] PW = 40'h10_0000_000C;
  localparam logic [39:0] PF = 40'h0C;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage
package pb;
  import pa::*;
  localparam RB = ((4'd15 + 4'd1) ==? 5'b1?000);
endpackage
module top;
  localparam R = pb::RB;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
