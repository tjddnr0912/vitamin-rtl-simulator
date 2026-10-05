package pa; localparam P = 1; endpackage
package pb; localparam P = 2; endpackage
package pn; localparam P = 3; endpackage
package pw; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
import pa::*;
import pb::*;
import pn::P;
module top;
  import pw::P;
  if (P > 65'd100) begin : big initial #1 $display("m13 big"); end
  else begin : sm initial #1 $display("m13 small"); end
  logic [P[3:0]:0] v;
  initial #1 $display("m13 P=%0d b=%0d", P, $bits(v));
  initial #100 $finish;
endmodule
