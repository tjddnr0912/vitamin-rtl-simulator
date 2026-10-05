module t;
  localparam [69:0] W70 = 70'h3 << 68;
  localparam [69:0] W70B = 70'h3 << 68;
`ifdef L1
  case (1) {64'd0, (4'b1100 ==? 4'b1?00)} : begin initial $display("L1 item"); end
    default : begin initial $display("L1 dflt"); end
  endcase
`endif
`ifdef L1P
  case (1) {64'd0, (4'b1100 == 4'b1100)} : begin initial $display("L1P item"); end
    default : begin initial $display("L1P dflt"); end
  endcase
`endif
`ifdef L2
  case (1) (W70 ==? 70'b11????????????????????????????????????????????????????????????????????) : begin initial $display("L2 item"); end
    default : begin initial $display("L2 dflt"); end
  endcase
`endif
`ifdef L2P
  case (1) (W70 == W70B) : begin initial $display("L2P item"); end
    default : begin initial $display("L2P dflt"); end
  endcase
`endif
`ifdef L3
`ifdef IV
  case (1) ((W70 ==? 70'b11????????????????????????????????????????????????????????????????????) || (W70 ==? 70'd5)) : begin initial $display("L3 item"); end
    default : begin initial $display("L3 dflt"); end
  endcase
`else
  case (1) (W70 inside {70'b11????????????????????????????????????????????????????????????????????, 70'd5}) : begin initial $display("L3 item"); end
    default : begin initial $display("L3 dflt"); end
  endcase
`endif
`endif
`ifdef L4
  case (1) ((W70 ==? 70'b11????????????????????????????????????????????????????????????????????) + 0) : begin initial $display("L4 item"); end
    default : begin initial $display("L4 dflt"); end
  endcase
`endif
`ifdef L4P
  case (1) ((W70 == W70B) + 0) : begin initial $display("L4P item"); end
    default : begin initial $display("L4P dflt"); end
  endcase
`endif
`ifdef L5
  case (0) (4'bx101 ==? 4'b1?00) : begin initial $display("L5 item"); end
    default : begin initial $display("L5 dflt"); end
  endcase
`endif
`ifdef L5N
  case (1) (4'bx101 !=? 4'b1?00) : begin initial $display("L5N item"); end
    default : begin initial $display("L5N dflt"); end
  endcase
`endif
`ifdef L6
`ifdef IV
  case (1) ((4'bx100 ==? 4'b1?00) || (4'bx100 ==? 4'b??00)) : begin initial $display("L6 item"); end
    default : begin initial $display("L6 dflt"); end
  endcase
`else
  case (1) (4'bx100 inside {4'b1?00, 4'b??00}) : begin initial $display("L6 item"); end
    default : begin initial $display("L6 dflt"); end
  endcase
`endif
`endif
`ifdef L7
  case (0) {1'b0, (4'bx100 ==? 4'b1?00)} : begin initial $display("L7 item"); end
    default : begin initial $display("L7 dflt"); end
  endcase
`endif
`ifdef L8
  case (1) ((4'b1100 ==? 4'b1?00) * 65'd1) : begin initial $display("L8 item"); end
    default : begin initial $display("L8 dflt"); end
  endcase
`endif
`ifdef L8P
  case (1) ((4'b1100 == 4'b1100) * 65'd1) : begin initial $display("L8P item"); end
    default : begin initial $display("L8P dflt"); end
  endcase
`endif
`ifdef L9
  case (1) ("ab" ==? 16'b0110_0001_0110_00??) : begin initial $display("L9 item"); end
    default : begin initial $display("L9 dflt"); end
  endcase
`endif
`ifdef L10
  case (1) ((4'b1100 ==? 4'b1?00) ? 65'd1 : 65'd0) : begin initial $display("L10 item"); end
    default : begin initial $display("L10 dflt"); end
  endcase
`endif
`ifdef L10P
  case (1) ((4'b1100 == 4'b1100) ? 65'd1 : 65'd0) : begin initial $display("L10P item"); end
    default : begin initial $display("L10P dflt"); end
  endcase
`endif
`ifdef L11
  case (1) 0, (4'bx100 ==? 4'b1?00) : begin initial $display("L11 item"); end
    default : begin initial $display("L11 dflt"); end
  endcase
`endif
`ifdef L12
  case (1) (4'bx100 ==? 4'b1?00), 1 : begin initial $display("L12 item"); end
    default : begin initial $display("L12 dflt"); end
  endcase
`endif
  initial #5 $finish;
endmodule
