package q; localparam int P = 5; endpackage
package r; localparam int P = 9; endpackage
module tb; import q::*;
  if (1) begin : g
    import r::P;
    initial $display("DIGEST=%0d", P);
  end
  initial begin #1 $finish; end
endmodule