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
  case (10) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h0 initial $display("@Rc21_0 hit"); end default: begin : d0 initial $display("@Rc21_0 def"); end endcase
  case (10) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h1 initial $display("@Rc21_1 hit"); end default: begin : d1 initial $display("@Rc21_1 def"); end endcase
  case (1'h0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h2 initial $display("@Rc21_2 hit"); end default: begin : d2 initial $display("@Rc21_2 def"); end endcase
  case (1'sh0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h3 initial $display("@Rc21_3 hit"); end default: begin : d3 initial $display("@Rc21_3 def"); end endcase
  case (4'h0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h4 initial $display("@Rc21_4 hit"); end default: begin : d4 initial $display("@Rc21_4 def"); end endcase
  case (64'h0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h5 initial $display("@Rc21_5 hit"); end default: begin : d5 initial $display("@Rc21_5 def"); end endcase
  case (0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h6 initial $display("@Rc21_6 hit"); end default: begin : d6 initial $display("@Rc21_6 def"); end endcase
  case (0) $onehot0(33'((3'sh3 ? 4'h6 : 63'h69e3615551238b69))): begin : h7 initial $display("@Rc21_7 hit"); end default: begin : d7 initial $display("@Rc21_7 def"); end endcase
  case (1'h0) $onehot(65'({S8, S4})): begin : h8 initial $display("@Rc21_8 hit"); end default: begin : d8 initial $display("@Rc21_8 def"); end endcase
  case (1'sh0) $onehot(65'({S8, S4})): begin : h9 initial $display("@Rc21_9 hit"); end default: begin : d9 initial $display("@Rc21_9 def"); end endcase
  case (4'h0) $onehot(65'({S8, S4})): begin : h10 initial $display("@Rc21_10 hit"); end default: begin : d10 initial $display("@Rc21_10 def"); end endcase
  case (64'h0) $onehot(65'({S8, S4})): begin : h11 initial $display("@Rc21_11 hit"); end default: begin : d11 initial $display("@Rc21_11 def"); end endcase
  case (0) $onehot(65'({S8, S4})): begin : h12 initial $display("@Rc21_12 hit"); end default: begin : d12 initial $display("@Rc21_12 def"); end endcase
  case (0) $onehot(65'({S8, S4})): begin : h13 initial $display("@Rc21_13 hit"); end default: begin : d13 initial $display("@Rc21_13 def"); end endcase
  case (63'h46608b75e3911aba) 63'sh46608b75e3911aba: begin : h14 initial $display("@Rc21_14 hit"); end default: begin : d14 initial $display("@Rc21_14 def"); end endcase
  case (63'sh46608b75e3911aba) 63'sh46608b75e3911aba: begin : h15 initial $display("@Rc21_15 hit"); end default: begin : d15 initial $display("@Rc21_15 def"); end endcase
  case (64'h46608b75e3911aba) 63'sh46608b75e3911aba: begin : h16 initial $display("@Rc21_16 hit"); end default: begin : d16 initial $display("@Rc21_16 def"); end endcase
  case (64'hc6608b75e3911aba) 63'sh46608b75e3911aba: begin : h17 initial $display("@Rc21_17 hit"); end default: begin : d17 initial $display("@Rc21_17 def"); end endcase
  case (1'h0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h18 initial $display("@Rc21_18 hit"); end default: begin : d18 initial $display("@Rc21_18 def"); end endcase
  case (1'sh0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h19 initial $display("@Rc21_19 hit"); end default: begin : d19 initial $display("@Rc21_19 def"); end endcase
  case (4'h0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h20 initial $display("@Rc21_20 hit"); end default: begin : d20 initial $display("@Rc21_20 def"); end endcase
  case (64'h0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h21 initial $display("@Rc21_21 hit"); end default: begin : d21 initial $display("@Rc21_21 def"); end endcase
  case (0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h22 initial $display("@Rc21_22 hit"); end default: begin : d22 initial $display("@Rc21_22 def"); end endcase
  case (0) (&($onehot0(33'h1_0000_0001) && (2'sh0 >> 2))): begin : h23 initial $display("@Rc21_23 hit"); end default: begin : d23 initial $display("@Rc21_23 def"); end endcase
  case (33'h1b6972a80) 33'h1b6972a80: begin : h24 initial $display("@Rc21_24 hit"); end default: begin : d24 initial $display("@Rc21_24 def"); end endcase
endmodule
