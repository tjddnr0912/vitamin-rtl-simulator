module m (); parameter real R = 4.0; parameter logic [R-1:0] X = 4'h9; initial $display("DIGEST=%h %0d", X, $bits(X)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
