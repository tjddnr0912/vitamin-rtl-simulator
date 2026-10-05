module top;
  localparam int W6 = 6;
  localparam logic [7:0] P8 = 8'hF0;
  case (6'sh30) W6'(P8): begin : a initial $display("@A paramcast-signedscrut hit"); end default: begin : ad initial $display("@A paramcast-signedscrut def"); end endcase
  case (6'sh30) 6'(P8): begin : b initial $display("@B litcast-signedscrut hit"); end default: begin : bd initial $display("@B litcast-signedscrut def"); end endcase
  case (11'h783) {P8, 3'(6'(3'h3))}: begin : e initial $display("@E concat-litcast hit"); end default: begin : ed initial $display("@E concat-litcast def"); end endcase
  case (11'h783) {P8, 3'(W6'(3'h3))}: begin : f initial $display("@F concat-paramcast hit"); end default: begin : fd initial $display("@F concat-paramcast def"); end endcase
  case (8'hFF) W6'(-1): begin : g initial $display("@G paramcast-unsignedscrut hit"); end default: begin : gd initial $display("@G paramcast-unsignedscrut def"); end endcase
  case (8'hFF) 6'(-1): begin : h initial $display("@H litcast-unsignedscrut hit"); end default: begin : hd initial $display("@H litcast-unsignedscrut def"); end endcase
endmodule
