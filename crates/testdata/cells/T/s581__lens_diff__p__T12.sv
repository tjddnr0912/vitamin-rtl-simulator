module top;
  localparam logic [7:0] P8 = 8'h5;
  case (8'shFF) $signed(4'hF): begin : a initial $display("@A hit"); end default: begin : ad initial $display("@A def"); end endcase
  case (-1) $unsigned(-1): begin : b initial $display("@B hit"); end default: begin : bd initial $display("@B def"); end endcase
  case (1'sb1) -1: begin : c initial $display("@C hit"); end default: begin : cd initial $display("@C def"); end endcase
  case (5) $clog2(17): begin : e initial $display("@E hit"); end default: begin : ed initial $display("@E def"); end endcase
  case (8) $bits(P8): begin : f initial $display("@F hit"); end default: begin : fd initial $display("@F def"); end endcase
  case (4'sb1100) $signed(4'b1000) >>> 1: begin : g initial $display("@G hit"); end default: begin : gd initial $display("@G def"); end endcase
  case (-4) $signed(4'b1000) >>> 1: begin : h initial $display("@H hit"); end default: begin : hd initial $display("@H def"); end endcase
  case (8'hFF) $signed(4'hF): begin : k initial $display("@K hit"); end default: begin : kd initial $display("@K def"); end endcase
  case (-1) $signed(P8 - 8'd6): begin : l initial $display("@L hit"); end default: begin : ld initial $display("@L def"); end endcase
  case (3) $clog2(P8): begin : m initial $display("@M hit"); end default: begin : md initial $display("@M def"); end endcase
endmodule
