package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::*;
  import pa::*;
  localparam int K = P;
  initial #1 $display("val P=%0d K=%0d", P, K);
  initial #100 $finish;
endmodule
