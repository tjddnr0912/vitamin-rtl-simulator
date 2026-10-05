module t;
  localparam logic [67:0] P68 = 68'hC;
  localparam logic [63:0] PF = 64'hFFFF_FFFF_FFFF_FFFF;
  localparam SP = "ab";
  initial #100 $finish;
`ifdef G1
  case (1)
    (68'h8_0000_0000_0000_0001 ==? 68'h8_0000_0000_0000_000?): begin : ga initial $display("G1 item"); end
    default: begin : gb initial $display("G1 default"); end
  endcase
`endif
`ifdef G2
  case (1)
    (P68 ==? 4'b1?00): begin : ga initial $display("G2 item"); end
    default: begin : gb initial $display("G2 default"); end
  endcase
`endif
`ifdef G3
  case (1)
    (P68 inside {4'b1?00}): begin : ga initial $display("G3 item"); end
    default: begin : gb initial $display("G3 default"); end
  endcase
`endif
`ifdef G4
  case (1)
    ("ab" ==? 16'h616?): begin : ga initial $display("G4 item"); end
    default: begin : gb initial $display("G4 default"); end
  endcase
`endif
`ifdef G5
  case (1)
    ((4'b1100 ==? 4'b1?00) && (P68 == 68'hC)): begin : ga initial $display("G5 item"); end
    default: begin : gb initial $display("G5 default"); end
  endcase
`endif
`ifdef G6
  case (1)
    ((PF + 64'h1) ==? 65'h1_0000_0000_0000_000?): begin : ga initial $display("G6 item"); end
    default: begin : gb initial $display("G6 default"); end
  endcase
`endif
`ifdef G7
  case (0)
    (P68 !=? 4'b1?00): begin : ga initial $display("G7 item"); end
    default: begin : gb initial $display("G7 default"); end
  endcase
`endif
`ifdef G8
  case (1)
    (4'b1100 ==? 4'b1?00): begin : ga initial $display("G8 item"); end
    default: begin : gb initial $display("G8 default"); end
  endcase
`endif
`ifdef G9
  case (1)
    (SP ==? 16'h616?): begin : ga initial $display("G9 item"); end
    default: begin : gb initial $display("G9 default"); end
  endcase
`endif
`ifdef LV
  localparam logic L1 = (68'h8_0000_0000_0000_0001 ==? 68'h8_0000_0000_0000_000?);
  localparam logic L2 = (P68 ==? 4'b1?00);
  localparam logic L6 = ((PF + 64'h1) ==? 65'h1_0000_0000_0000_000?);
  logic [63:0] pf;
  initial begin
    pf = PF; #2;
    $display("L1 %b", L1);
    $display("L2 %b", L2);
    $display("L6 %b", L6);
    $display("R6 %b", ((pf + 64'h1) ==? 65'h1_0000_0000_0000_000?));
  end
`endif
endmodule
