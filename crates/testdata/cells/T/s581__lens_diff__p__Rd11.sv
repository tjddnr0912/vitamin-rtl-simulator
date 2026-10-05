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
  typedef logic signed [5:0] s6_t;
  localparam int W6 = 6;
  localparam logic [127:0] P128 = {64'hFFFF_FFFF_FFFF_FFFF, 64'h0123_4567_89AB_CDEF};
  case (64'h0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h0 initial $display("@Rd11_0 hit"); end default: begin : d0 initial $display("@Rd11_0 def"); end endcase
  case (0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h1 initial $display("@Rd11_1 hit"); end default: begin : d1 initial $display("@Rd11_1 def"); end endcase
  case (0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h2 initial $display("@Rd11_2 hit"); end default: begin : d2 initial $display("@Rd11_2 def"); end endcase
  case (32'h1) (I & (P4 >>> 3)): begin : h3 initial $display("@Rd11_3 hit"); end default: begin : d3 initial $display("@Rd11_3 def"); end endcase
  case (32'sh1) (I & (P4 >>> 3)): begin : h4 initial $display("@Rd11_4 hit"); end default: begin : d4 initial $display("@Rd11_4 def"); end endcase
  case (35'h1) (I & (P4 >>> 3)): begin : h5 initial $display("@Rd11_5 hit"); end default: begin : d5 initial $display("@Rd11_5 def"); end endcase
  case (64'h1) (I & (P4 >>> 3)): begin : h6 initial $display("@Rd11_6 hit"); end default: begin : d6 initial $display("@Rd11_6 def"); end endcase
  case (1) (I & (P4 >>> 3)): begin : h7 initial $display("@Rd11_7 hit"); end default: begin : d7 initial $display("@Rd11_7 def"); end endcase
  case (1) (I & (P4 >>> 3)): begin : h8 initial $display("@Rd11_8 hit"); end default: begin : d8 initial $display("@Rd11_8 def"); end endcase
  case (32'hfffffffa) (-6): begin : h9 initial $display("@Rd11_9 hit"); end default: begin : d9 initial $display("@Rd11_9 def"); end endcase
  case (32'shfffffffa) (-6): begin : h10 initial $display("@Rd11_10 hit"); end default: begin : d10 initial $display("@Rd11_10 def"); end endcase
  case (35'hfffffffa) (-6): begin : h11 initial $display("@Rd11_11 hit"); end default: begin : d11 initial $display("@Rd11_11 def"); end endcase
  case (64'hfffffffffffffffa) (-6): begin : h12 initial $display("@Rd11_12 hit"); end default: begin : d12 initial $display("@Rd11_12 def"); end endcase
  case ((-6)) (-6): begin : h13 initial $display("@Rd11_13 hit"); end default: begin : d13 initial $display("@Rd11_13 def"); end endcase
  case (1'h0) (UN == P65): begin : h14 initial $display("@Rd11_14 hit"); end default: begin : d14 initial $display("@Rd11_14 def"); end endcase
  case (1'sh0) (UN == P65): begin : h15 initial $display("@Rd11_15 hit"); end default: begin : d15 initial $display("@Rd11_15 def"); end endcase
  case (4'h0) (UN == P65): begin : h16 initial $display("@Rd11_16 hit"); end default: begin : d16 initial $display("@Rd11_16 def"); end endcase
  case (64'h0) (UN == P65): begin : h17 initial $display("@Rd11_17 hit"); end default: begin : d17 initial $display("@Rd11_17 def"); end endcase
  case (0) (UN == P65): begin : h18 initial $display("@Rd11_18 hit"); end default: begin : d18 initial $display("@Rd11_18 def"); end endcase
  case (0) (UN == P65): begin : h19 initial $display("@Rd11_19 hit"); end default: begin : d19 initial $display("@Rd11_19 def"); end endcase
  case (6'h1c) 6'(S8): begin : h20 initial $display("@Rd11_20 hit"); end default: begin : d20 initial $display("@Rd11_20 def"); end endcase
  case (6'sh1c) 6'(S8): begin : h21 initial $display("@Rd11_21 hit"); end default: begin : d21 initial $display("@Rd11_21 def"); end endcase
  case (9'h1c) 6'(S8): begin : h22 initial $display("@Rd11_22 hit"); end default: begin : d22 initial $display("@Rd11_22 def"); end endcase
  case (64'h1c) 6'(S8): begin : h23 initial $display("@Rd11_23 hit"); end default: begin : d23 initial $display("@Rd11_23 def"); end endcase
  case (28) 6'(S8): begin : h24 initial $display("@Rd11_24 hit"); end default: begin : d24 initial $display("@Rd11_24 def"); end endcase
endmodule
