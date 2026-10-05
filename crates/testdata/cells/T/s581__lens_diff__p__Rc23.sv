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
  case (12'hd9c) {S4, 8'sh9C}: begin : h0 initial $display("@Rc23_0 hit"); end default: begin : d0 initial $display("@Rc23_0 def"); end endcase
  case (12'shd9c) {S4, 8'sh9C}: begin : h1 initial $display("@Rc23_1 hit"); end default: begin : d1 initial $display("@Rc23_1 def"); end endcase
  case (15'hd9c) {S4, 8'sh9C}: begin : h2 initial $display("@Rc23_2 hit"); end default: begin : d2 initial $display("@Rc23_2 def"); end endcase
  case (64'hfffffffffffffd9c) {S4, 8'sh9C}: begin : h3 initial $display("@Rc23_3 hit"); end default: begin : d3 initial $display("@Rc23_3 def"); end endcase
  case ((-612)) {S4, 8'sh9C}: begin : h4 initial $display("@Rc23_4 hit"); end default: begin : d4 initial $display("@Rc23_4 def"); end endcase
  case (3484) {S4, 8'sh9C}: begin : h5 initial $display("@Rc23_5 hit"); end default: begin : d5 initial $display("@Rc23_5 def"); end endcase
  case (1'h1) $signed(1'h1): begin : h6 initial $display("@Rc23_6 hit"); end default: begin : d6 initial $display("@Rc23_6 def"); end endcase
  case (1'sh1) $signed(1'h1): begin : h7 initial $display("@Rc23_7 hit"); end default: begin : d7 initial $display("@Rc23_7 def"); end endcase
  case (4'h1) $signed(1'h1): begin : h8 initial $display("@Rc23_8 hit"); end default: begin : d8 initial $display("@Rc23_8 def"); end endcase
  case (64'hffffffffffffffff) $signed(1'h1): begin : h9 initial $display("@Rc23_9 hit"); end default: begin : d9 initial $display("@Rc23_9 def"); end endcase
  case ((-1)) $signed(1'h1): begin : h10 initial $display("@Rc23_10 hit"); end default: begin : d10 initial $display("@Rc23_10 def"); end endcase
  case (1) $signed(1'h1): begin : h11 initial $display("@Rc23_11 hit"); end default: begin : d11 initial $display("@Rc23_11 def"); end endcase
  case (3'h5) 3'((7 * 63'h2eeb8d464194f00b)): begin : h12 initial $display("@Rc23_12 hit"); end default: begin : d12 initial $display("@Rc23_12 def"); end endcase
  case (3'sh5) 3'((7 * 63'h2eeb8d464194f00b)): begin : h13 initial $display("@Rc23_13 hit"); end default: begin : d13 initial $display("@Rc23_13 def"); end endcase
  case (6'h5) 3'((7 * 63'h2eeb8d464194f00b)): begin : h14 initial $display("@Rc23_14 hit"); end default: begin : d14 initial $display("@Rc23_14 def"); end endcase
  case (64'hfffffffffffffffd) 3'((7 * 63'h2eeb8d464194f00b)): begin : h15 initial $display("@Rc23_15 hit"); end default: begin : d15 initial $display("@Rc23_15 def"); end endcase
  case ((-3)) 3'((7 * 63'h2eeb8d464194f00b)): begin : h16 initial $display("@Rc23_16 hit"); end default: begin : d16 initial $display("@Rc23_16 def"); end endcase
  case (5) 3'((7 * 63'h2eeb8d464194f00b)): begin : h17 initial $display("@Rc23_17 hit"); end default: begin : d17 initial $display("@Rc23_17 def"); end endcase
  case (1'h1) $onehot0(1'(S8)): begin : h18 initial $display("@Rc23_18 hit"); end default: begin : d18 initial $display("@Rc23_18 def"); end endcase
  case (1'sh1) $onehot0(1'(S8)): begin : h19 initial $display("@Rc23_19 hit"); end default: begin : d19 initial $display("@Rc23_19 def"); end endcase
  case (4'h1) $onehot0(1'(S8)): begin : h20 initial $display("@Rc23_20 hit"); end default: begin : d20 initial $display("@Rc23_20 def"); end endcase
  case (64'hffffffffffffffff) $onehot0(1'(S8)): begin : h21 initial $display("@Rc23_21 hit"); end default: begin : d21 initial $display("@Rc23_21 def"); end endcase
  case ((-1)) $onehot0(1'(S8)): begin : h22 initial $display("@Rc23_22 hit"); end default: begin : d22 initial $display("@Rc23_22 def"); end endcase
  case (1) $onehot0(1'(S8)): begin : h23 initial $display("@Rc23_23 hit"); end default: begin : d23 initial $display("@Rc23_23 def"); end endcase
  case (6'h21) (((-P8) || (18 % W6)) | ((16'sh38f / 63'h268581aab552dd15) ? $signed(UN) : W6'(16'shf9e1))): begin : h24 initial $display("@Rc23_24 hit"); end default: begin : d24 initial $display("@Rc23_24 def"); end endcase
endmodule
