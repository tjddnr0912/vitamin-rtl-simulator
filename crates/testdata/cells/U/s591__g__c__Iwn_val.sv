package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pa::*;
  import pb::P;
  localparam int K = P;
  localparam [64:0] KW = P + 65'd1;
  wire [64:0] wv = P;
  initial #1 $display("val P=%0d K=%0d KW=%0d wv=%0d b=%0d sh=%0d", P, K, KW, wv, $bits(P), P >> 60);
  initial #100 $finish;
endmodule
