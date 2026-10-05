package pa; logic [7:0] W = 8'd5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  wire [7:0] A = W;
  import pb::*;
  initial #1 $display("vw A=%0d W=%0d", A, W);
  initial #100 $finish;
endmodule
