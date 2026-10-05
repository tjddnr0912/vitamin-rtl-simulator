module m #(parameter logic [7:0] P = 8'd1, parameter [64:0] W = 65'h0) ();
  case (P) -1: begin : a initial $display("@%m P m1"); end 255: begin : b initial $display("@%m P 255"); end default: begin : d initial $display("@%m P def"); end endcase
  case (1) W: begin : wa initial $display("@%m W hit"); end default: begin : wd initial $display("@%m W def"); end endcase
endmodule
module top; m u(); defparam u.P = 8'hFF; defparam u.W = 65'h1; endmodule
