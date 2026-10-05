package q; localparam int P = 7; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*;
  generate import r::P; endgenerate
  initial begin $display("DIGEST=%0d", P); #1 $finish; end
endmodule