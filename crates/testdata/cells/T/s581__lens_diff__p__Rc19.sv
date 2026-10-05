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
  case (0) (-((16'sh4732 & S4) >> (65'h4d700de4b96ac1c2 << 0))): begin : h0 initial $display("@Rc19_0 hit"); end default: begin : d0 initial $display("@Rc19_0 def"); end endcase
  case (1'h0) (P65 && $signed(1'sh0)): begin : h1 initial $display("@Rc19_1 hit"); end default: begin : d1 initial $display("@Rc19_1 def"); end endcase
  case (1'sh0) (P65 && $signed(1'sh0)): begin : h2 initial $display("@Rc19_2 hit"); end default: begin : d2 initial $display("@Rc19_2 def"); end endcase
  case (4'h0) (P65 && $signed(1'sh0)): begin : h3 initial $display("@Rc19_3 hit"); end default: begin : d3 initial $display("@Rc19_3 def"); end endcase
  case (64'h0) (P65 && $signed(1'sh0)): begin : h4 initial $display("@Rc19_4 hit"); end default: begin : d4 initial $display("@Rc19_4 def"); end endcase
  case (0) (P65 && $signed(1'sh0)): begin : h5 initial $display("@Rc19_5 hit"); end default: begin : d5 initial $display("@Rc19_5 def"); end endcase
  case (0) (P65 && $signed(1'sh0)): begin : h6 initial $display("@Rc19_6 hit"); end default: begin : d6 initial $display("@Rc19_6 def"); end endcase
  case (32'h0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h7 initial $display("@Rc19_7 hit"); end default: begin : d7 initial $display("@Rc19_7 def"); end endcase
  case (32'sh0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h8 initial $display("@Rc19_8 hit"); end default: begin : d8 initial $display("@Rc19_8 def"); end endcase
  case (35'h0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h9 initial $display("@Rc19_9 hit"); end default: begin : d9 initial $display("@Rc19_9 def"); end endcase
  case (64'h0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h10 initial $display("@Rc19_10 hit"); end default: begin : d10 initial $display("@Rc19_10 def"); end endcase
  case (0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h11 initial $display("@Rc19_11 hit"); end default: begin : d11 initial $display("@Rc19_11 def"); end endcase
  case (0) ($countones(S65) >>> (16'h2295 >> 5)): begin : h12 initial $display("@Rc19_12 hit"); end default: begin : d12 initial $display("@Rc19_12 def"); end endcase
  case (1'h1) (~&((W6 >= 16'h8d07) || (!P8))): begin : h13 initial $display("@Rc19_13 hit"); end default: begin : d13 initial $display("@Rc19_13 def"); end endcase
  case (1'sh1) (~&((W6 >= 16'h8d07) || (!P8))): begin : h14 initial $display("@Rc19_14 hit"); end default: begin : d14 initial $display("@Rc19_14 def"); end endcase
  case (4'h1) (~&((W6 >= 16'h8d07) || (!P8))): begin : h15 initial $display("@Rc19_15 hit"); end default: begin : d15 initial $display("@Rc19_15 def"); end endcase
  case (64'hffffffffffffffff) (~&((W6 >= 16'h8d07) || (!P8))): begin : h16 initial $display("@Rc19_16 hit"); end default: begin : d16 initial $display("@Rc19_16 def"); end endcase
  case ((-1)) (~&((W6 >= 16'h8d07) || (!P8))): begin : h17 initial $display("@Rc19_17 hit"); end default: begin : d17 initial $display("@Rc19_17 def"); end endcase
  case (1) (~&((W6 >= 16'h8d07) || (!P8))): begin : h18 initial $display("@Rc19_18 hit"); end default: begin : d18 initial $display("@Rc19_18 def"); end endcase
  case (1'h0) $onehot0(8'({2{P8}})): begin : h19 initial $display("@Rc19_19 hit"); end default: begin : d19 initial $display("@Rc19_19 def"); end endcase
  case (1'sh0) $onehot0(8'({2{P8}})): begin : h20 initial $display("@Rc19_20 hit"); end default: begin : d20 initial $display("@Rc19_20 def"); end endcase
  case (4'h0) $onehot0(8'({2{P8}})): begin : h21 initial $display("@Rc19_21 hit"); end default: begin : d21 initial $display("@Rc19_21 def"); end endcase
  case (64'h0) $onehot0(8'({2{P8}})): begin : h22 initial $display("@Rc19_22 hit"); end default: begin : d22 initial $display("@Rc19_22 def"); end endcase
  case (0) $onehot0(8'({2{P8}})): begin : h23 initial $display("@Rc19_23 hit"); end default: begin : d23 initial $display("@Rc19_23 def"); end endcase
  case (0) $onehot0(8'({2{P8}})): begin : h24 initial $display("@Rc19_24 hit"); end default: begin : d24 initial $display("@Rc19_24 def"); end endcase
endmodule
