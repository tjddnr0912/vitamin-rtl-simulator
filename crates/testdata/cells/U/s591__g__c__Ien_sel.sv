package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  import pa::*;
  initial #1 $display("sel s0=%b s64=%b lo=%0d", P[0], P[64], P[3:0]);
  initial #100 $finish;
endmodule
