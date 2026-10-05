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
  case (4'h1) ((|8'sh58) ^ (~|S8)): begin : h0 initial $display("@Rc14_0 hit"); end default: begin : d0 initial $display("@Rc14_0 def"); end endcase
  case (64'hffffffffffffffff) ((|8'sh58) ^ (~|S8)): begin : h1 initial $display("@Rc14_1 hit"); end default: begin : d1 initial $display("@Rc14_1 def"); end endcase
  case ((-1)) ((|8'sh58) ^ (~|S8)): begin : h2 initial $display("@Rc14_2 hit"); end default: begin : d2 initial $display("@Rc14_2 def"); end endcase
  case (1) ((|8'sh58) ^ (~|S8)): begin : h3 initial $display("@Rc14_3 hit"); end default: begin : d3 initial $display("@Rc14_3 def"); end endcase
  case (32'hb) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h4 initial $display("@Rc14_4 hit"); end default: begin : d4 initial $display("@Rc14_4 def"); end endcase
  case (32'shb) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h5 initial $display("@Rc14_5 hit"); end default: begin : d5 initial $display("@Rc14_5 def"); end endcase
  case (35'hb) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h6 initial $display("@Rc14_6 hit"); end default: begin : d6 initial $display("@Rc14_6 def"); end endcase
  case (64'hb) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h7 initial $display("@Rc14_7 hit"); end default: begin : d7 initial $display("@Rc14_7 def"); end endcase
  case (11) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h8 initial $display("@Rc14_8 hit"); end default: begin : d8 initial $display("@Rc14_8 def"); end endcase
  case (11) ({8'sh9C, P8} ? 11 : $unsigned(17)): begin : h9 initial $display("@Rc14_9 hit"); end default: begin : d9 initial $display("@Rc14_9 def"); end endcase
  case (1'h1) (~&(32'h837beac2 < 4'sha)): begin : h10 initial $display("@Rc14_10 hit"); end default: begin : d10 initial $display("@Rc14_10 def"); end endcase
  case (1'sh1) (~&(32'h837beac2 < 4'sha)): begin : h11 initial $display("@Rc14_11 hit"); end default: begin : d11 initial $display("@Rc14_11 def"); end endcase
  case (4'h1) (~&(32'h837beac2 < 4'sha)): begin : h12 initial $display("@Rc14_12 hit"); end default: begin : d12 initial $display("@Rc14_12 def"); end endcase
  case (64'hffffffffffffffff) (~&(32'h837beac2 < 4'sha)): begin : h13 initial $display("@Rc14_13 hit"); end default: begin : d13 initial $display("@Rc14_13 def"); end endcase
  case ((-1)) (~&(32'h837beac2 < 4'sha)): begin : h14 initial $display("@Rc14_14 hit"); end default: begin : d14 initial $display("@Rc14_14 def"); end endcase
  case (1) (~&(32'h837beac2 < 4'sha)): begin : h15 initial $display("@Rc14_15 hit"); end default: begin : d15 initial $display("@Rc14_15 def"); end endcase
  case (32'hfffffffd) ((-3) >> 0): begin : h16 initial $display("@Rc14_16 hit"); end default: begin : d16 initial $display("@Rc14_16 def"); end endcase
  case (32'shfffffffd) ((-3) >> 0): begin : h17 initial $display("@Rc14_17 hit"); end default: begin : d17 initial $display("@Rc14_17 def"); end endcase
  case (35'hfffffffd) ((-3) >> 0): begin : h18 initial $display("@Rc14_18 hit"); end default: begin : d18 initial $display("@Rc14_18 def"); end endcase
  case (64'hfffffffffffffffd) ((-3) >> 0): begin : h19 initial $display("@Rc14_19 hit"); end default: begin : d19 initial $display("@Rc14_19 def"); end endcase
  case ((-3)) ((-3) >> 0): begin : h20 initial $display("@Rc14_20 hit"); end default: begin : d20 initial $display("@Rc14_20 def"); end endcase
  case (6'h30) (s6_t'(U32) >> (L64 >= S4)): begin : h21 initial $display("@Rc14_21 hit"); end default: begin : d21 initial $display("@Rc14_21 def"); end endcase
  case (6'sh30) (s6_t'(U32) >> (L64 >= S4)): begin : h22 initial $display("@Rc14_22 hit"); end default: begin : d22 initial $display("@Rc14_22 def"); end endcase
  case (9'h30) (s6_t'(U32) >> (L64 >= S4)): begin : h23 initial $display("@Rc14_23 hit"); end default: begin : d23 initial $display("@Rc14_23 def"); end endcase
  case (64'hfffffffffffffff0) (s6_t'(U32) >> (L64 >= S4)): begin : h24 initial $display("@Rc14_24 hit"); end default: begin : d24 initial $display("@Rc14_24 def"); end endcase
endmodule
