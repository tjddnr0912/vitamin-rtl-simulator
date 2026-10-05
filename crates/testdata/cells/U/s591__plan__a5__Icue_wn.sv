package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
import pb::P;
module top;
  import pa::P;
  localparam [64:0] K = P;
  initial #1 $display("cuewn P=%0d K=%0d b=%0d", P, K, $bits(P));
  initial #100 $finish;
endmodule
