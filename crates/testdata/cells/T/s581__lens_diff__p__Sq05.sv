localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = (8'sb1010_1100 ==? 4'sb?100);
  localparam N = (8'sb1010_1100 !=? 4'sb?100);
  logic [(8'sb1010_1100 ==? 4'sb?100) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", (8'sb1010_1100 ==? 4'sb?100), (8'sb1010_1100 !=? 4'sb?100));
  if (8'sb1010_1100 ==? 4'sb?100) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// 8'sb1010_1100 ==? 4'sb?100
