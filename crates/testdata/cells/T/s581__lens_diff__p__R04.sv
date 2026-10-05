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
  case (64'h556dbbef4fabcad8) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h0 initial $display("@4_0 hit"); end default: begin : d0 initial $display("@4_0 def"); end endcase
  case (6155782902193572568) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h1 initial $display("@4_1 hit"); end default: begin : d1 initial $display("@4_1 def"); end endcase
  case (16'he5ce) 16'she5ce: begin : h2 initial $display("@4_2 hit"); end default: begin : d2 initial $display("@4_2 def"); end endcase
  case (16'she5ce) 16'she5ce: begin : h3 initial $display("@4_3 hit"); end default: begin : d3 initial $display("@4_3 def"); end endcase
  case (19'he5ce) 16'she5ce: begin : h4 initial $display("@4_4 hit"); end default: begin : d4 initial $display("@4_4 def"); end endcase
  case ((-6706)) 16'she5ce: begin : h5 initial $display("@4_5 hit"); end default: begin : d5 initial $display("@4_5 def"); end endcase
  case (64'hffffffffffffe5ce) 16'she5ce: begin : h6 initial $display("@4_6 hit"); end default: begin : d6 initial $display("@4_6 def"); end endcase
  case (58830) 16'she5ce: begin : h7 initial $display("@4_7 hit"); end default: begin : d7 initial $display("@4_7 def"); end endcase
  case (4'h0) ((5'ha == I) - (UN << 3)): begin : h8 initial $display("@4_8 hit"); end default: begin : d8 initial $display("@4_8 def"); end endcase
  case (4'sh0) ((5'ha == I) - (UN << 3)): begin : h9 initial $display("@4_9 hit"); end default: begin : d9 initial $display("@4_9 def"); end endcase
  case (7'h0) ((5'ha == I) - (UN << 3)): begin : h10 initial $display("@4_10 hit"); end default: begin : d10 initial $display("@4_10 def"); end endcase
  case (0) ((5'ha == I) - (UN << 3)): begin : h11 initial $display("@4_11 hit"); end default: begin : d11 initial $display("@4_11 def"); end endcase
  case (64'h0) ((5'ha == I) - (UN << 3)): begin : h12 initial $display("@4_12 hit"); end default: begin : d12 initial $display("@4_12 def"); end endcase
  case (0) ((5'ha == I) - (UN << 3)): begin : h13 initial $display("@4_13 hit"); end default: begin : d13 initial $display("@4_13 def"); end endcase
  case (8'h7) ((~P8) >> 1): begin : h14 initial $display("@4_14 hit"); end default: begin : d14 initial $display("@4_14 def"); end endcase
  case (8'sh7) ((~P8) >> 1): begin : h15 initial $display("@4_15 hit"); end default: begin : d15 initial $display("@4_15 def"); end endcase
  case (11'h7) ((~P8) >> 1): begin : h16 initial $display("@4_16 hit"); end default: begin : d16 initial $display("@4_16 def"); end endcase
  case (7) ((~P8) >> 1): begin : h17 initial $display("@4_17 hit"); end default: begin : d17 initial $display("@4_17 def"); end endcase
  case (64'h7) ((~P8) >> 1): begin : h18 initial $display("@4_18 hit"); end default: begin : d18 initial $display("@4_18 def"); end endcase
  case (7) ((~P8) >> 1): begin : h19 initial $display("@4_19 hit"); end default: begin : d19 initial $display("@4_19 def"); end endcase
  case (1'h1) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h20 initial $display("@4_20 hit"); end default: begin : d20 initial $display("@4_20 def"); end endcase
  case (1'sh1) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h21 initial $display("@4_21 hit"); end default: begin : d21 initial $display("@4_21 def"); end endcase
  case (4'h1) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h22 initial $display("@4_22 hit"); end default: begin : d22 initial $display("@4_22 def"); end endcase
  case ((-1)) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h23 initial $display("@4_23 hit"); end default: begin : d23 initial $display("@4_23 def"); end endcase
  case (64'hffffffffffffffff) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h24 initial $display("@4_24 hit"); end default: begin : d24 initial $display("@4_24 def"); end endcase
endmodule
