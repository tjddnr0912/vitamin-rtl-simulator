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
  case (32'sh7acf04cb) $unsigned((8'h38 ? 32'h7acf04cb : 5'sh18)): begin : h0 initial $display("@Rd1_0 hit"); end default: begin : d0 initial $display("@Rd1_0 def"); end endcase
  case (35'h7acf04cb) $unsigned((8'h38 ? 32'h7acf04cb : 5'sh18)): begin : h1 initial $display("@Rd1_1 hit"); end default: begin : d1 initial $display("@Rd1_1 def"); end endcase
  case (64'h7acf04cb) $unsigned((8'h38 ? 32'h7acf04cb : 5'sh18)): begin : h2 initial $display("@Rd1_2 hit"); end default: begin : d2 initial $display("@Rd1_2 def"); end endcase
  case (2060387531) $unsigned((8'h38 ? 32'h7acf04cb : 5'sh18)): begin : h3 initial $display("@Rd1_3 hit"); end default: begin : d3 initial $display("@Rd1_3 def"); end endcase
  case (2060387531) $unsigned((8'h38 ? 32'h7acf04cb : 5'sh18)): begin : h4 initial $display("@Rd1_4 hit"); end default: begin : d4 initial $display("@Rd1_4 def"); end endcase
  case (32'h0) $clog2(1'h0): begin : h5 initial $display("@Rd1_5 hit"); end default: begin : d5 initial $display("@Rd1_5 def"); end endcase
  case (32'sh0) $clog2(1'h0): begin : h6 initial $display("@Rd1_6 hit"); end default: begin : d6 initial $display("@Rd1_6 def"); end endcase
  case (35'h0) $clog2(1'h0): begin : h7 initial $display("@Rd1_7 hit"); end default: begin : d7 initial $display("@Rd1_7 def"); end endcase
  case (64'h0) $clog2(1'h0): begin : h8 initial $display("@Rd1_8 hit"); end default: begin : d8 initial $display("@Rd1_8 def"); end endcase
  case (0) $clog2(1'h0): begin : h9 initial $display("@Rd1_9 hit"); end default: begin : d9 initial $display("@Rd1_9 def"); end endcase
  case (0) $clog2(1'h0): begin : h10 initial $display("@Rd1_10 hit"); end default: begin : d10 initial $display("@Rd1_10 def"); end endcase
  case (36'hfffffffeb) {33'($signed(S4)), 3'({S4, 33'((-5))})}: begin : h11 initial $display("@Rd1_11 hit"); end default: begin : d11 initial $display("@Rd1_11 def"); end endcase
  case (36'shfffffffeb) {33'($signed(S4)), 3'({S4, 33'((-5))})}: begin : h12 initial $display("@Rd1_12 hit"); end default: begin : d12 initial $display("@Rd1_12 def"); end endcase
  case (39'hfffffffeb) {33'($signed(S4)), 3'({S4, 33'((-5))})}: begin : h13 initial $display("@Rd1_13 hit"); end default: begin : d13 initial $display("@Rd1_13 def"); end endcase
  case (64'hffffffffffffffeb) {33'($signed(S4)), 3'({S4, 33'((-5))})}: begin : h14 initial $display("@Rd1_14 hit"); end default: begin : d14 initial $display("@Rd1_14 def"); end endcase
  case ((-21)) {33'($signed(S4)), 3'({S4, 33'((-5))})}: begin : h15 initial $display("@Rd1_15 hit"); end default: begin : d15 initial $display("@Rd1_15 def"); end endcase
  case (64'h51) (L64 ** 2): begin : h16 initial $display("@Rd1_16 hit"); end default: begin : d16 initial $display("@Rd1_16 def"); end endcase
  case (64'sh51) (L64 ** 2): begin : h17 initial $display("@Rd1_17 hit"); end default: begin : d17 initial $display("@Rd1_17 def"); end endcase
  case (64'h51) (L64 ** 2): begin : h18 initial $display("@Rd1_18 hit"); end default: begin : d18 initial $display("@Rd1_18 def"); end endcase
  case (64'h51) (L64 ** 2): begin : h19 initial $display("@Rd1_19 hit"); end default: begin : d19 initial $display("@Rd1_19 def"); end endcase
  case (81) (L64 ** 2): begin : h20 initial $display("@Rd1_20 hit"); end default: begin : d20 initial $display("@Rd1_20 def"); end endcase
  case (81) (L64 ** 2): begin : h21 initial $display("@Rd1_21 hit"); end default: begin : d21 initial $display("@Rd1_21 def"); end endcase
  case (12'hc9c) {4'(S8), 8'sh9C}: begin : h22 initial $display("@Rd1_22 hit"); end default: begin : d22 initial $display("@Rd1_22 def"); end endcase
  case (12'shc9c) {4'(S8), 8'sh9C}: begin : h23 initial $display("@Rd1_23 hit"); end default: begin : d23 initial $display("@Rd1_23 def"); end endcase
  case (15'hc9c) {4'(S8), 8'sh9C}: begin : h24 initial $display("@Rd1_24 hit"); end default: begin : d24 initial $display("@Rd1_24 def"); end endcase
endmodule
