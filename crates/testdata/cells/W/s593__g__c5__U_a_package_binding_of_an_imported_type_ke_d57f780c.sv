package p1; typedef enum logic [1:0] {A1=1, B1=2} e_t; endpackage
package p2; import p1::*; e_t E = A1; endpackage
module tb; typedef enum logic [1:0] {A2=1, B2=2} e_t; import p2::*;
  initial begin $display("DIGEST=%s", E.name()); #1 $finish; end
endmodule