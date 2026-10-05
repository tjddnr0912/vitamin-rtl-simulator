package p; parameter logic [79:0] Wd = 80'hABCD_0000_0000_0000_1234; endpackage
module m import p::*; #(parameter logic [79:0] X = Wd) (); initial $display("DIGEST=%h", X); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
