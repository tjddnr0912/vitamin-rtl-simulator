module top;
  case (64'h6162636465666768) "xabcdefgh": begin : a initial $display("@S72 wrong"); end "abcdefgh": begin : b initial $display("@S72 right"); end default: begin : d initial $display("@S72 def"); end endcase
  case (8'h61) "ba": begin : a2 initial $display("@S16 wrong"); end "a": begin : b2 initial $display("@S16 a"); end default: begin : d2 initial $display("@S16 def"); end endcase
  case (16'h6162) 1, "ab", 3: begin : a3 initial $display("@SM hit"); end default: begin : d3 initial $display("@SM def"); end endcase
  case (-1) "\377\377\377\377": begin : a4 initial $display("@SN hit"); end default: begin : d4 initial $display("@SN def"); end endcase
  case (-64'sd1) "\377\377\377\377": begin : a5 initial $display("@SN64 hit"); end default: begin : d5 initial $display("@SN64 def"); end endcase
endmodule
