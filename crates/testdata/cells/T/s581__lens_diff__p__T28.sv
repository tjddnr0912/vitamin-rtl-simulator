localparam int U = -1;
package pk; localparam int K = -1; localparam int K2 = -1; endpackage
module top;
  import pk::*;
  localparam logic [3:0] U = 4'hF;
  localparam logic [3:0] K = 4'hF;
  case (-1) U: begin : a initial $display("@U unit-misread"); end default: begin : ad initial $display("@U def"); end endcase
  case (-1) K: begin : b initial $display("@K pkg-misread"); end default: begin : bd initial $display("@K def"); end endcase
  case (-1) K2: begin : c initial $display("@K2 hit"); end default: begin : cd initial $display("@K2 def"); end endcase
  case (64'hFFFF_FFFF_FFFF_FFFF) K2: begin : e initial $display("@K2u hit"); end default: begin : ed initial $display("@K2u def"); end endcase
endmodule
