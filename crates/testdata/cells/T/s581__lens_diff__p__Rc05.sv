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
  case (41'h1bef) {33'(27), 8'(P128)}: begin : h0 initial $display("@Rc5_0 hit"); end default: begin : d0 initial $display("@Rc5_0 def"); end endcase
  case (41'sh1bef) {33'(27), 8'(P128)}: begin : h1 initial $display("@Rc5_1 hit"); end default: begin : d1 initial $display("@Rc5_1 def"); end endcase
  case (44'h1bef) {33'(27), 8'(P128)}: begin : h2 initial $display("@Rc5_2 hit"); end default: begin : d2 initial $display("@Rc5_2 def"); end endcase
  case (64'h1bef) {33'(27), 8'(P128)}: begin : h3 initial $display("@Rc5_3 hit"); end default: begin : d3 initial $display("@Rc5_3 def"); end endcase
  case (7151) {33'(27), 8'(P128)}: begin : h4 initial $display("@Rc5_4 hit"); end default: begin : d4 initial $display("@Rc5_4 def"); end endcase
  case (7151) {33'(27), 8'(P128)}: begin : h5 initial $display("@Rc5_5 hit"); end default: begin : d5 initial $display("@Rc5_5 def"); end endcase
  case (37'h100000001d) {33'h1_0000_0001, S4}: begin : h6 initial $display("@Rc5_6 hit"); end default: begin : d6 initial $display("@Rc5_6 def"); end endcase
  case (37'sh100000001d) {33'h1_0000_0001, S4}: begin : h7 initial $display("@Rc5_7 hit"); end default: begin : d7 initial $display("@Rc5_7 def"); end endcase
  case (40'h100000001d) {33'h1_0000_0001, S4}: begin : h8 initial $display("@Rc5_8 hit"); end default: begin : d8 initial $display("@Rc5_8 def"); end endcase
  case (64'hfffffff00000001d) {33'h1_0000_0001, S4}: begin : h9 initial $display("@Rc5_9 hit"); end default: begin : d9 initial $display("@Rc5_9 def"); end endcase
  case (32'h0) $clog2((I && (-9))): begin : h10 initial $display("@Rc5_10 hit"); end default: begin : d10 initial $display("@Rc5_10 def"); end endcase
  case (32'sh0) $clog2((I && (-9))): begin : h11 initial $display("@Rc5_11 hit"); end default: begin : d11 initial $display("@Rc5_11 def"); end endcase
  case (35'h0) $clog2((I && (-9))): begin : h12 initial $display("@Rc5_12 hit"); end default: begin : d12 initial $display("@Rc5_12 def"); end endcase
  case (64'h0) $clog2((I && (-9))): begin : h13 initial $display("@Rc5_13 hit"); end default: begin : d13 initial $display("@Rc5_13 def"); end endcase
  case (0) $clog2((I && (-9))): begin : h14 initial $display("@Rc5_14 hit"); end default: begin : d14 initial $display("@Rc5_14 def"); end endcase
  case (0) $clog2((I && (-9))): begin : h15 initial $display("@Rc5_15 hit"); end default: begin : d15 initial $display("@Rc5_15 def"); end endcase
  case (1'h0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h16 initial $display("@Rc5_16 hit"); end default: begin : d16 initial $display("@Rc5_16 def"); end endcase
  case (1'sh0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h17 initial $display("@Rc5_17 hit"); end default: begin : d17 initial $display("@Rc5_17 def"); end endcase
  case (4'h0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h18 initial $display("@Rc5_18 hit"); end default: begin : d18 initial $display("@Rc5_18 def"); end endcase
  case (64'h0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h19 initial $display("@Rc5_19 hit"); end default: begin : d19 initial $display("@Rc5_19 def"); end endcase
  case (0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h20 initial $display("@Rc5_20 hit"); end default: begin : d20 initial $display("@Rc5_20 def"); end endcase
  case (0) ((~&16'shefc7) >> (S65 ? S4 : 2'h2)): begin : h21 initial $display("@Rc5_21 hit"); end default: begin : d21 initial $display("@Rc5_21 def"); end endcase
  case (1'h0) ({8'sh9C, 1'(11)} == $unsigned(16'hdb3f)): begin : h22 initial $display("@Rc5_22 hit"); end default: begin : d22 initial $display("@Rc5_22 def"); end endcase
  case (1'sh0) ({8'sh9C, 1'(11)} == $unsigned(16'hdb3f)): begin : h23 initial $display("@Rc5_23 hit"); end default: begin : d23 initial $display("@Rc5_23 def"); end endcase
  case (4'h0) ({8'sh9C, 1'(11)} == $unsigned(16'hdb3f)): begin : h24 initial $display("@Rc5_24 hit"); end default: begin : d24 initial $display("@Rc5_24 def"); end endcase
endmodule
