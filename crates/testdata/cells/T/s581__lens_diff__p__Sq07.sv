localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = (33'h1_0000_0000 ==? 33'h1_????_???0);
  localparam N = (33'h1_0000_0000 !=? 33'h1_????_???0);
  logic [(33'h1_0000_0000 ==? 33'h1_????_???0) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", (33'h1_0000_0000 ==? 33'h1_????_???0), (33'h1_0000_0000 !=? 33'h1_????_???0));
  if (33'h1_0000_0000 ==? 33'h1_????_???0) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// 33'h1_0000_0000 ==? 33'h1_????_???0
