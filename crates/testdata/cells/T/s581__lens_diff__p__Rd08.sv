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
  case (35'h6) W6: begin : h0 initial $display("@Rd8_0 hit"); end default: begin : d0 initial $display("@Rd8_0 def"); end endcase
  case (64'h6) W6: begin : h1 initial $display("@Rd8_1 hit"); end default: begin : d1 initial $display("@Rd8_1 def"); end endcase
  case (6) W6: begin : h2 initial $display("@Rd8_2 hit"); end default: begin : d2 initial $display("@Rd8_2 def"); end endcase
  case (6) W6: begin : h3 initial $display("@Rd8_3 hit"); end default: begin : d3 initial $display("@Rd8_3 def"); end endcase
  case (1'h0) (27 == 2'sh3): begin : h4 initial $display("@Rd8_4 hit"); end default: begin : d4 initial $display("@Rd8_4 def"); end endcase
  case (1'sh0) (27 == 2'sh3): begin : h5 initial $display("@Rd8_5 hit"); end default: begin : d5 initial $display("@Rd8_5 def"); end endcase
  case (4'h0) (27 == 2'sh3): begin : h6 initial $display("@Rd8_6 hit"); end default: begin : d6 initial $display("@Rd8_6 def"); end endcase
  case (64'h0) (27 == 2'sh3): begin : h7 initial $display("@Rd8_7 hit"); end default: begin : d7 initial $display("@Rd8_7 def"); end endcase
  case (0) (27 == 2'sh3): begin : h8 initial $display("@Rd8_8 hit"); end default: begin : d8 initial $display("@Rd8_8 def"); end endcase
  case (0) (27 == 2'sh3): begin : h9 initial $display("@Rd8_9 hit"); end default: begin : d9 initial $display("@Rd8_9 def"); end endcase
  case (32'he6412a52) 32'he6412a52: begin : h10 initial $display("@Rd8_10 hit"); end default: begin : d10 initial $display("@Rd8_10 def"); end endcase
  case (32'she6412a52) 32'he6412a52: begin : h11 initial $display("@Rd8_11 hit"); end default: begin : d11 initial $display("@Rd8_11 def"); end endcase
  case (35'he6412a52) 32'he6412a52: begin : h12 initial $display("@Rd8_12 hit"); end default: begin : d12 initial $display("@Rd8_12 def"); end endcase
  case (64'hffffffffe6412a52) 32'he6412a52: begin : h13 initial $display("@Rd8_13 hit"); end default: begin : d13 initial $display("@Rd8_13 def"); end endcase
  case ((-431936942)) 32'he6412a52: begin : h14 initial $display("@Rd8_14 hit"); end default: begin : d14 initial $display("@Rd8_14 def"); end endcase
  case (32'h2) $countones(33'h1_0000_0001): begin : h15 initial $display("@Rd8_15 hit"); end default: begin : d15 initial $display("@Rd8_15 def"); end endcase
  case (32'sh2) $countones(33'h1_0000_0001): begin : h16 initial $display("@Rd8_16 hit"); end default: begin : d16 initial $display("@Rd8_16 def"); end endcase
  case (35'h2) $countones(33'h1_0000_0001): begin : h17 initial $display("@Rd8_17 hit"); end default: begin : d17 initial $display("@Rd8_17 def"); end endcase
  case (64'h2) $countones(33'h1_0000_0001): begin : h18 initial $display("@Rd8_18 hit"); end default: begin : d18 initial $display("@Rd8_18 def"); end endcase
  case (2) $countones(33'h1_0000_0001): begin : h19 initial $display("@Rd8_19 hit"); end default: begin : d19 initial $display("@Rd8_19 def"); end endcase
  case (2) $countones(33'h1_0000_0001): begin : h20 initial $display("@Rd8_20 hit"); end default: begin : d20 initial $display("@Rd8_20 def"); end endcase
  case (16'h9c9c) {2{8'(S8)}}: begin : h21 initial $display("@Rd8_21 hit"); end default: begin : d21 initial $display("@Rd8_21 def"); end endcase
  case (16'sh9c9c) {2{8'(S8)}}: begin : h22 initial $display("@Rd8_22 hit"); end default: begin : d22 initial $display("@Rd8_22 def"); end endcase
  case (19'h9c9c) {2{8'(S8)}}: begin : h23 initial $display("@Rd8_23 hit"); end default: begin : d23 initial $display("@Rd8_23 def"); end endcase
  case (64'hffffffffffff9c9c) {2{8'(S8)}}: begin : h24 initial $display("@Rd8_24 hit"); end default: begin : d24 initial $display("@Rd8_24 def"); end endcase
endmodule
