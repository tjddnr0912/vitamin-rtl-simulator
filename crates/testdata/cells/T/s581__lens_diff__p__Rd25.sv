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
  case (64'h91239dedce384ca6) 64'sh91239dedce384ca6: begin : h0 initial $display("@Rd25_0 hit"); end default: begin : d0 initial $display("@Rd25_0 def"); end endcase
  case (32'h2) $countones(33'h1_0000_0001): begin : h1 initial $display("@Rd25_1 hit"); end default: begin : d1 initial $display("@Rd25_1 def"); end endcase
  case (32'sh2) $countones(33'h1_0000_0001): begin : h2 initial $display("@Rd25_2 hit"); end default: begin : d2 initial $display("@Rd25_2 def"); end endcase
  case (35'h2) $countones(33'h1_0000_0001): begin : h3 initial $display("@Rd25_3 hit"); end default: begin : d3 initial $display("@Rd25_3 def"); end endcase
  case (64'h2) $countones(33'h1_0000_0001): begin : h4 initial $display("@Rd25_4 hit"); end default: begin : d4 initial $display("@Rd25_4 def"); end endcase
  case (2) $countones(33'h1_0000_0001): begin : h5 initial $display("@Rd25_5 hit"); end default: begin : d5 initial $display("@Rd25_5 def"); end endcase
  case (2) $countones(33'h1_0000_0001): begin : h6 initial $display("@Rd25_6 hit"); end default: begin : d6 initial $display("@Rd25_6 def"); end endcase
  case (1'h0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h7 initial $display("@Rd25_7 hit"); end default: begin : d7 initial $display("@Rd25_7 def"); end endcase
  case (1'sh0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h8 initial $display("@Rd25_8 hit"); end default: begin : d8 initial $display("@Rd25_8 def"); end endcase
  case (4'h0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h9 initial $display("@Rd25_9 hit"); end default: begin : d9 initial $display("@Rd25_9 def"); end endcase
  case (64'h0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h10 initial $display("@Rd25_10 hit"); end default: begin : d10 initial $display("@Rd25_10 def"); end endcase
  case (0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h11 initial $display("@Rd25_11 hit"); end default: begin : d11 initial $display("@Rd25_11 def"); end endcase
  case (0) ((&(31'h5203921c - P8)) >= (31'h74916d93 | (63'sh3794efb82e1780d / P65))): begin : h12 initial $display("@Rd25_12 hit"); end default: begin : d12 initial $display("@Rd25_12 def"); end endcase
  case (32'h1) ((~&S8) % 33): begin : h13 initial $display("@Rd25_13 hit"); end default: begin : d13 initial $display("@Rd25_13 def"); end endcase
  case (32'sh1) ((~&S8) % 33): begin : h14 initial $display("@Rd25_14 hit"); end default: begin : d14 initial $display("@Rd25_14 def"); end endcase
  case (35'h1) ((~&S8) % 33): begin : h15 initial $display("@Rd25_15 hit"); end default: begin : d15 initial $display("@Rd25_15 def"); end endcase
  case (64'h1) ((~&S8) % 33): begin : h16 initial $display("@Rd25_16 hit"); end default: begin : d16 initial $display("@Rd25_16 def"); end endcase
  case (1) ((~&S8) % 33): begin : h17 initial $display("@Rd25_17 hit"); end default: begin : d17 initial $display("@Rd25_17 def"); end endcase
  case (1) ((~&S8) % 33): begin : h18 initial $display("@Rd25_18 hit"); end default: begin : d18 initial $display("@Rd25_18 def"); end endcase
  case (32'hf) signed'(15): begin : h19 initial $display("@Rd25_19 hit"); end default: begin : d19 initial $display("@Rd25_19 def"); end endcase
  case (32'shf) signed'(15): begin : h20 initial $display("@Rd25_20 hit"); end default: begin : d20 initial $display("@Rd25_20 def"); end endcase
  case (35'hf) signed'(15): begin : h21 initial $display("@Rd25_21 hit"); end default: begin : d21 initial $display("@Rd25_21 def"); end endcase
  case (64'hf) signed'(15): begin : h22 initial $display("@Rd25_22 hit"); end default: begin : d22 initial $display("@Rd25_22 def"); end endcase
  case (15) signed'(15): begin : h23 initial $display("@Rd25_23 hit"); end default: begin : d23 initial $display("@Rd25_23 def"); end endcase
  case (15) signed'(15): begin : h24 initial $display("@Rd25_24 hit"); end default: begin : d24 initial $display("@Rd25_24 def"); end endcase
endmodule
