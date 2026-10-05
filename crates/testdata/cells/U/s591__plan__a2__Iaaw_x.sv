package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pd; localparam [64:0] P = 65'h1_0000_0000_0000_0004; endpackage
module top;
  import pa::*;
  import pb::*;
  import pd::P;
  localparam [64:0] Q = P;
  initial #1 $display("aawx P=%0d Q=%0d b=%0d", P, Q, $bits(P));
  initial #100 $finish;
endmodule
