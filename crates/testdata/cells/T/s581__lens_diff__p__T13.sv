module top;
  localparam logic [1:0] P2 = 2'b10;
  localparam logic signed [1:0] S2 = -2'sd1;
  case (4'b1001) {2'b10, 2'b01}: begin : a initial $display("@A hit"); end default: begin : ad initial $display("@A def"); end endcase
  case (8'hAA) {4{P2}}: begin : b initial $display("@B hit"); end default: begin : bd initial $display("@B def"); end endcase
  case (-1) {P2, P2}: begin : c initial $display("@C hit"); end default: begin : cd initial $display("@C def"); end endcase
  case (-1) {S2}: begin : e initial $display("@E hit"); end default: begin : ed initial $display("@E def"); end endcase
  case (-1) S2: begin : f initial $display("@F hit"); end default: begin : fd initial $display("@F def"); end endcase
  case (6'b100000) {P2, 4'h0}: begin : g initial $display("@G hit"); end default: begin : gd initial $display("@G def"); end endcase
  case (16'h0F0F) {2{{4'h0}, {4'hF}}}: begin : h initial $display("@H hit"); end default: begin : hd initial $display("@H def"); end endcase
endmodule
