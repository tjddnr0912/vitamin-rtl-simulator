package p; typedef struct packed { logic [3:0] a; logic [3:0] b; } s_t; parameter s_t Dflt = '{a:4'h1, b:4'h2}; endpackage
module m import p::*; #(parameter s_t X = Dflt) (); initial $display("DIGEST=%h %h", X, X.b); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
