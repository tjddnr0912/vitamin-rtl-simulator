module top;
  case (64'h1_0000_0000) 4294967296: begin : a initial $display("@A hit"); end default: begin : ad initial $display("@A def"); end endcase
  case (64'h1_0000_0000) 'h1_0000_0000: begin : b initial $display("@B hit"); end default: begin : bd initial $display("@B def"); end endcase
  case (0) 1 << 32: begin : c initial $display("@C hit"); end default: begin : cd initial $display("@C def"); end endcase
  case (1) 1 << 64: begin : e initial $display("@E hit"); end default: begin : ed initial $display("@E def"); end endcase
  case (16) 2 ** 4: begin : f initial $display("@F hit"); end default: begin : fd initial $display("@F def"); end endcase
  case (0) 2 ** -1: begin : g initial $display("@G hit"); end default: begin : gd initial $display("@G def"); end endcase
  case (-1) (-1) ** -1: begin : h initial $display("@H hit"); end default: begin : hd initial $display("@H def"); end endcase
  case (0) 0 ** -1: begin : k initial $display("@K hit"); end default: begin : kd initial $display("@K def"); end endcase
  case (1) (4'b1x00 === 4'b1x00): begin : l initial $display("@L hit"); end default: begin : ld initial $display("@L def"); end endcase
  case (0) (4'b1x00 == 4'b1x00): begin : m initial $display("@M hit"); end default: begin : md initial $display("@M def"); end endcase
  case (1) (1'bx || 1'b1): begin : n initial $display("@N hit"); end default: begin : nd initial $display("@N def"); end endcase
  case (-1) -8 >>> 4: begin : o initial $display("@O hit"); end default: begin : od initial $display("@O def"); end endcase
  case (0) 1 / 0: begin : q initial $display("@Q hit"); end default: begin : qd initial $display("@Q def"); end endcase
  case (2) -7 % 3 + 3: begin : r initial $display("@R hit"); end default: begin : rd initial $display("@R def"); end endcase
  case (1) 1 << -1: begin : s initial $display("@S hit"); end default: begin : sd initial $display("@S def"); end endcase
endmodule
