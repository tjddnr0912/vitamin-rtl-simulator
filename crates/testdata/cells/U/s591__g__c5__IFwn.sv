package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
interface ifc;
  import pa::*;
  import pb::P;
  localparam int K = P;
  initial #1 $display("ifwn %m P=%0d K=%0d b=%0d", P, K, $bits(P));
endinterface
module top;
  ifc x ();
  initial #100 $finish;
endmodule
