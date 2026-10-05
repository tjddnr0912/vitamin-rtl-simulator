package pk; typedef enum logic [64:0] {PE1 = 65'h1_0000_0000_0000_0000, PE0 = 65'h0} pe_t; endpackage
module top;
  typedef enum logic [64:0] {E1 = 65'h1_0000_0000_0000_0000, E0 = 65'h0} e_t;
  typedef enum logic signed [3:0] {SN = -4'sd1, SZ = 4'sd0} s_t;
  case (0) E1: begin : a initial $display("@E E1"); end E0: begin : b initial $display("@E E0"); end default: begin : d initial $display("@E def"); end endcase
  case (0) pk::PE1: begin : a2 initial $display("@PE PE1"); end pk::PE0: begin : b2 initial $display("@PE PE0"); end default: begin : d2 initial $display("@PE def"); end endcase
  case (-1) SN: begin : a3 initial $display("@SN hit"); end default: begin : d3 initial $display("@SN def"); end endcase
  case (8'hFF) SN: begin : a4 initial $display("@SNu hit"); end default: begin : d4 initial $display("@SNu def"); end endcase
  case (SN) 8'hFF: begin : a5 initial $display("@SNs hit"); end -1: begin : b5 initial $display("@SNs m1"); end default: begin : d5 initial $display("@SNs def"); end endcase
endmodule
