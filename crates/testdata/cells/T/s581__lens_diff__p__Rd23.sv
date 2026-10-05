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
  case (0) $onehot(33'h1_0000_0001): begin : h0 initial $display("@Rd23_0 hit"); end default: begin : d0 initial $display("@Rd23_0 def"); end endcase
  case (0) $onehot(33'h1_0000_0001): begin : h1 initial $display("@Rd23_1 hit"); end default: begin : d1 initial $display("@Rd23_1 def"); end endcase
  case (2'h0) (P65 ? 1'sh0 : 2'h2): begin : h2 initial $display("@Rd23_2 hit"); end default: begin : d2 initial $display("@Rd23_2 def"); end endcase
  case (2'sh0) (P65 ? 1'sh0 : 2'h2): begin : h3 initial $display("@Rd23_3 hit"); end default: begin : d3 initial $display("@Rd23_3 def"); end endcase
  case (5'h0) (P65 ? 1'sh0 : 2'h2): begin : h4 initial $display("@Rd23_4 hit"); end default: begin : d4 initial $display("@Rd23_4 def"); end endcase
  case (64'h0) (P65 ? 1'sh0 : 2'h2): begin : h5 initial $display("@Rd23_5 hit"); end default: begin : d5 initial $display("@Rd23_5 def"); end endcase
  case (0) (P65 ? 1'sh0 : 2'h2): begin : h6 initial $display("@Rd23_6 hit"); end default: begin : d6 initial $display("@Rd23_6 def"); end endcase
  case (0) (P65 ? 1'sh0 : 2'h2): begin : h7 initial $display("@Rd23_7 hit"); end default: begin : d7 initial $display("@Rd23_7 def"); end endcase
  case (6'h35) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h8 initial $display("@Rd23_8 hit"); end default: begin : d8 initial $display("@Rd23_8 def"); end endcase
  case (6'sh35) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h9 initial $display("@Rd23_9 hit"); end default: begin : d9 initial $display("@Rd23_9 def"); end endcase
  case (9'h35) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h10 initial $display("@Rd23_10 hit"); end default: begin : d10 initial $display("@Rd23_10 def"); end endcase
  case (64'hfffffffffffffff5) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h11 initial $display("@Rd23_11 hit"); end default: begin : d11 initial $display("@Rd23_11 def"); end endcase
  case ((-11)) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h12 initial $display("@Rd23_12 hit"); end default: begin : d12 initial $display("@Rd23_12 def"); end endcase
  case (53) (((|P128) - 3'h6) * s6_t'((P8 >>> 4))): begin : h13 initial $display("@Rd23_13 hit"); end default: begin : d13 initial $display("@Rd23_13 def"); end endcase
  case (32'h0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h14 initial $display("@Rd23_14 hit"); end default: begin : d14 initial $display("@Rd23_14 def"); end endcase
  case (32'sh0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h15 initial $display("@Rd23_15 hit"); end default: begin : d15 initial $display("@Rd23_15 def"); end endcase
  case (35'h0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h16 initial $display("@Rd23_16 hit"); end default: begin : d16 initial $display("@Rd23_16 def"); end endcase
  case (64'h0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h17 initial $display("@Rd23_17 hit"); end default: begin : d17 initial $display("@Rd23_17 def"); end endcase
  case (0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h18 initial $display("@Rd23_18 hit"); end default: begin : d18 initial $display("@Rd23_18 def"); end endcase
  case (0) ($clog2((^W6)) / {S4, 1'(P8)}): begin : h19 initial $display("@Rd23_19 hit"); end default: begin : d19 initial $display("@Rd23_19 def"); end endcase
  case (6'hc) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h20 initial $display("@Rd23_20 hit"); end default: begin : d20 initial $display("@Rd23_20 def"); end endcase
  case (6'shc) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h21 initial $display("@Rd23_21 hit"); end default: begin : d21 initial $display("@Rd23_21 def"); end endcase
  case (9'hc) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h22 initial $display("@Rd23_22 hit"); end default: begin : d22 initial $display("@Rd23_22 def"); end endcase
  case (64'hc) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h23 initial $display("@Rd23_23 hit"); end default: begin : d23 initial $display("@Rd23_23 def"); end endcase
  case (12) s6_t'(((33 ? L64 : P65) ^ signed'(I))): begin : h24 initial $display("@Rd23_24 hit"); end default: begin : d24 initial $display("@Rd23_24 def"); end endcase
endmodule
