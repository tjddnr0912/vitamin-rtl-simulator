package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  localparam P = 3;
  localparam int K = P;
  localparam [64:0] KW = P + 65'd1;
  wire [64:0] wv = P;
  initial #1 $display("val P=%0d K=%0d KW=%0d wv=%0d b=%0d sh=%0d", P, K, KW, wv, $bits(P), P >> 60);
  if (P > 65'd100) begin : t initial #1 $display("gif big"); end
  else begin : e initial #1 $display("gif small"); end
  initial #100 $finish;
endmodule
