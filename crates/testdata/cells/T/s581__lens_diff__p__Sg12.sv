localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
module top;
  case (1) (8'bxxxx_0101 ==? 8'b????_0101): begin : a initial $display("@g1 hit"); end default: begin : ad initial $display("@g1 def"); end endcase
  case (0) (8'bxxxx_0101 ==? 8'b????_0101): begin : b initial $display("@g0 hit"); end default: begin : bd initial $display("@g0 def"); end endcase
  case ((8'bxxxx_0101 ==? 8'b????_0101)) 1'b1: begin : e initial $display("@gs 1"); end 1'b0: begin : e2 initial $display("@gs 0"); end default: begin : ed initial $display("@gs def"); end endcase
endmodule
