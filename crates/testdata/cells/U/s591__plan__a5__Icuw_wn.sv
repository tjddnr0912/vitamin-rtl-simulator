package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
import pb::*;
module top;
  import pa::*;
  localparam int K = P;
  initial #1 $display("cuwwn P=%0d K=%0d b=%0d", P, K, $bits(P));
  initial #100 $finish;
endmodule
