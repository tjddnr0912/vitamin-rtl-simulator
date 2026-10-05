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
  case (0) (~|(-7)): begin : h0 initial $display("@Rd3_0 hit"); end default: begin : d0 initial $display("@Rd3_0 def"); end endcase
  case (0) (~|(-7)): begin : h1 initial $display("@Rd3_1 hit"); end default: begin : d1 initial $display("@Rd3_1 def"); end endcase
  case (4'hc) $signed(P4): begin : h2 initial $display("@Rd3_2 hit"); end default: begin : d2 initial $display("@Rd3_2 def"); end endcase
  case (4'shc) $signed(P4): begin : h3 initial $display("@Rd3_3 hit"); end default: begin : d3 initial $display("@Rd3_3 def"); end endcase
  case (7'hc) $signed(P4): begin : h4 initial $display("@Rd3_4 hit"); end default: begin : d4 initial $display("@Rd3_4 def"); end endcase
  case (64'hfffffffffffffffc) $signed(P4): begin : h5 initial $display("@Rd3_5 hit"); end default: begin : d5 initial $display("@Rd3_5 def"); end endcase
  case ((-4)) $signed(P4): begin : h6 initial $display("@Rd3_6 hit"); end default: begin : d6 initial $display("@Rd3_6 def"); end endcase
  case (12) $signed(P4): begin : h7 initial $display("@Rd3_7 hit"); end default: begin : d7 initial $display("@Rd3_7 def"); end endcase
  case (8'hcc) {2{P4}}: begin : h8 initial $display("@Rd3_8 hit"); end default: begin : d8 initial $display("@Rd3_8 def"); end endcase
  case (8'shcc) {2{P4}}: begin : h9 initial $display("@Rd3_9 hit"); end default: begin : d9 initial $display("@Rd3_9 def"); end endcase
  case (11'hcc) {2{P4}}: begin : h10 initial $display("@Rd3_10 hit"); end default: begin : d10 initial $display("@Rd3_10 def"); end endcase
  case (64'hffffffffffffffcc) {2{P4}}: begin : h11 initial $display("@Rd3_11 hit"); end default: begin : d11 initial $display("@Rd3_11 def"); end endcase
  case ((-52)) {2{P4}}: begin : h12 initial $display("@Rd3_12 hit"); end default: begin : d12 initial $display("@Rd3_12 def"); end endcase
  case (204) {2{P4}}: begin : h13 initial $display("@Rd3_13 hit"); end default: begin : d13 initial $display("@Rd3_13 def"); end endcase
  case (1'h0) $onehot0(P8): begin : h14 initial $display("@Rd3_14 hit"); end default: begin : d14 initial $display("@Rd3_14 def"); end endcase
  case (1'sh0) $onehot0(P8): begin : h15 initial $display("@Rd3_15 hit"); end default: begin : d15 initial $display("@Rd3_15 def"); end endcase
  case (4'h0) $onehot0(P8): begin : h16 initial $display("@Rd3_16 hit"); end default: begin : d16 initial $display("@Rd3_16 def"); end endcase
  case (64'h0) $onehot0(P8): begin : h17 initial $display("@Rd3_17 hit"); end default: begin : d17 initial $display("@Rd3_17 def"); end endcase
  case (0) $onehot0(P8): begin : h18 initial $display("@Rd3_18 hit"); end default: begin : d18 initial $display("@Rd3_18 def"); end endcase
  case (0) $onehot0(P8): begin : h19 initial $display("@Rd3_19 hit"); end default: begin : d19 initial $display("@Rd3_19 def"); end endcase
  case (1'h0) (&$unsigned((!4'sh8))): begin : h20 initial $display("@Rd3_20 hit"); end default: begin : d20 initial $display("@Rd3_20 def"); end endcase
  case (1'sh0) (&$unsigned((!4'sh8))): begin : h21 initial $display("@Rd3_21 hit"); end default: begin : d21 initial $display("@Rd3_21 def"); end endcase
  case (4'h0) (&$unsigned((!4'sh8))): begin : h22 initial $display("@Rd3_22 hit"); end default: begin : d22 initial $display("@Rd3_22 def"); end endcase
  case (64'h0) (&$unsigned((!4'sh8))): begin : h23 initial $display("@Rd3_23 hit"); end default: begin : d23 initial $display("@Rd3_23 def"); end endcase
  case (0) (&$unsigned((!4'sh8))): begin : h24 initial $display("@Rd3_24 hit"); end default: begin : d24 initial $display("@Rd3_24 def"); end endcase
endmodule
