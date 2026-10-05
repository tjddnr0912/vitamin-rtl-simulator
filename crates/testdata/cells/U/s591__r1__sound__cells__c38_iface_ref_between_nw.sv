package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
interface ifc;
  import pa::*;
  localparam int A = P;
  import pb::*;
  initial #1 $display("inw %m A=%0d P=%0d", A, P);
endinterface
module top; ifc x (); initial #100 $finish; endmodule
