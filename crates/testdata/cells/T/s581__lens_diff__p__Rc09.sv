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
  case (11'h6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h0 initial $display("@Rc9_0 hit"); end default: begin : d0 initial $display("@Rc9_0 def"); end endcase
  case (11'sh6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h1 initial $display("@Rc9_1 hit"); end default: begin : d1 initial $display("@Rc9_1 def"); end endcase
  case (14'h6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h2 initial $display("@Rc9_2 hit"); end default: begin : d2 initial $display("@Rc9_2 def"); end endcase
  case (64'h6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h3 initial $display("@Rc9_3 hit"); end default: begin : d3 initial $display("@Rc9_3 def"); end endcase
  case (6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h4 initial $display("@Rc9_4 hit"); end default: begin : d4 initial $display("@Rc9_4 def"); end endcase
  case (6) {8'(4'sh0), 3'(31'sh14f4f796)}: begin : h5 initial $display("@Rc9_5 hit"); end default: begin : d5 initial $display("@Rc9_5 def"); end endcase
  case (5'he) $unsigned((5'h1d >> 1)): begin : h6 initial $display("@Rc9_6 hit"); end default: begin : d6 initial $display("@Rc9_6 def"); end endcase
  case (5'she) $unsigned((5'h1d >> 1)): begin : h7 initial $display("@Rc9_7 hit"); end default: begin : d7 initial $display("@Rc9_7 def"); end endcase
  case (8'he) $unsigned((5'h1d >> 1)): begin : h8 initial $display("@Rc9_8 hit"); end default: begin : d8 initial $display("@Rc9_8 def"); end endcase
  case (64'he) $unsigned((5'h1d >> 1)): begin : h9 initial $display("@Rc9_9 hit"); end default: begin : d9 initial $display("@Rc9_9 def"); end endcase
  case (14) $unsigned((5'h1d >> 1)): begin : h10 initial $display("@Rc9_10 hit"); end default: begin : d10 initial $display("@Rc9_10 def"); end endcase
  case (14) $unsigned((5'h1d >> 1)): begin : h11 initial $display("@Rc9_11 hit"); end default: begin : d11 initial $display("@Rc9_11 def"); end endcase
  case (64'h6b8c0991496b123a) 64'sh6b8c0991496b123a: begin : h12 initial $display("@Rc9_12 hit"); end default: begin : d12 initial $display("@Rc9_12 def"); end endcase
  case (64'sh6b8c0991496b123a) 64'sh6b8c0991496b123a: begin : h13 initial $display("@Rc9_13 hit"); end default: begin : d13 initial $display("@Rc9_13 def"); end endcase
  case (64'h6b8c0991496b123a) 64'sh6b8c0991496b123a: begin : h14 initial $display("@Rc9_14 hit"); end default: begin : d14 initial $display("@Rc9_14 def"); end endcase
  case (64'h6b8c0991496b123a) 64'sh6b8c0991496b123a: begin : h15 initial $display("@Rc9_15 hit"); end default: begin : d15 initial $display("@Rc9_15 def"); end endcase
  case (6'h5) s6_t'(P65): begin : h16 initial $display("@Rc9_16 hit"); end default: begin : d16 initial $display("@Rc9_16 def"); end endcase
  case (6'sh5) s6_t'(P65): begin : h17 initial $display("@Rc9_17 hit"); end default: begin : d17 initial $display("@Rc9_17 def"); end endcase
  case (9'h5) s6_t'(P65): begin : h18 initial $display("@Rc9_18 hit"); end default: begin : d18 initial $display("@Rc9_18 def"); end endcase
  case (64'h5) s6_t'(P65): begin : h19 initial $display("@Rc9_19 hit"); end default: begin : d19 initial $display("@Rc9_19 def"); end endcase
  case (5) s6_t'(P65): begin : h20 initial $display("@Rc9_20 hit"); end default: begin : d20 initial $display("@Rc9_20 def"); end endcase
  case (5) s6_t'(P65): begin : h21 initial $display("@Rc9_21 hit"); end default: begin : d21 initial $display("@Rc9_21 def"); end endcase
  case (1'h0) (!(-7)): begin : h22 initial $display("@Rc9_22 hit"); end default: begin : d22 initial $display("@Rc9_22 def"); end endcase
  case (1'sh0) (!(-7)): begin : h23 initial $display("@Rc9_23 hit"); end default: begin : d23 initial $display("@Rc9_23 def"); end endcase
  case (4'h0) (!(-7)): begin : h24 initial $display("@Rc9_24 hit"); end default: begin : d24 initial $display("@Rc9_24 def"); end endcase
endmodule
