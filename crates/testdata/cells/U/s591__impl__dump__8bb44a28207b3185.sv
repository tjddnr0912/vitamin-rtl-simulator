package pa; localparam W = 5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage
import pa::*;
module X3;
  import pb::W;
  wire [W[3:0]:0] v;
  initial begin
    $display("W=%h bits_v=%0d eq=%b", W, $bits(v), (W == 65'h1_0000_0000_0000_0007));
    #1 $finish;
  end
endmodule
