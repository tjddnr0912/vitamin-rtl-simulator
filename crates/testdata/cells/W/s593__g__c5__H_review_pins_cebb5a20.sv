package p; parameter int Dflt = 37; endpackage
import p::Dflt;
module tb; localparam int Dflt = 5; initial begin $display("DIGEST=%0d", Dflt); #1 $finish; end endmodule
