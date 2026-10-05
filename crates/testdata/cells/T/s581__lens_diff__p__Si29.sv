localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam I = ((64'hFFFF_FFFF_FFFF_FFFF + 64'd1) inside {65'h1_????_????_????_???0});
  logic [((64'hFFFF_FFFF_FFFF_FFFF + 64'd1) inside {65'h1_????_????_????_???0}) : 0] v;
  initial $display("@I %b bits=%0d", I, $bits(v));
  initial $display("@R %b", ((64'hFFFF_FFFF_FFFF_FFFF + 64'd1) inside {65'h1_????_????_????_???0}));
  if ((64'hFFFF_FFFF_FFFF_FFFF + 64'd1) inside {65'h1_????_????_????_???0}) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// (64'hFFFF_FFFF_FFFF_FFFF + 64'd1) inside 65'h1_????_????_????_???0
