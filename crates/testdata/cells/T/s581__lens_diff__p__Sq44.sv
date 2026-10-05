localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = (16'hFFFF ==? 8'sb1???_????);
  localparam N = (16'hFFFF !=? 8'sb1???_????);
  logic [(16'hFFFF ==? 8'sb1???_????) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", (16'hFFFF ==? 8'sb1???_????), (16'hFFFF !=? 8'sb1???_????));
  if (16'hFFFF ==? 8'sb1???_????) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// 16'hFFFF ==? 8'sb1???_????
