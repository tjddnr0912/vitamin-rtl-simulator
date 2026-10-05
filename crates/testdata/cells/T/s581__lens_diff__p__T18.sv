module top;
  localparam [3:0] A = 4'b1100;
  case (1) (A ==? 4'b1?00): begin : a initial $display("@A hit"); end default: begin : ad initial $display("@A def"); end endcase
  case (1) (A inside {[4'd10:4'd13]}): begin : c initial $display("@C hit"); end default: begin : cd initial $display("@C def"); end endcase
  case (1'b1) (A !=? 4'b0?00): begin : e initial $display("@E hit"); end default: begin : ed initial $display("@E def"); end endcase
  case (1) (A inside {4'b11?1, 4'b1100}): begin : f initial $display("@F hit"); end default: begin : fd initial $display("@F def"); end endcase
  case (0) (A ==? 4'b0?00): begin : g initial $display("@G hit"); end default: begin : gd initial $display("@G def"); end endcase
  case (1) (A inside {[4'd13:$]}): begin : h initial $display("@H hit"); end default: begin : hd initial $display("@H def"); end endcase
endmodule
