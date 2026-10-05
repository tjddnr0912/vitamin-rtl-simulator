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
  case (1'h0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h0 initial $display("@Ra16_0 hit"); end default: begin : d0 initial $display("@Ra16_0 def"); end endcase
  case (1'sh0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h1 initial $display("@Ra16_1 hit"); end default: begin : d1 initial $display("@Ra16_1 def"); end endcase
  case (4'h0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h2 initial $display("@Ra16_2 hit"); end default: begin : d2 initial $display("@Ra16_2 def"); end endcase
  case (64'h0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h3 initial $display("@Ra16_3 hit"); end default: begin : d3 initial $display("@Ra16_3 def"); end endcase
  case (0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h4 initial $display("@Ra16_4 hit"); end default: begin : d4 initial $display("@Ra16_4 def"); end endcase
  case (0) (((~P4) + (!S4)) == ((P8 << 2) ^ (40 | 35))): begin : h5 initial $display("@Ra16_5 hit"); end default: begin : d5 initial $display("@Ra16_5 def"); end endcase
  case (64'hfffffffffffffffb) $unsigned((L64 + (P4 & L64))): begin : h6 initial $display("@Ra16_6 hit"); end default: begin : d6 initial $display("@Ra16_6 def"); end endcase
  case (64'shfffffffffffffffb) $unsigned((L64 + (P4 & L64))): begin : h7 initial $display("@Ra16_7 hit"); end default: begin : d7 initial $display("@Ra16_7 def"); end endcase
  case (64'hfffffffffffffffb) $unsigned((L64 + (P4 & L64))): begin : h8 initial $display("@Ra16_8 hit"); end default: begin : d8 initial $display("@Ra16_8 def"); end endcase
  case (64'hfffffffffffffffb) $unsigned((L64 + (P4 & L64))): begin : h9 initial $display("@Ra16_9 hit"); end default: begin : d9 initial $display("@Ra16_9 def"); end endcase
  case ((-5)) $unsigned((L64 + (P4 & L64))): begin : h10 initial $display("@Ra16_10 hit"); end default: begin : d10 initial $display("@Ra16_10 def"); end endcase
  case (1'h1) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h11 initial $display("@Ra16_11 hit"); end default: begin : d11 initial $display("@Ra16_11 def"); end endcase
  case (1'sh1) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h12 initial $display("@Ra16_12 hit"); end default: begin : d12 initial $display("@Ra16_12 def"); end endcase
  case (4'h1) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h13 initial $display("@Ra16_13 hit"); end default: begin : d13 initial $display("@Ra16_13 def"); end endcase
  case (64'hffffffffffffffff) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h14 initial $display("@Ra16_14 hit"); end default: begin : d14 initial $display("@Ra16_14 def"); end endcase
  case ((-1)) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h15 initial $display("@Ra16_15 hit"); end default: begin : d15 initial $display("@Ra16_15 def"); end endcase
  case (1) ((32'h6a4d76e6 ^ 2'sh0) || {1'(S65), 8'sh9C}): begin : h16 initial $display("@Ra16_16 hit"); end default: begin : d16 initial $display("@Ra16_16 def"); end endcase
  case (2'h3) ((-2'sh2) >>> 2): begin : h17 initial $display("@Ra16_17 hit"); end default: begin : d17 initial $display("@Ra16_17 def"); end endcase
  case (2'sh3) ((-2'sh2) >>> 2): begin : h18 initial $display("@Ra16_18 hit"); end default: begin : d18 initial $display("@Ra16_18 def"); end endcase
  case (5'h3) ((-2'sh2) >>> 2): begin : h19 initial $display("@Ra16_19 hit"); end default: begin : d19 initial $display("@Ra16_19 def"); end endcase
  case (64'hffffffffffffffff) ((-2'sh2) >>> 2): begin : h20 initial $display("@Ra16_20 hit"); end default: begin : d20 initial $display("@Ra16_20 def"); end endcase
  case ((-1)) ((-2'sh2) >>> 2): begin : h21 initial $display("@Ra16_21 hit"); end default: begin : d21 initial $display("@Ra16_21 def"); end endcase
  case (3) ((-2'sh2) >>> 2): begin : h22 initial $display("@Ra16_22 hit"); end default: begin : d22 initial $display("@Ra16_22 def"); end endcase
  case (1'h0) $signed((!I)): begin : h23 initial $display("@Ra16_23 hit"); end default: begin : d23 initial $display("@Ra16_23 def"); end endcase
  case (1'sh0) $signed((!I)): begin : h24 initial $display("@Ra16_24 hit"); end default: begin : d24 initial $display("@Ra16_24 def"); end endcase
endmodule
