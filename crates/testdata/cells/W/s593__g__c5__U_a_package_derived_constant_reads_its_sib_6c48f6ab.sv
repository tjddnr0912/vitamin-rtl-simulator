package p; localparam logic [3:0] P = 5'h1F; localparam W = P + 1;
  localparam int I = 40'h1_0000_0001; localparam WI = I + 1; endpackage
module tb; import p::*; initial begin $display("DIGEST=%0d %0d %0d %0d", P, W, I, WI); #1 $finish; end endmodule