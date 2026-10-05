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
  case (6'h1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h0 initial $display("@Rc0_0 hit"); end default: begin : d0 initial $display("@Rc0_0 def"); end endcase
  case (6'sh1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h1 initial $display("@Rc0_1 hit"); end default: begin : d1 initial $display("@Rc0_1 def"); end endcase
  case (9'h1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h2 initial $display("@Rc0_2 hit"); end default: begin : d2 initial $display("@Rc0_2 def"); end endcase
  case (64'h1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h3 initial $display("@Rc0_3 hit"); end default: begin : d3 initial $display("@Rc0_3 def"); end endcase
  case (1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h4 initial $display("@Rc0_4 hit"); end default: begin : d4 initial $display("@Rc0_4 def"); end endcase
  case (1) s6_t'((|(P8 % 33'sh1522162b3))): begin : h5 initial $display("@Rc0_5 hit"); end default: begin : d5 initial $display("@Rc0_5 def"); end endcase
  case (32'h17) (P128 ? 23 : (-7)): begin : h6 initial $display("@Rc0_6 hit"); end default: begin : d6 initial $display("@Rc0_6 def"); end endcase
  case (32'sh17) (P128 ? 23 : (-7)): begin : h7 initial $display("@Rc0_7 hit"); end default: begin : d7 initial $display("@Rc0_7 def"); end endcase
  case (35'h17) (P128 ? 23 : (-7)): begin : h8 initial $display("@Rc0_8 hit"); end default: begin : d8 initial $display("@Rc0_8 def"); end endcase
  case (64'h17) (P128 ? 23 : (-7)): begin : h9 initial $display("@Rc0_9 hit"); end default: begin : d9 initial $display("@Rc0_9 def"); end endcase
  case (23) (P128 ? 23 : (-7)): begin : h10 initial $display("@Rc0_10 hit"); end default: begin : d10 initial $display("@Rc0_10 def"); end endcase
  case (23) (P128 ? 23 : (-7)): begin : h11 initial $display("@Rc0_11 hit"); end default: begin : d11 initial $display("@Rc0_11 def"); end endcase
  case (11'h4cc) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h12 initial $display("@Rc0_12 hit"); end default: begin : d12 initial $display("@Rc0_12 def"); end endcase
  case (11'sh4cc) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h13 initial $display("@Rc0_13 hit"); end default: begin : d13 initial $display("@Rc0_13 def"); end endcase
  case (14'h4cc) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h14 initial $display("@Rc0_14 hit"); end default: begin : d14 initial $display("@Rc0_14 def"); end endcase
  case (64'hfffffffffffffccc) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h15 initial $display("@Rc0_15 hit"); end default: begin : d15 initial $display("@Rc0_15 def"); end endcase
  case ((-820)) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h16 initial $display("@Rc0_16 hit"); end default: begin : d16 initial $display("@Rc0_16 def"); end endcase
  case (1228) {3'((31'h6d6f7b70 ? 4'h4 : 1'h1)), 8'({2{P4}})}: begin : h17 initial $display("@Rc0_17 hit"); end default: begin : d17 initial $display("@Rc0_17 def"); end endcase
  case (1'h0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h18 initial $display("@Rc0_18 hit"); end default: begin : d18 initial $display("@Rc0_18 def"); end endcase
  case (1'sh0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h19 initial $display("@Rc0_19 hit"); end default: begin : d19 initial $display("@Rc0_19 def"); end endcase
  case (4'h0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h20 initial $display("@Rc0_20 hit"); end default: begin : d20 initial $display("@Rc0_20 def"); end endcase
  case (64'h0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h21 initial $display("@Rc0_21 hit"); end default: begin : d21 initial $display("@Rc0_21 def"); end endcase
  case (0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h22 initial $display("@Rc0_22 hit"); end default: begin : d22 initial $display("@Rc0_22 def"); end endcase
  case (0) (~|$signed((8'hfc ? 8'h49 : 65'sh137a87c40e3f7a468))): begin : h23 initial $display("@Rc0_23 hit"); end default: begin : d23 initial $display("@Rc0_23 def"); end endcase
  case (1'h0) (!(P65 | 6)): begin : h24 initial $display("@Rc0_24 hit"); end default: begin : d24 initial $display("@Rc0_24 def"); end endcase
endmodule
