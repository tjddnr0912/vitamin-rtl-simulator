package p; parameter logic [Nope1-1:0] P = 4'h9; endpackage
module m (); generate if (1) begin : g localparam logic [Nope2-1:0] L = 4'h9; initial $display("DIGEST=%0d", $bits(L)); end endgenerate initial $display("DIGEST=%0d", $bits(p::P)); endmodule
module tb; m u(); initial begin #1 $finish; end endmodule
