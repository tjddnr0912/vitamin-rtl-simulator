module leaf #(parameter [64:0] W = 0, parameter logic signed [7:0] S = 0) ();
  case (1) W: begin : wa initial $display("@%m W hit"); end default: begin : wd initial $display("@%m W def"); end endcase
  case (-1) S: begin : sa initial $display("@%m S hit"); end default: begin : sd initial $display("@%m S def"); end endcase
endmodule
module mid #(parameter [64:0] W = 0) (); leaf #(.W(W >> 64), .S(-8'sd1)) l (); endmodule
module top; mid #(.W(65'h1)) m1 (); mid #(.W(65'h1_0000_0000_0000_0000)) m2 (); endmodule
