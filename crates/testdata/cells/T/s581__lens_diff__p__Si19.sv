localparam logic [3:0] A4 = 4'b1100;
localparam logic signed [3:0] S4 = -4'sd4;
parameter UP = 4'b1100;
localparam logic [64:0] W65 = {1'b1, 64'd1};
package pk; localparam logic signed [7:0] K8 = -8'sd4; endpackage
module top;
  localparam I = (65'h1_0000_0000_0000_0001 inside {'b?1});
  logic [(65'h1_0000_0000_0000_0001 inside {'b?1}) : 0] v;
  initial $display("@I %b bits=%0d", I, $bits(v));
  initial $display("@R %b", (65'h1_0000_0000_0000_0001 inside {'b?1}));
  if (65'h1_0000_0000_0000_0001 inside {'b?1}) begin : gi initial $display("@G 1"); end else begin : ge initial $display("@G 0"); end
endmodule
// 65'h1_0000_0000_0000_0001 inside 'b?1
