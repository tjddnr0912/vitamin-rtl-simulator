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
  case (1'sh0) (!(S8 >>> 2)): begin : h0 initial $display("@Rb14_0 hit"); end default: begin : d0 initial $display("@Rb14_0 def"); end endcase
  case (4'h0) (!(S8 >>> 2)): begin : h1 initial $display("@Rb14_1 hit"); end default: begin : d1 initial $display("@Rb14_1 def"); end endcase
  case (64'h0) (!(S8 >>> 2)): begin : h2 initial $display("@Rb14_2 hit"); end default: begin : d2 initial $display("@Rb14_2 def"); end endcase
  case (0) (!(S8 >>> 2)): begin : h3 initial $display("@Rb14_3 hit"); end default: begin : d3 initial $display("@Rb14_3 def"); end endcase
  case (0) (!(S8 >>> 2)): begin : h4 initial $display("@Rb14_4 hit"); end default: begin : d4 initial $display("@Rb14_4 def"); end endcase
  case (1'h0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h5 initial $display("@Rb14_5 hit"); end default: begin : d5 initial $display("@Rb14_5 def"); end endcase
  case (1'sh0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h6 initial $display("@Rb14_6 hit"); end default: begin : d6 initial $display("@Rb14_6 def"); end endcase
  case (4'h0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h7 initial $display("@Rb14_7 hit"); end default: begin : d7 initial $display("@Rb14_7 def"); end endcase
  case (64'h0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h8 initial $display("@Rb14_8 hit"); end default: begin : d8 initial $display("@Rb14_8 def"); end endcase
  case (0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h9 initial $display("@Rb14_9 hit"); end default: begin : d9 initial $display("@Rb14_9 def"); end endcase
  case (0) (($unsigned(S65) - {P8, 4'(3'h0)}) ? (((-2) ? P8 : 22) < (!(-1))) : ((16'h4d1c && P8) == {P65, 8'(33'hb1263b67)})): begin : h10 initial $display("@Rb14_10 hit"); end default: begin : d10 initial $display("@Rb14_10 def"); end endcase
  case (63'h14adb82a03ab9691) 63'sh14adb82a03ab9691: begin : h11 initial $display("@Rb14_11 hit"); end default: begin : d11 initial $display("@Rb14_11 def"); end endcase
  case (63'sh14adb82a03ab9691) 63'sh14adb82a03ab9691: begin : h12 initial $display("@Rb14_12 hit"); end default: begin : d12 initial $display("@Rb14_12 def"); end endcase
  case (64'h14adb82a03ab9691) 63'sh14adb82a03ab9691: begin : h13 initial $display("@Rb14_13 hit"); end default: begin : d13 initial $display("@Rb14_13 def"); end endcase
  case (64'h14adb82a03ab9691) 63'sh14adb82a03ab9691: begin : h14 initial $display("@Rb14_14 hit"); end default: begin : d14 initial $display("@Rb14_14 def"); end endcase
  case (2'h1) ({2{1'((-5))}} * 2'sh3): begin : h15 initial $display("@Rb14_15 hit"); end default: begin : d15 initial $display("@Rb14_15 def"); end endcase
  case (2'sh1) ({2{1'((-5))}} * 2'sh3): begin : h16 initial $display("@Rb14_16 hit"); end default: begin : d16 initial $display("@Rb14_16 def"); end endcase
  case (5'h1) ({2{1'((-5))}} * 2'sh3): begin : h17 initial $display("@Rb14_17 hit"); end default: begin : d17 initial $display("@Rb14_17 def"); end endcase
  case (64'h1) ({2{1'((-5))}} * 2'sh3): begin : h18 initial $display("@Rb14_18 hit"); end default: begin : d18 initial $display("@Rb14_18 def"); end endcase
  case (1) ({2{1'((-5))}} * 2'sh3): begin : h19 initial $display("@Rb14_19 hit"); end default: begin : d19 initial $display("@Rb14_19 def"); end endcase
  case (1) ({2{1'((-5))}} * 2'sh3): begin : h20 initial $display("@Rb14_20 hit"); end default: begin : d20 initial $display("@Rb14_20 def"); end endcase
  case (11'h2f) ({3'(64'h7b65dfa530b57fd), 8'(L64)} >> 5): begin : h21 initial $display("@Rb14_21 hit"); end default: begin : d21 initial $display("@Rb14_21 def"); end endcase
  case (11'sh2f) ({3'(64'h7b65dfa530b57fd), 8'(L64)} >> 5): begin : h22 initial $display("@Rb14_22 hit"); end default: begin : d22 initial $display("@Rb14_22 def"); end endcase
  case (14'h2f) ({3'(64'h7b65dfa530b57fd), 8'(L64)} >> 5): begin : h23 initial $display("@Rb14_23 hit"); end default: begin : d23 initial $display("@Rb14_23 def"); end endcase
  case (64'h2f) ({3'(64'h7b65dfa530b57fd), 8'(L64)} >> 5): begin : h24 initial $display("@Rb14_24 hit"); end default: begin : d24 initial $display("@Rb14_24 def"); end endcase
endmodule
