module top;
  localparam logic [3:0] P4 = 4'hC;
  localparam logic signed [3:0] S4 = -4'sd3;
  localparam logic [7:0] P8 = 8'hF0;
  localparam logic signed [7:0] S8 = -8'sd100;
  localparam int I = -5;
  localparam int unsigned U32 = 32'hFFFF_FFF0;
  localparam logic [64:0] P65 = {1'b1, 64'h5};
  localparam logic signed [64:0] S65 = -65'sd7;
  localparam longint L64 = -9;
  parameter UN = 4'hA;
  case (1'h0) (!{4'(35), 3'(P65)}): begin : h0 initial $display("@0_0 hit"); end default: begin : d0 initial $display("@0_0 def"); end endcase
  case (1'sh0) (!{4'(35), 3'(P65)}): begin : h1 initial $display("@0_1 hit"); end default: begin : d1 initial $display("@0_1 def"); end endcase
  case (4'h0) (!{4'(35), 3'(P65)}): begin : h2 initial $display("@0_2 hit"); end default: begin : d2 initial $display("@0_2 def"); end endcase
  case (0) (!{4'(35), 3'(P65)}): begin : h3 initial $display("@0_3 hit"); end default: begin : d3 initial $display("@0_3 def"); end endcase
  case (64'h0) (!{4'(35), 3'(P65)}): begin : h4 initial $display("@0_4 hit"); end default: begin : d4 initial $display("@0_4 def"); end endcase
  case (0) (!{4'(35), 3'(P65)}): begin : h5 initial $display("@0_5 hit"); end default: begin : d5 initial $display("@0_5 def"); end endcase
  case (1'h0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h6 initial $display("@0_6 hit"); end default: begin : d6 initial $display("@0_6 def"); end endcase
  case (1'sh0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h7 initial $display("@0_7 hit"); end default: begin : d7 initial $display("@0_7 def"); end endcase
  case (4'h0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h8 initial $display("@0_8 hit"); end default: begin : d8 initial $display("@0_8 def"); end endcase
  case (0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h9 initial $display("@0_9 hit"); end default: begin : d9 initial $display("@0_9 def"); end endcase
  case (64'h0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h10 initial $display("@0_10 hit"); end default: begin : d10 initial $display("@0_10 def"); end endcase
  case (0) (1'h0 >> (5'h1c / 63'sh53ba703424293209)): begin : h11 initial $display("@0_11 hit"); end default: begin : d11 initial $display("@0_11 def"); end endcase
  case (16'h5ecd) 16'sh5ecd: begin : h12 initial $display("@0_12 hit"); end default: begin : d12 initial $display("@0_12 def"); end endcase
  case (16'sh5ecd) 16'sh5ecd: begin : h13 initial $display("@0_13 hit"); end default: begin : d13 initial $display("@0_13 def"); end endcase
  case (19'h5ecd) 16'sh5ecd: begin : h14 initial $display("@0_14 hit"); end default: begin : d14 initial $display("@0_14 def"); end endcase
  case (24269) 16'sh5ecd: begin : h15 initial $display("@0_15 hit"); end default: begin : d15 initial $display("@0_15 def"); end endcase
  case (64'h5ecd) 16'sh5ecd: begin : h16 initial $display("@0_16 hit"); end default: begin : d16 initial $display("@0_16 def"); end endcase
  case (24269) 16'sh5ecd: begin : h17 initial $display("@0_17 hit"); end default: begin : d17 initial $display("@0_17 def"); end endcase
  case (31'he75ae10) $signed((31'sh71ceb5c2 << 3)): begin : h18 initial $display("@0_18 hit"); end default: begin : d18 initial $display("@0_18 def"); end endcase
  case (31'she75ae10) $signed((31'sh71ceb5c2 << 3)): begin : h19 initial $display("@0_19 hit"); end default: begin : d19 initial $display("@0_19 def"); end endcase
  case (34'he75ae10) $signed((31'sh71ceb5c2 << 3)): begin : h20 initial $display("@0_20 hit"); end default: begin : d20 initial $display("@0_20 def"); end endcase
  case (242593296) $signed((31'sh71ceb5c2 << 3)): begin : h21 initial $display("@0_21 hit"); end default: begin : d21 initial $display("@0_21 def"); end endcase
  case (64'he75ae10) $signed((31'sh71ceb5c2 << 3)): begin : h22 initial $display("@0_22 hit"); end default: begin : d22 initial $display("@0_22 def"); end endcase
  case (242593296) $signed((31'sh71ceb5c2 << 3)): begin : h23 initial $display("@0_23 hit"); end default: begin : d23 initial $display("@0_23 def"); end endcase
  case (1'h1) ((I == 3'sh0) || U32): begin : h24 initial $display("@0_24 hit"); end default: begin : d24 initial $display("@0_24 def"); end endcase
endmodule
