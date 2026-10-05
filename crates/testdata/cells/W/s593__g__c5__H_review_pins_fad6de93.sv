package q; parameter int QW = 6; endpackage
package p; import q::*; typedef logic [QW-1:0] t; endpackage
module tb; localparam int QW = 12; p::t v;
  initial begin v='1; $display("DIGEST=%0d", $bits(v)); #1 $finish; end
endmodule
