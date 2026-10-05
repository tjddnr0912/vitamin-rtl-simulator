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
  case (14'h783) {P8, 3'(W6'(3'h3))}: begin : h0 initial $display("@Rd5_0 hit"); end default: begin : d0 initial $display("@Rd5_0 def"); end endcase
  case (64'hffffffffffffff83) {P8, 3'(W6'(3'h3))}: begin : h1 initial $display("@Rd5_1 hit"); end default: begin : d1 initial $display("@Rd5_1 def"); end endcase
  case ((-125)) {P8, 3'(W6'(3'h3))}: begin : h2 initial $display("@Rd5_2 hit"); end default: begin : d2 initial $display("@Rd5_2 def"); end endcase
  case (1923) {P8, 3'(W6'(3'h3))}: begin : h3 initial $display("@Rd5_3 hit"); end default: begin : d3 initial $display("@Rd5_3 def"); end endcase
  case (1'h1) (65'hea28d7de02c47d31 || 4): begin : h4 initial $display("@Rd5_4 hit"); end default: begin : d4 initial $display("@Rd5_4 def"); end endcase
  case (1'sh1) (65'hea28d7de02c47d31 || 4): begin : h5 initial $display("@Rd5_5 hit"); end default: begin : d5 initial $display("@Rd5_5 def"); end endcase
  case (4'h1) (65'hea28d7de02c47d31 || 4): begin : h6 initial $display("@Rd5_6 hit"); end default: begin : d6 initial $display("@Rd5_6 def"); end endcase
  case (64'hffffffffffffffff) (65'hea28d7de02c47d31 || 4): begin : h7 initial $display("@Rd5_7 hit"); end default: begin : d7 initial $display("@Rd5_7 def"); end endcase
  case ((-1)) (65'hea28d7de02c47d31 || 4): begin : h8 initial $display("@Rd5_8 hit"); end default: begin : d8 initial $display("@Rd5_8 def"); end endcase
  case (1) (65'hea28d7de02c47d31 || 4): begin : h9 initial $display("@Rd5_9 hit"); end default: begin : d9 initial $display("@Rd5_9 def"); end endcase
  case (31'h24fc030) (31'sh5024fc03 << 4): begin : h10 initial $display("@Rd5_10 hit"); end default: begin : d10 initial $display("@Rd5_10 def"); end endcase
  case (31'sh24fc030) (31'sh5024fc03 << 4): begin : h11 initial $display("@Rd5_11 hit"); end default: begin : d11 initial $display("@Rd5_11 def"); end endcase
  case (34'h24fc030) (31'sh5024fc03 << 4): begin : h12 initial $display("@Rd5_12 hit"); end default: begin : d12 initial $display("@Rd5_12 def"); end endcase
  case (64'h24fc030) (31'sh5024fc03 << 4): begin : h13 initial $display("@Rd5_13 hit"); end default: begin : d13 initial $display("@Rd5_13 def"); end endcase
  case (38780976) (31'sh5024fc03 << 4): begin : h14 initial $display("@Rd5_14 hit"); end default: begin : d14 initial $display("@Rd5_14 def"); end endcase
  case (38780976) (31'sh5024fc03 << 4): begin : h15 initial $display("@Rd5_15 hit"); end default: begin : d15 initial $display("@Rd5_15 def"); end endcase
  case (1'h0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h16 initial $display("@Rd5_16 hit"); end default: begin : d16 initial $display("@Rd5_16 def"); end endcase
  case (1'sh0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h17 initial $display("@Rd5_17 hit"); end default: begin : d17 initial $display("@Rd5_17 def"); end endcase
  case (4'h0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h18 initial $display("@Rd5_18 hit"); end default: begin : d18 initial $display("@Rd5_18 def"); end endcase
  case (64'h0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h19 initial $display("@Rd5_19 hit"); end default: begin : d19 initial $display("@Rd5_19 def"); end endcase
  case (0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h20 initial $display("@Rd5_20 hit"); end default: begin : d20 initial $display("@Rd5_20 def"); end endcase
  case (0) (&$onehot0(4'(33'h14c8c2fe9))): begin : h21 initial $display("@Rd5_21 hit"); end default: begin : d21 initial $display("@Rd5_21 def"); end endcase
  case (1'h1) (unsigned'(L64) && (-63'sh5c81140cc2d495e2)): begin : h22 initial $display("@Rd5_22 hit"); end default: begin : d22 initial $display("@Rd5_22 def"); end endcase
  case (1'sh1) (unsigned'(L64) && (-63'sh5c81140cc2d495e2)): begin : h23 initial $display("@Rd5_23 hit"); end default: begin : d23 initial $display("@Rd5_23 def"); end endcase
  case (4'h1) (unsigned'(L64) && (-63'sh5c81140cc2d495e2)): begin : h24 initial $display("@Rd5_24 hit"); end default: begin : d24 initial $display("@Rd5_24 def"); end endcase
endmodule
