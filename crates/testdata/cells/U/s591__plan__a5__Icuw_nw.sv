package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
import pa::*;
module top;
  import pb::*;
  localparam [64:0] K = P;
  initial #1 $display("cuwnw P=%0d K=%0d", P, K);
  initial #100 $finish;
endmodule
