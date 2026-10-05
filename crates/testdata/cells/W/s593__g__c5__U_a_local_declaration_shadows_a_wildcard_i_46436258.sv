package p; localparam logic [5:0] P = 6'd35; localparam logic [5:0] Q = 6'd9; endpackage
module tb; import p::*; logic [5:0] P = 6'd42; localparam logic [5:0] Q = 6'd7;
  initial begin $display("DIGEST=%0d %0d %0d", P, Q, p::P); #1 $finish; end endmodule