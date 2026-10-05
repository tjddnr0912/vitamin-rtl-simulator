`timescale 1ns/1ns
package pa; localparam W = 5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage
module t;
  import pa::*;
  import pb::W;
  localparam [64:0] D = W + 65'd1;
  initial begin
    $display("W=%h D=%h bits=%0d hi=%b", W, D, $bits(W), W[64]);
    #1 $finish;
  end
endmodule
