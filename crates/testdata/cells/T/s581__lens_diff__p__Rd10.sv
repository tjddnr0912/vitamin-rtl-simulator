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
  case (1'h1) (~^{S65, S65}): begin : h0 initial $display("@Rd10_0 hit"); end default: begin : d0 initial $display("@Rd10_0 def"); end endcase
  case (1'sh1) (~^{S65, S65}): begin : h1 initial $display("@Rd10_1 hit"); end default: begin : d1 initial $display("@Rd10_1 def"); end endcase
  case (4'h1) (~^{S65, S65}): begin : h2 initial $display("@Rd10_2 hit"); end default: begin : d2 initial $display("@Rd10_2 def"); end endcase
  case (64'hffffffffffffffff) (~^{S65, S65}): begin : h3 initial $display("@Rd10_3 hit"); end default: begin : d3 initial $display("@Rd10_3 def"); end endcase
  case ((-1)) (~^{S65, S65}): begin : h4 initial $display("@Rd10_4 hit"); end default: begin : d4 initial $display("@Rd10_4 def"); end endcase
  case (1) (~^{S65, S65}): begin : h5 initial $display("@Rd10_5 hit"); end default: begin : d5 initial $display("@Rd10_5 def"); end endcase
  case (33'h1541819a6) unsigned'(33'h1541819a6): begin : h6 initial $display("@Rd10_6 hit"); end default: begin : d6 initial $display("@Rd10_6 def"); end endcase
  case (33'sh1541819a6) unsigned'(33'h1541819a6): begin : h7 initial $display("@Rd10_7 hit"); end default: begin : d7 initial $display("@Rd10_7 def"); end endcase
  case (36'h1541819a6) unsigned'(33'h1541819a6): begin : h8 initial $display("@Rd10_8 hit"); end default: begin : d8 initial $display("@Rd10_8 def"); end endcase
  case (64'hffffffff541819a6) unsigned'(33'h1541819a6): begin : h9 initial $display("@Rd10_9 hit"); end default: begin : d9 initial $display("@Rd10_9 def"); end endcase
  case (1'h0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h10 initial $display("@Rd10_10 hit"); end default: begin : d10 initial $display("@Rd10_10 def"); end endcase
  case (1'sh0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h11 initial $display("@Rd10_11 hit"); end default: begin : d11 initial $display("@Rd10_11 def"); end endcase
  case (4'h0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h12 initial $display("@Rd10_12 hit"); end default: begin : d12 initial $display("@Rd10_12 def"); end endcase
  case (64'h0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h13 initial $display("@Rd10_13 hit"); end default: begin : d13 initial $display("@Rd10_13 def"); end endcase
  case (0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h14 initial $display("@Rd10_14 hit"); end default: begin : d14 initial $display("@Rd10_14 def"); end endcase
  case (0) (-((P4 - S4) == (~&32'hf96ffcd8))): begin : h15 initial $display("@Rd10_15 hit"); end default: begin : d15 initial $display("@Rd10_15 def"); end endcase
  case (32'h1e) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h16 initial $display("@Rd10_16 hit"); end default: begin : d16 initial $display("@Rd10_16 def"); end endcase
  case (32'sh1e) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h17 initial $display("@Rd10_17 hit"); end default: begin : d17 initial $display("@Rd10_17 def"); end endcase
  case (35'h1e) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h18 initial $display("@Rd10_18 hit"); end default: begin : d18 initial $display("@Rd10_18 def"); end endcase
  case (64'h1e) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h19 initial $display("@Rd10_19 hit"); end default: begin : d19 initial $display("@Rd10_19 def"); end endcase
  case (30) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h20 initial $display("@Rd10_20 hit"); end default: begin : d20 initial $display("@Rd10_20 def"); end endcase
  case (30) ((3'h4 | S65) ? 30 : (1'sh0 < W6)): begin : h21 initial $display("@Rd10_21 hit"); end default: begin : d21 initial $display("@Rd10_21 def"); end endcase
  case (1'h0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h22 initial $display("@Rd10_22 hit"); end default: begin : d22 initial $display("@Rd10_22 def"); end endcase
  case (1'sh0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h23 initial $display("@Rd10_23 hit"); end default: begin : d23 initial $display("@Rd10_23 def"); end endcase
  case (4'h0) ($signed((S4 ^ 14)) == ((P8 & 31'sh5d4de2d8) * {2{S4}})): begin : h24 initial $display("@Rd10_24 hit"); end default: begin : d24 initial $display("@Rd10_24 def"); end endcase
endmodule
