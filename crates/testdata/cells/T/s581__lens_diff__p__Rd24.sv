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
  case (12) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h0 initial $display("@Rd24_0 hit"); end default: begin : d0 initial $display("@Rd24_0 def"); end endcase
  case (1'h1) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h1 initial $display("@Rd24_1 hit"); end default: begin : d1 initial $display("@Rd24_1 def"); end endcase
  case (1'sh1) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h2 initial $display("@Rd24_2 hit"); end default: begin : d2 initial $display("@Rd24_2 def"); end endcase
  case (4'h1) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h3 initial $display("@Rd24_3 hit"); end default: begin : d3 initial $display("@Rd24_3 def"); end endcase
  case (64'hffffffffffffffff) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h4 initial $display("@Rd24_4 hit"); end default: begin : d4 initial $display("@Rd24_4 def"); end endcase
  case ((-1)) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h5 initial $display("@Rd24_5 hit"); end default: begin : d5 initial $display("@Rd24_5 def"); end endcase
  case (1) (((P4 | S4) >= $unsigned(5)) >= $onehot0(8'sh9C)): begin : h6 initial $display("@Rd24_6 hit"); end default: begin : d6 initial $display("@Rd24_6 def"); end endcase
  case (4'ha) UN: begin : h7 initial $display("@Rd24_7 hit"); end default: begin : d7 initial $display("@Rd24_7 def"); end endcase
  case (4'sha) UN: begin : h8 initial $display("@Rd24_8 hit"); end default: begin : d8 initial $display("@Rd24_8 def"); end endcase
  case (7'ha) UN: begin : h9 initial $display("@Rd24_9 hit"); end default: begin : d9 initial $display("@Rd24_9 def"); end endcase
  case (64'hfffffffffffffffa) UN: begin : h10 initial $display("@Rd24_10 hit"); end default: begin : d10 initial $display("@Rd24_10 def"); end endcase
  case ((-6)) UN: begin : h11 initial $display("@Rd24_11 hit"); end default: begin : d11 initial $display("@Rd24_11 def"); end endcase
  case (10) UN: begin : h12 initial $display("@Rd24_12 hit"); end default: begin : d12 initial $display("@Rd24_12 def"); end endcase
  case (32'hfffffff0) U32: begin : h13 initial $display("@Rd24_13 hit"); end default: begin : d13 initial $display("@Rd24_13 def"); end endcase
  case (32'shfffffff0) U32: begin : h14 initial $display("@Rd24_14 hit"); end default: begin : d14 initial $display("@Rd24_14 def"); end endcase
  case (35'hfffffff0) U32: begin : h15 initial $display("@Rd24_15 hit"); end default: begin : d15 initial $display("@Rd24_15 def"); end endcase
  case (64'hfffffffffffffff0) U32: begin : h16 initial $display("@Rd24_16 hit"); end default: begin : d16 initial $display("@Rd24_16 def"); end endcase
  case ((-16)) U32: begin : h17 initial $display("@Rd24_17 hit"); end default: begin : d17 initial $display("@Rd24_17 def"); end endcase
  case (37'h100000001c) {33'h1_0000_0001, P4}: begin : h18 initial $display("@Rd24_18 hit"); end default: begin : d18 initial $display("@Rd24_18 def"); end endcase
  case (37'sh100000001c) {33'h1_0000_0001, P4}: begin : h19 initial $display("@Rd24_19 hit"); end default: begin : d19 initial $display("@Rd24_19 def"); end endcase
  case (40'h100000001c) {33'h1_0000_0001, P4}: begin : h20 initial $display("@Rd24_20 hit"); end default: begin : d20 initial $display("@Rd24_20 def"); end endcase
  case (64'hfffffff00000001c) {33'h1_0000_0001, P4}: begin : h21 initial $display("@Rd24_21 hit"); end default: begin : d21 initial $display("@Rd24_21 def"); end endcase
  case (64'h91239dedce384ca6) 64'sh91239dedce384ca6: begin : h22 initial $display("@Rd24_22 hit"); end default: begin : d22 initial $display("@Rd24_22 def"); end endcase
  case (64'sh91239dedce384ca6) 64'sh91239dedce384ca6: begin : h23 initial $display("@Rd24_23 hit"); end default: begin : d23 initial $display("@Rd24_23 def"); end endcase
  case (64'h91239dedce384ca6) 64'sh91239dedce384ca6: begin : h24 initial $display("@Rd24_24 hit"); end default: begin : d24 initial $display("@Rd24_24 def"); end endcase
endmodule
