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
  case (64'hffffffffffffff9c) S8: begin : h0 initial $display("@Rb6_0 hit"); end default: begin : d0 initial $display("@Rb6_0 def"); end endcase
  case ((-100)) S8: begin : h1 initial $display("@Rb6_1 hit"); end default: begin : d1 initial $display("@Rb6_1 def"); end endcase
  case (156) S8: begin : h2 initial $display("@Rb6_2 hit"); end default: begin : d2 initial $display("@Rb6_2 def"); end endcase
  case (32'h1) $unsigned(1): begin : h3 initial $display("@Rb6_3 hit"); end default: begin : d3 initial $display("@Rb6_3 def"); end endcase
  case (32'sh1) $unsigned(1): begin : h4 initial $display("@Rb6_4 hit"); end default: begin : d4 initial $display("@Rb6_4 def"); end endcase
  case (35'h1) $unsigned(1): begin : h5 initial $display("@Rb6_5 hit"); end default: begin : d5 initial $display("@Rb6_5 def"); end endcase
  case (64'h1) $unsigned(1): begin : h6 initial $display("@Rb6_6 hit"); end default: begin : d6 initial $display("@Rb6_6 def"); end endcase
  case (1) $unsigned(1): begin : h7 initial $display("@Rb6_7 hit"); end default: begin : d7 initial $display("@Rb6_7 def"); end endcase
  case (1) $unsigned(1): begin : h8 initial $display("@Rb6_8 hit"); end default: begin : d8 initial $display("@Rb6_8 def"); end endcase
  case (4'ha) $signed(4'ha): begin : h9 initial $display("@Rb6_9 hit"); end default: begin : d9 initial $display("@Rb6_9 def"); end endcase
  case (4'sha) $signed(4'ha): begin : h10 initial $display("@Rb6_10 hit"); end default: begin : d10 initial $display("@Rb6_10 def"); end endcase
  case (7'ha) $signed(4'ha): begin : h11 initial $display("@Rb6_11 hit"); end default: begin : d11 initial $display("@Rb6_11 def"); end endcase
  case (64'hfffffffffffffffa) $signed(4'ha): begin : h12 initial $display("@Rb6_12 hit"); end default: begin : d12 initial $display("@Rb6_12 def"); end endcase
  case ((-6)) $signed(4'ha): begin : h13 initial $display("@Rb6_13 hit"); end default: begin : d13 initial $display("@Rb6_13 def"); end endcase
  case (10) $signed(4'ha): begin : h14 initial $display("@Rb6_14 hit"); end default: begin : d14 initial $display("@Rb6_14 def"); end endcase
  case (4'h2) (~S4): begin : h15 initial $display("@Rb6_15 hit"); end default: begin : d15 initial $display("@Rb6_15 def"); end endcase
  case (4'sh2) (~S4): begin : h16 initial $display("@Rb6_16 hit"); end default: begin : d16 initial $display("@Rb6_16 def"); end endcase
  case (7'h2) (~S4): begin : h17 initial $display("@Rb6_17 hit"); end default: begin : d17 initial $display("@Rb6_17 def"); end endcase
  case (64'h2) (~S4): begin : h18 initial $display("@Rb6_18 hit"); end default: begin : d18 initial $display("@Rb6_18 def"); end endcase
  case (2) (~S4): begin : h19 initial $display("@Rb6_19 hit"); end default: begin : d19 initial $display("@Rb6_19 def"); end endcase
  case (2) (~S4): begin : h20 initial $display("@Rb6_20 hit"); end default: begin : d20 initial $display("@Rb6_20 def"); end endcase
  case (5'h4) 5'sh4: begin : h21 initial $display("@Rb6_21 hit"); end default: begin : d21 initial $display("@Rb6_21 def"); end endcase
  case (5'sh4) 5'sh4: begin : h22 initial $display("@Rb6_22 hit"); end default: begin : d22 initial $display("@Rb6_22 def"); end endcase
  case (8'h4) 5'sh4: begin : h23 initial $display("@Rb6_23 hit"); end default: begin : d23 initial $display("@Rb6_23 def"); end endcase
  case (64'h4) 5'sh4: begin : h24 initial $display("@Rb6_24 hit"); end default: begin : d24 initial $display("@Rb6_24 def"); end endcase
endmodule
