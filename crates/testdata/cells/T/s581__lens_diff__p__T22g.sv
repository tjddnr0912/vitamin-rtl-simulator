module top;
  parameter Q1 = -4'sd1;
  parameter Q2 = -1;
  parameter Q3 = "ab";
  parameter signed Q4 = 4'hF;
  parameter logic signed Q5 = 1'b1;
  parameter [3:0] Q6 = -1;
  parameter Q7 = 4'hF;
  case (-1) Q1: begin : a initial $display("@Q1 hit"); end default: begin : ad initial $display("@Q1 def"); end endcase
  case (-64'sd1) Q2: begin : b initial $display("@Q2 hit"); end default: begin : bd initial $display("@Q2 def"); end endcase
  case (16'h6162) Q3: begin : c initial $display("@Q3 hit"); end default: begin : cd initial $display("@Q3 def"); end endcase
  case (-1) Q4: begin : e initial $display("@Q4 hit"); end default: begin : ed initial $display("@Q4 def"); end endcase
  case (-1) Q5: begin : f initial $display("@Q5 hit"); end default: begin : fd initial $display("@Q5 def"); end endcase
  case (-1) Q6: begin : g initial $display("@Q6 hit"); end default: begin : gd initial $display("@Q6 def"); end endcase
  case (-1) Q7: begin : h initial $display("@Q7 hit"); end 255: begin : h2 initial $display("@Q7 255"); end default: begin : hd initial $display("@Q7 def"); end endcase
  case (Q1) 8'hFF: begin : k initial $display("@sQ1 ff"); end -1: begin : k2 initial $display("@sQ1 m1"); end default: begin : kd initial $display("@sQ1 def"); end endcase
  case (Q2) 64'hFFFF_FFFF_FFFF_FFFF: begin : l initial $display("@sQ2 ff"); end -1: begin : l2 initial $display("@sQ2 m1"); end default: begin : ld initial $display("@sQ2 def"); end endcase
  case (Q5) 2'b11: begin : m initial $display("@sQ5 11"); end -1: begin : m2 initial $display("@sQ5 m1"); end default: begin : md initial $display("@sQ5 def"); end endcase
endmodule
