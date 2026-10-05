module t;
  initial #100 $finish;
`ifdef J1
  case (1)
    $countbits(4'bx100 ==? 4'b1?00, 1'bx): begin : ga initial $display("J1 item"); end
    default: begin : gb initial $display("J1 default"); end
  endcase
`endif
`ifdef J2
  case (1)
    (4'bx100 ==? 4'b1?00) | 1'b1: begin : ga initial $display("J2 item"); end
    default: begin : gb initial $display("J2 default"); end
  endcase
`endif
`ifdef J3
  case (0)
    (4'bx100 ==? 4'b1?00) & 1'b0: begin : ga initial $display("J3 item"); end
    default: begin : gb initial $display("J3 default"); end
  endcase
`endif
`ifdef J4
  case (0)
    (4'bx100 ==? 4'b1?00) >> 1: begin : ga initial $display("J4 item"); end
    default: begin : gb initial $display("J4 default"); end
  endcase
`endif
`ifdef J5
  case (1)
    |{(4'bx100 ==? 4'b1?00), 1'b1}: begin : ga initial $display("J5 item"); end
    default: begin : gb initial $display("J5 default"); end
  endcase
`endif
`ifdef J6
  case (0)
    !$isunknown(4'bx100 ==? 4'b1?00): begin : ga initial $display("J6 item"); end
    default: begin : gb initial $display("J6 default"); end
  endcase
`endif
`ifdef J7
  case (1)
    $isunknown(4'bx100 inside {4'b1?00}): begin : ga initial $display("J7 item"); end
    default: begin : gb initial $display("J7 default"); end
  endcase
`endif
`ifdef J8
  case (1)
    2'd2, $isunknown(4'bx100 ==? 4'b1?00): begin : ga initial $display("J8 item"); end
    default: begin : gb initial $display("J8 default"); end
  endcase
`endif
`ifdef J9
  case (1)
    $isunknown(4'b1100 ==? 4'b1?00): begin : ga initial $display("J9 item"); end
    default: begin : gb initial $display("J9 default"); end
  endcase
`endif
`ifdef JV
  localparam logic PV = $isunknown(4'bx100 ==? 4'b1?00);
  logic [3:0] a;
  initial begin a = 4'bx100; #2; $display("V1 %b", PV); $display("V2 %b", $isunknown(a ==? 4'b1?00)); end
`endif
endmodule
