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
  case (1'h0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h0 initial $display("@Rc20_0 hit"); end default: begin : d0 initial $display("@Rc20_0 def"); end endcase
  case (1'sh0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h1 initial $display("@Rc20_1 hit"); end default: begin : d1 initial $display("@Rc20_1 def"); end endcase
  case (4'h0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h2 initial $display("@Rc20_2 hit"); end default: begin : d2 initial $display("@Rc20_2 def"); end endcase
  case (64'h0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h3 initial $display("@Rc20_3 hit"); end default: begin : d3 initial $display("@Rc20_3 def"); end endcase
  case (0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h4 initial $display("@Rc20_4 hit"); end default: begin : d4 initial $display("@Rc20_4 def"); end endcase
  case (0) (3'h7 && $onehot(65'((S8 | 31'sh80bd4df)))): begin : h5 initial $display("@Rc20_5 hit"); end default: begin : d5 initial $display("@Rc20_5 def"); end endcase
  case (1'h0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h6 initial $display("@Rc20_6 hit"); end default: begin : d6 initial $display("@Rc20_6 def"); end endcase
  case (1'sh0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h7 initial $display("@Rc20_7 hit"); end default: begin : d7 initial $display("@Rc20_7 def"); end endcase
  case (4'h0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h8 initial $display("@Rc20_8 hit"); end default: begin : d8 initial $display("@Rc20_8 def"); end endcase
  case (64'h0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h9 initial $display("@Rc20_9 hit"); end default: begin : d9 initial $display("@Rc20_9 def"); end endcase
  case (0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h10 initial $display("@Rc20_10 hit"); end default: begin : d10 initial $display("@Rc20_10 def"); end endcase
  case (0) ((!(U32 >> 1)) ? $onehot(P8) : (~|signed'(2'sh2))): begin : h11 initial $display("@Rc20_11 hit"); end default: begin : d11 initial $display("@Rc20_11 def"); end endcase
  case (64'h1656963774db3b6) 64'h1656963774db3b6: begin : h12 initial $display("@Rc20_12 hit"); end default: begin : d12 initial $display("@Rc20_12 def"); end endcase
  case (64'sh1656963774db3b6) 64'h1656963774db3b6: begin : h13 initial $display("@Rc20_13 hit"); end default: begin : d13 initial $display("@Rc20_13 def"); end endcase
  case (64'h1656963774db3b6) 64'h1656963774db3b6: begin : h14 initial $display("@Rc20_14 hit"); end default: begin : d14 initial $display("@Rc20_14 def"); end endcase
  case (64'h1656963774db3b6) 64'h1656963774db3b6: begin : h15 initial $display("@Rc20_15 hit"); end default: begin : d15 initial $display("@Rc20_15 def"); end endcase
  case (32'ha) (UN ? UN : W6): begin : h16 initial $display("@Rc20_16 hit"); end default: begin : d16 initial $display("@Rc20_16 def"); end endcase
  case (32'sha) (UN ? UN : W6): begin : h17 initial $display("@Rc20_17 hit"); end default: begin : d17 initial $display("@Rc20_17 def"); end endcase
  case (35'ha) (UN ? UN : W6): begin : h18 initial $display("@Rc20_18 hit"); end default: begin : d18 initial $display("@Rc20_18 def"); end endcase
  case (64'hffffffff0000000a) (UN ? UN : W6): begin : h19 initial $display("@Rc20_19 hit"); end default: begin : d19 initial $display("@Rc20_19 def"); end endcase
  case (10) (UN ? UN : W6): begin : h20 initial $display("@Rc20_20 hit"); end default: begin : d20 initial $display("@Rc20_20 def"); end endcase
  case (32'ha) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h21 initial $display("@Rc20_21 hit"); end default: begin : d21 initial $display("@Rc20_21 def"); end endcase
  case (32'sha) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h22 initial $display("@Rc20_22 hit"); end default: begin : d22 initial $display("@Rc20_22 def"); end endcase
  case (35'ha) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h23 initial $display("@Rc20_23 hit"); end default: begin : d23 initial $display("@Rc20_23 def"); end endcase
  case (64'ha) ((UN % 31) >> (~|64'sh832fd33d2ccefd12)): begin : h24 initial $display("@Rc20_24 hit"); end default: begin : d24 initial $display("@Rc20_24 def"); end endcase
endmodule
