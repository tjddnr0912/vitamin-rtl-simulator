package p; typedef logic [5:0] u; endpackage
module m import p::*; #(parameter u A = 6'd1, B = 9'h1FF) (output logic [63:0] o);
  assign o = A + B;
endmodule
module tb; logic [63:0] o; m u(.o(o));
  initial begin #1 $display("DIGEST=%0d", o); $finish; end
endmodule