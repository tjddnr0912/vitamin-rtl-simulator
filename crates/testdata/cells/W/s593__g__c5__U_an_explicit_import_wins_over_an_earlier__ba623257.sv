package q; typedef struct packed { logic [7:0] hi; logic [3:0] lo; } qt; localparam qt P = '{hi:8'hAA, lo:4'h5}; endpackage
package r; localparam logic [11:0] P = 12'h122; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display("DIGEST=%h %h", P, P[3:0]); #1 $finish; end
endmodule