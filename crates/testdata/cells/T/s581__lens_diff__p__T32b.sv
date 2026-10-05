module m #(parameter type T = logic [3:0], parameter int W = 4) ();
  localparam T TP = '1;
  localparam logic [W-1:0] WP = '1;
  case (-1) TP: begin : a initial $display("@%m TP-1 hit"); end default: begin : ad initial $display("@%m TP-1 def"); end endcase
  case (8'hFF) TP: begin : b initial $display("@%m TPff hit"); end default: begin : bd initial $display("@%m TPff def"); end endcase
  case (TP) 16'hFFFF: begin : c initial $display("@%m sTP ffff"); end -1: begin : c2 initial $display("@%m sTP m1"); end default: begin : cd initial $display("@%m sTP def"); end endcase
  case (-64'sd1) WP: begin : e initial $display("@%m WP hit"); end default: begin : ed initial $display("@%m WP def"); end endcase
endmodule
module top;
  m u_def ();
  m #(.T(logic signed [3:0])) u_s4 ();
  m #(.T(logic [7:0])) u_u8 ();
  m #(.T(byte)) u_byte ();
  m #(.T(int unsigned)) u_iu ();
endmodule
