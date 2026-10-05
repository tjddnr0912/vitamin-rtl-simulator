package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc; localparam P = 7; endpackage
module top;
  import pa::*;
  import pb::*;
  import pc::P;
  initial #1 $display("aax P=%0d b=%0d", P, $bits(P));
  initial #100 $finish;
endmodule
