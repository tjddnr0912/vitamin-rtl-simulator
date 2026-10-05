package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  import pa::*;
  sub #(.P(P)) u ();
  initial #100 $finish;
endmodule

module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
module sub2 #(parameter N = 0) (input logic [N:0] a);
  initial #3 $display("port %m b=%0d", $bits(a));
endmodule
