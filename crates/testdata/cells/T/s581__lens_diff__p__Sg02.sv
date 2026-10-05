localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
module top;
  case (1) (33'h1_0000_0000 ==? 33'h1_????_???0): begin : a initial $display("@g1 hit"); end default: begin : ad initial $display("@g1 def"); end endcase
  case (0) (33'h1_0000_0000 ==? 33'h1_????_???0): begin : b initial $display("@g0 hit"); end default: begin : bd initial $display("@g0 def"); end endcase
  case ((33'h1_0000_0000 ==? 33'h1_????_???0)) 1'b1: begin : e initial $display("@gs 1"); end 1'b0: begin : e2 initial $display("@gs 0"); end default: begin : ed initial $display("@gs def"); end endcase
endmodule
