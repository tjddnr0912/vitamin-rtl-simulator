package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
interface ifc;
  import pa::*;
  import pb::P;
  logic [P[3:0]:0] v;
  initial #1 $display("ifnw %m P=%0d bv=%0d b=%0d", P, $bits(v), $bits(P));
endinterface
module top;
  ifc x ();
  initial #100 $finish;
endmodule
