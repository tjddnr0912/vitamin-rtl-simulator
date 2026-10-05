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
  case (4) 5'sh4: begin : h0 initial $display("@Rb7_0 hit"); end default: begin : d0 initial $display("@Rb7_0 def"); end endcase
  case (4) 5'sh4: begin : h1 initial $display("@Rb7_1 hit"); end default: begin : d1 initial $display("@Rb7_1 def"); end endcase
  case (2'h0) 2'sh0: begin : h2 initial $display("@Rb7_2 hit"); end default: begin : d2 initial $display("@Rb7_2 def"); end endcase
  case (2'sh0) 2'sh0: begin : h3 initial $display("@Rb7_3 hit"); end default: begin : d3 initial $display("@Rb7_3 def"); end endcase
  case (5'h0) 2'sh0: begin : h4 initial $display("@Rb7_4 hit"); end default: begin : d4 initial $display("@Rb7_4 def"); end endcase
  case (64'h0) 2'sh0: begin : h5 initial $display("@Rb7_5 hit"); end default: begin : d5 initial $display("@Rb7_5 def"); end endcase
  case (0) 2'sh0: begin : h6 initial $display("@Rb7_6 hit"); end default: begin : d6 initial $display("@Rb7_6 def"); end endcase
  case (0) 2'sh0: begin : h7 initial $display("@Rb7_7 hit"); end default: begin : d7 initial $display("@Rb7_7 def"); end endcase
  case (1'h1) $signed(((P8 >>> 5) < (S65 << 1))): begin : h8 initial $display("@Rb7_8 hit"); end default: begin : d8 initial $display("@Rb7_8 def"); end endcase
  case (1'sh1) $signed(((P8 >>> 5) < (S65 << 1))): begin : h9 initial $display("@Rb7_9 hit"); end default: begin : d9 initial $display("@Rb7_9 def"); end endcase
  case (4'h1) $signed(((P8 >>> 5) < (S65 << 1))): begin : h10 initial $display("@Rb7_10 hit"); end default: begin : d10 initial $display("@Rb7_10 def"); end endcase
  case (64'hffffffffffffffff) $signed(((P8 >>> 5) < (S65 << 1))): begin : h11 initial $display("@Rb7_11 hit"); end default: begin : d11 initial $display("@Rb7_11 def"); end endcase
  case ((-1)) $signed(((P8 >>> 5) < (S65 << 1))): begin : h12 initial $display("@Rb7_12 hit"); end default: begin : d12 initial $display("@Rb7_12 def"); end endcase
  case (1) $signed(((P8 >>> 5) < (S65 << 1))): begin : h13 initial $display("@Rb7_13 hit"); end default: begin : d13 initial $display("@Rb7_13 def"); end endcase
  case (1'h1) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h14 initial $display("@Rb7_14 hit"); end default: begin : d14 initial $display("@Rb7_14 def"); end endcase
  case (1'sh1) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h15 initial $display("@Rb7_15 hit"); end default: begin : d15 initial $display("@Rb7_15 def"); end endcase
  case (4'h1) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h16 initial $display("@Rb7_16 hit"); end default: begin : d16 initial $display("@Rb7_16 def"); end endcase
  case (64'hffffffffffffffff) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h17 initial $display("@Rb7_17 hit"); end default: begin : d17 initial $display("@Rb7_17 def"); end endcase
  case ((-1)) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h18 initial $display("@Rb7_18 hit"); end default: begin : d18 initial $display("@Rb7_18 def"); end endcase
  case (1) (((4'h9 >> 0) < $signed(S4)) >= ((-(-3)) >>> 1)): begin : h19 initial $display("@Rb7_19 hit"); end default: begin : d19 initial $display("@Rb7_19 def"); end endcase
  case (1'h0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h20 initial $display("@Rb7_20 hit"); end default: begin : d20 initial $display("@Rb7_20 def"); end endcase
  case (1'sh0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h21 initial $display("@Rb7_21 hit"); end default: begin : d21 initial $display("@Rb7_21 def"); end endcase
  case (4'h0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h22 initial $display("@Rb7_22 hit"); end default: begin : d22 initial $display("@Rb7_22 def"); end endcase
  case (64'h0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h23 initial $display("@Rb7_23 hit"); end default: begin : d23 initial $display("@Rb7_23 def"); end endcase
  case (0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h24 initial $display("@Rb7_24 hit"); end default: begin : d24 initial $display("@Rb7_24 def"); end endcase
endmodule
