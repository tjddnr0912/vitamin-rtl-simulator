package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
interface ifc;
  import pa::*;
  import pb::P;
  if (P > 65'd100) begin : t initial #1 $display("gif %m big"); end
  else begin : e initial #1 $display("gif %m small"); end
endinterface
module top;
  ifc x ();
  initial #100 $finish;
endmodule
