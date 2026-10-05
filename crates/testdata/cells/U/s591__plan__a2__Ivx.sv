package pa; localparam [64:0] W = 65'h1_0000_0000_0000_0009; endpackage
package pb; logic [7:0] W = 8'd5; endpackage
module top;
  import pa::*;
  import pb::W;
  initial #1 $display("vx W=%0d b=%0d", W, $bits(W));
  initial #100 $finish;
endmodule
