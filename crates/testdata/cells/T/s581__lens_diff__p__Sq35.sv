localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = (UP ==? 4'b1?00);
  localparam N = (UP !=? 4'b1?00);
  logic [(UP ==? 4'b1?00) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", (UP ==? 4'b1?00), (UP !=? 4'b1?00));
  if (UP ==? 4'b1?00) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// UP ==? 4'b1?00
