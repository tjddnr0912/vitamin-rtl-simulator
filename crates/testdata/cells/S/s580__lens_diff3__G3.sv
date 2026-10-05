module t;
`ifdef M9
  localparam [64:0] LP = {64'd0, (4'b1100 ==? 4'b1?00)};
`endif
`ifdef M9P
  localparam [64:0] LPP = {64'd0, (4'b1100 == 4'b1100)};
  case (1) LPP : begin initial $display("M9P item"); end default : begin initial $display("M9P dflt"); end endcase
`endif
`ifdef M1
`ifdef IV
  case (1) {64'd0, (4'b1100 ==? 4'b1?00)} : begin initial $display("M1 item"); end default : begin initial $display("M1 dflt"); end endcase
`else
  case (1) {64'd0, (4'b1100 inside {4'b1?00})} : begin initial $display("M1 item"); end default : begin initial $display("M1 dflt"); end endcase
`endif
`endif
`ifdef M2
  case (1) {64'd0, (4'b1110 !=? 4'b1?00)} : begin initial $display("M2 item"); end default : begin initial $display("M2 dflt"); end endcase
`endif
`ifdef M3
  case (1) 5, {64'd0, (4'b1100 ==? 4'b1?00)} : begin initial $display("M3 item"); end default : begin initial $display("M3 dflt"); end endcase
`endif
`ifdef M4
  case (1) 65'(4'b1100 ==? 4'b1?00) : begin initial $display("M4 item"); end default : begin initial $display("M4 dflt"); end endcase
`endif
`ifdef M4P
  case (1) 65'(4'b1100 == 4'b1100) : begin initial $display("M4P item"); end default : begin initial $display("M4P dflt"); end endcase
`endif
`ifdef M6
  case (0) {64'd0, (4'b1100 ==? 4'b0?00)} : begin initial $display("M6 item"); end default : begin initial $display("M6 dflt"); end endcase
`endif
`ifdef M7
  case (1) {100'd0, (4'b1100 ==? 4'b1?00)} : begin initial $display("M7 item"); end default : begin initial $display("M7 dflt"); end endcase
`endif
`ifdef M8
  case (1) {64'd0, (4'bx100 ==? 4'b1?00)} : begin initial $display("M8 item"); end default : begin initial $display("M8 dflt"); end endcase
`endif
`ifdef M9
  case (1) LP : begin initial $display("M9 item"); end default : begin initial $display("M9 dflt"); end endcase
`endif
`ifdef M10
  case (1) {64'd0, (4'b1100 ==? 4'b1?00)} : begin initial $display("M10 item"); end 1 : begin initial $display("M10 one"); end default : begin initial $display("M10 dflt"); end endcase
`endif
  initial #5 $finish;
endmodule
