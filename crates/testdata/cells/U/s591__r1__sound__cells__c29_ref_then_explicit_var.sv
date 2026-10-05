package pa; localparam [64:0] W = 65'h1_0000_0000_0000_0009; endpackage
package pb; logic [7:0] W = 8'd5; endpackage
module top;
  import pa::*;
  localparam [64:0] A = W;
  import pb::W;
  initial #1 $display("xv A=%0d W=%0d b=%0d", A, W, $bits(W));
  initial #100 $finish;
endmodule
