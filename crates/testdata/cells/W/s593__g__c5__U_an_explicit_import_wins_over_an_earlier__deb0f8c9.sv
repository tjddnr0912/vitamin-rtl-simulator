package q; typedef struct packed { logic [7:0] hi; logic [3:0] lo; } qt; localparam qt P = '{hi:8'hAA, lo:4'h5}; endpackage
package r; typedef struct packed { logic [3:0] lo; logic [7:0] hi; } rt; localparam rt P = '{lo:4'h1, hi:8'h22}; endpackage
module tb; import r::P; import q::*;
  initial begin #1 $display("DIGEST=%h %h %h", P, P.lo, P.hi); #1 $finish; end
endmodule