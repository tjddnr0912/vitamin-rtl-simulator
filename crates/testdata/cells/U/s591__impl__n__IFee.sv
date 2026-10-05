package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
interface ifc;
  import pa::P;
  import pb::P;
  int k;
  initial begin k = P; #1 $display("ifee P=%0d b=%0d", P, $bits(P)); end
endinterface
module top;
  ifc i();
  initial #100 $finish;
endmodule
