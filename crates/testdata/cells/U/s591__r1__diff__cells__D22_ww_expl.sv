package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0003; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::P;
  initial #1 $display("d22 P=%0d b=%0d", P, $bits(P));
  initial #100 $finish;
endmodule
