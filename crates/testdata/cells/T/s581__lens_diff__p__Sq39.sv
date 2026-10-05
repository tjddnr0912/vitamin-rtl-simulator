localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam L = ((S4 >>> 1) ==? 8'sb1111_1?10);
  localparam N = ((S4 >>> 1) !=? 8'sb1111_1?10);
  logic [((S4 >>> 1) ==? 8'sb1111_1?10) : 0] v;
  initial $display("@L %b %b bits=%0d", L, N, $bits(v));
  initial $display("@R %b %b", ((S4 >>> 1) ==? 8'sb1111_1?10), ((S4 >>> 1) !=? 8'sb1111_1?10));
  if ((S4 >>> 1) ==? 8'sb1111_1?10) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// (S4 >>> 1) ==? 8'sb1111_1?10
