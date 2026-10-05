module top;
  case (4'b10x0) 4'b1000: begin : a initial $display("@X 1000"); end 4'b10x0: begin : b initial $display("@X 10x0"); end default: begin : d initial $display("@X def"); end endcase
  case (4'b1z00) 4'b1000: begin : a2 initial $display("@Z 1000"); end 4'b1z00: begin : b2 initial $display("@Z 1z00"); end default: begin : d2 initial $display("@Z def"); end endcase
  case (8) 4'b1x00: begin : a3 initial $display("@L 1x00"); end 4'b1?00: begin : b3 initial $display("@L 1?00"); end 8: begin : c3 initial $display("@L 8"); end default: begin : d3 initial $display("@L def"); end endcase
endmodule
