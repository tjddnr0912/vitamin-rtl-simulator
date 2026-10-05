package p; parameter int Dflt = 37; endpackage
module m import p::*; #(parameter X = 2, localparam Y = Dflt * X) (); initial $display("DIGEST=%0d", Y); endmodule
module tb; m u(); m #(.X(3)) v(); initial begin #1 $finish; end endmodule
