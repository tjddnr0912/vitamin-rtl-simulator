module top;
  typedef enum logic signed [3:0] {SN = -4'sd1, SZ = 4'sd0} s_t;
  case (-1) SN: begin : a3 initial $display("@SN hit"); end default: begin : d3 initial $display("@SN def"); end endcase
  case (8'hFF) SN: begin : a4 initial $display("@SNu hit"); end default: begin : d4 initial $display("@SNu def"); end endcase
  case (SN) 8'hFF: begin : a5 initial $display("@SNs hit"); end -1: begin : b5 initial $display("@SNs m1"); end default: begin : d5 initial $display("@SNs def"); end endcase
endmodule
