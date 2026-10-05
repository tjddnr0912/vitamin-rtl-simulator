localparam int U = -1;
package pk; localparam int K = -1; localparam logic signed [7:0] KS = -8'sd2; localparam logic [7:0] KU = 8'hFE; endpackage
module top;
  import pk::*;
  case (-64'sd1) U: begin : a initial $display("@U hit"); end default: begin : ad initial $display("@U def"); end endcase
  case (-64'sd1) pk::K: begin : b initial $display("@pkK hit"); end default: begin : bd initial $display("@pkK def"); end endcase
  case (-64'sd1) K: begin : c initial $display("@K hit"); end default: begin : cd initial $display("@K def"); end endcase
  case (-64'sd2) KS: begin : e initial $display("@KS hit"); end default: begin : ed initial $display("@KS def"); end endcase
  case (-64'sd2) pk::KU: begin : f initial $display("@KU hit"); end default: begin : fd initial $display("@KU def"); end endcase
  case (64'hFFFF_FFFF_FFFF_FFFF) U: begin : g initial $display("@Uu hit"); end default: begin : gd initial $display("@Uu def"); end endcase
  case (8'shFE) KS: begin : h initial $display("@KS8 hit"); end default: begin : hd initial $display("@KS8 def"); end endcase
endmodule
