package pa; localparam W = 5; endpackage
package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage
module sub #(parameter [64:0] P = 0) (); initial #0 $display("sub.P=%h", P); endmodule
module X6;
  import pa::*;
  import pb::W;
  localparam int LI = W[3:0];
  localparam [64:0] LW = W;
  sub #(.P(W)) u ();
  wire [64:0] n = W;
  initial begin
    #0;
    $display("W=%h LI=%0d LW=%h n=%h sel=%h top=%h", W, LI, LW, n, W[3:0], X6.W);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
