package q; typedef enum logic [1:0] {A=1,B=2} qe; localparam qe P = B; endpackage
package r; typedef enum logic [2:0] {X=5,Y=6} re; localparam re P = Y; endpackage
module tb; import q::*; import r::P;
  initial begin #1 $display("DIGEST=%0d %s", P, P.name()); #1 $finish; end
endmodule