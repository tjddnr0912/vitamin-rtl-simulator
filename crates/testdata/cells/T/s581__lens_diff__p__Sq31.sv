localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = (~4'b0000 ==? 8'b1111_????);
  localparam N = (~4'b0000 !=? 8'b1111_????);
  logic [(~4'b0000 ==? 8'b1111_????) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", (~4'b0000 ==? 8'b1111_????), (~4'b0000 !=? 8'b1111_????));
  if (~4'b0000 ==? 8'b1111_????) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// ~4'b0000 ==? 8'b1111_????
