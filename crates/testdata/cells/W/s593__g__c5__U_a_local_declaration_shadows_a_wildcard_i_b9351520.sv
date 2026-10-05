package p; localparam logic [5:0] P = 6'd35; logic [5:0] V = 6'd9; localparam logic [127:0] WD = 128'h1; endpackage
module m #(parameter P = 3) (output logic [7:0] o); import p::*; assign o = P; endmodule
module tb; import p::*; logic [5:0] V = 6'd1; logic [7:0] o; m u(.o(o)); logic [7:0] WD = 8'd5;
  initial begin #1 $display("DIGEST=%0d %0d %0d %0d", o, V, p::V, WD); $finish; end endmodule