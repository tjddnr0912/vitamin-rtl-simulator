package q; localparam int P = 5; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display("DIGEST=%0d", P); #1 $finish; end
endmodule