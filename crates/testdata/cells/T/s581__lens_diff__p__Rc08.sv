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
  case (12) P4: begin : h0 initial $display("@Rc8_0 hit"); end default: begin : d0 initial $display("@Rc8_0 def"); end endcase
  case (1'h0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h1 initial $display("@Rc8_1 hit"); end default: begin : d1 initial $display("@Rc8_1 def"); end endcase
  case (1'sh0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h2 initial $display("@Rc8_2 hit"); end default: begin : d2 initial $display("@Rc8_2 def"); end endcase
  case (4'h0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h3 initial $display("@Rc8_3 hit"); end default: begin : d3 initial $display("@Rc8_3 def"); end endcase
  case (64'h0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h4 initial $display("@Rc8_4 hit"); end default: begin : d4 initial $display("@Rc8_4 def"); end endcase
  case (0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h5 initial $display("@Rc8_5 hit"); end default: begin : d5 initial $display("@Rc8_5 def"); end endcase
  case (0) ((32'h98ccf91e ? S8 : L64) == {2{1'(32'h432c9729)}}): begin : h6 initial $display("@Rc8_6 hit"); end default: begin : d6 initial $display("@Rc8_6 def"); end endcase
  case (1'h0) $onehot(P8): begin : h7 initial $display("@Rc8_7 hit"); end default: begin : d7 initial $display("@Rc8_7 def"); end endcase
  case (1'sh0) $onehot(P8): begin : h8 initial $display("@Rc8_8 hit"); end default: begin : d8 initial $display("@Rc8_8 def"); end endcase
  case (4'h0) $onehot(P8): begin : h9 initial $display("@Rc8_9 hit"); end default: begin : d9 initial $display("@Rc8_9 def"); end endcase
  case (64'h0) $onehot(P8): begin : h10 initial $display("@Rc8_10 hit"); end default: begin : d10 initial $display("@Rc8_10 def"); end endcase
  case (0) $onehot(P8): begin : h11 initial $display("@Rc8_11 hit"); end default: begin : d11 initial $display("@Rc8_11 def"); end endcase
  case (0) $onehot(P8): begin : h12 initial $display("@Rc8_12 hit"); end default: begin : d12 initial $display("@Rc8_12 def"); end endcase
  case (1'h0) (|(I < P4)): begin : h13 initial $display("@Rc8_13 hit"); end default: begin : d13 initial $display("@Rc8_13 def"); end endcase
  case (1'sh0) (|(I < P4)): begin : h14 initial $display("@Rc8_14 hit"); end default: begin : d14 initial $display("@Rc8_14 def"); end endcase
  case (4'h0) (|(I < P4)): begin : h15 initial $display("@Rc8_15 hit"); end default: begin : d15 initial $display("@Rc8_15 def"); end endcase
  case (64'h0) (|(I < P4)): begin : h16 initial $display("@Rc8_16 hit"); end default: begin : d16 initial $display("@Rc8_16 def"); end endcase
  case (0) (|(I < P4)): begin : h17 initial $display("@Rc8_17 hit"); end default: begin : d17 initial $display("@Rc8_17 def"); end endcase
  case (0) (|(I < P4)): begin : h18 initial $display("@Rc8_18 hit"); end default: begin : d18 initial $display("@Rc8_18 def"); end endcase
  case (1'h0) $onehot0(33'({8'sh9C, P4})): begin : h19 initial $display("@Rc8_19 hit"); end default: begin : d19 initial $display("@Rc8_19 def"); end endcase
  case (1'sh0) $onehot0(33'({8'sh9C, P4})): begin : h20 initial $display("@Rc8_20 hit"); end default: begin : d20 initial $display("@Rc8_20 def"); end endcase
  case (4'h0) $onehot0(33'({8'sh9C, P4})): begin : h21 initial $display("@Rc8_21 hit"); end default: begin : d21 initial $display("@Rc8_21 def"); end endcase
  case (64'h0) $onehot0(33'({8'sh9C, P4})): begin : h22 initial $display("@Rc8_22 hit"); end default: begin : d22 initial $display("@Rc8_22 def"); end endcase
  case (0) $onehot0(33'({8'sh9C, P4})): begin : h23 initial $display("@Rc8_23 hit"); end default: begin : d23 initial $display("@Rc8_23 def"); end endcase
  case (0) $onehot0(33'({8'sh9C, P4})): begin : h24 initial $display("@Rc8_24 hit"); end default: begin : d24 initial $display("@Rc8_24 def"); end endcase
endmodule
