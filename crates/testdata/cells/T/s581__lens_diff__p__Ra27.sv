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
  case (0) (!P4): begin : h0 initial $display("@Ra27_0 hit"); end default: begin : d0 initial $display("@Ra27_0 def"); end endcase
  case (64'hfffffffffffffffd) (((L64 / P65) - UN) ? ((4'h3 ? (-3) : 64'h953b1a8b3132b388) + (!64'hcf86926984b9bda5)) : ($unsigned(P65) || (4'shd - U32))): begin : h1 initial $display("@Ra27_1 hit"); end default: begin : d1 initial $display("@Ra27_1 def"); end endcase
  case (64'shfffffffffffffffd) (((L64 / P65) - UN) ? ((4'h3 ? (-3) : 64'h953b1a8b3132b388) + (!64'hcf86926984b9bda5)) : ($unsigned(P65) || (4'shd - U32))): begin : h2 initial $display("@Ra27_2 hit"); end default: begin : d2 initial $display("@Ra27_2 def"); end endcase
  case (64'hfffffffffffffffd) (((L64 / P65) - UN) ? ((4'h3 ? (-3) : 64'h953b1a8b3132b388) + (!64'hcf86926984b9bda5)) : ($unsigned(P65) || (4'shd - U32))): begin : h3 initial $display("@Ra27_3 hit"); end default: begin : d3 initial $display("@Ra27_3 def"); end endcase
  case (64'hfffffffffffffffd) (((L64 / P65) - UN) ? ((4'h3 ? (-3) : 64'h953b1a8b3132b388) + (!64'hcf86926984b9bda5)) : ($unsigned(P65) || (4'shd - U32))): begin : h4 initial $display("@Ra27_4 hit"); end default: begin : d4 initial $display("@Ra27_4 def"); end endcase
  case ((-3)) (((L64 / P65) - UN) ? ((4'h3 ? (-3) : 64'h953b1a8b3132b388) + (!64'hcf86926984b9bda5)) : ($unsigned(P65) || (4'shd - U32))): begin : h5 initial $display("@Ra27_5 hit"); end default: begin : d5 initial $display("@Ra27_5 def"); end endcase
  case (32'h2) (32'sh5b51e2c0 % 6): begin : h6 initial $display("@Ra27_6 hit"); end default: begin : d6 initial $display("@Ra27_6 def"); end endcase
  case (32'sh2) (32'sh5b51e2c0 % 6): begin : h7 initial $display("@Ra27_7 hit"); end default: begin : d7 initial $display("@Ra27_7 def"); end endcase
  case (35'h2) (32'sh5b51e2c0 % 6): begin : h8 initial $display("@Ra27_8 hit"); end default: begin : d8 initial $display("@Ra27_8 def"); end endcase
  case (64'h2) (32'sh5b51e2c0 % 6): begin : h9 initial $display("@Ra27_9 hit"); end default: begin : d9 initial $display("@Ra27_9 def"); end endcase
  case (2) (32'sh5b51e2c0 % 6): begin : h10 initial $display("@Ra27_10 hit"); end default: begin : d10 initial $display("@Ra27_10 def"); end endcase
  case (2) (32'sh5b51e2c0 % 6): begin : h11 initial $display("@Ra27_11 hit"); end default: begin : d11 initial $display("@Ra27_11 def"); end endcase
  case (1'h0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h12 initial $display("@Ra27_12 hit"); end default: begin : d12 initial $display("@Ra27_12 def"); end endcase
  case (1'sh0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h13 initial $display("@Ra27_13 hit"); end default: begin : d13 initial $display("@Ra27_13 def"); end endcase
  case (4'h0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h14 initial $display("@Ra27_14 hit"); end default: begin : d14 initial $display("@Ra27_14 def"); end endcase
  case (64'h0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h15 initial $display("@Ra27_15 hit"); end default: begin : d15 initial $display("@Ra27_15 def"); end endcase
  case (0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h16 initial $display("@Ra27_16 hit"); end default: begin : d16 initial $display("@Ra27_16 def"); end endcase
  case (0) ((!(S4 >> 0)) + (!{4'(33'h13d05a4cb), P8})): begin : h17 initial $display("@Ra27_17 hit"); end default: begin : d17 initial $display("@Ra27_17 def"); end endcase
  case (1'h1) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h18 initial $display("@Ra27_18 hit"); end default: begin : d18 initial $display("@Ra27_18 def"); end endcase
  case (1'sh1) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h19 initial $display("@Ra27_19 hit"); end default: begin : d19 initial $display("@Ra27_19 def"); end endcase
  case (4'h1) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h20 initial $display("@Ra27_20 hit"); end default: begin : d20 initial $display("@Ra27_20 def"); end endcase
  case (64'hffffffffffffffff) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h21 initial $display("@Ra27_21 hit"); end default: begin : d21 initial $display("@Ra27_21 def"); end endcase
  case ((-1)) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h22 initial $display("@Ra27_22 hit"); end default: begin : d22 initial $display("@Ra27_22 def"); end endcase
  case (1) ((5'sh3 | 4'h2) && {P4, 65'(UN)}): begin : h23 initial $display("@Ra27_23 hit"); end default: begin : d23 initial $display("@Ra27_23 def"); end endcase
  case (1'h1) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h24 initial $display("@Ra27_24 hit"); end default: begin : d24 initial $display("@Ra27_24 def"); end endcase
endmodule
