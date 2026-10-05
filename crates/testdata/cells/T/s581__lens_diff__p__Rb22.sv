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
  case (1) (UN && 38): begin : h0 initial $display("@Rb22_0 hit"); end default: begin : d0 initial $display("@Rb22_0 def"); end endcase
  case (1'h1) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h1 initial $display("@Rb22_1 hit"); end default: begin : d1 initial $display("@Rb22_1 def"); end endcase
  case (1'sh1) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h2 initial $display("@Rb22_2 hit"); end default: begin : d2 initial $display("@Rb22_2 def"); end endcase
  case (4'h1) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h3 initial $display("@Rb22_3 hit"); end default: begin : d3 initial $display("@Rb22_3 def"); end endcase
  case (64'hffffffffffffffff) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h4 initial $display("@Rb22_4 hit"); end default: begin : d4 initial $display("@Rb22_4 def"); end endcase
  case ((-1)) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h5 initial $display("@Rb22_5 hit"); end default: begin : d5 initial $display("@Rb22_5 def"); end endcase
  case (1) ((U32 < 63'h27f8d6c77156764d) - (!P65)): begin : h6 initial $display("@Rb22_6 hit"); end default: begin : d6 initial $display("@Rb22_6 def"); end endcase
  case (64'hfffffffffffffff7) (63'sh60691b48b4ef78f2 ? L64 : 25): begin : h7 initial $display("@Rb22_7 hit"); end default: begin : d7 initial $display("@Rb22_7 def"); end endcase
  case (64'shfffffffffffffff7) (63'sh60691b48b4ef78f2 ? L64 : 25): begin : h8 initial $display("@Rb22_8 hit"); end default: begin : d8 initial $display("@Rb22_8 def"); end endcase
  case (64'hfffffffffffffff7) (63'sh60691b48b4ef78f2 ? L64 : 25): begin : h9 initial $display("@Rb22_9 hit"); end default: begin : d9 initial $display("@Rb22_9 def"); end endcase
  case (64'hfffffffffffffff7) (63'sh60691b48b4ef78f2 ? L64 : 25): begin : h10 initial $display("@Rb22_10 hit"); end default: begin : d10 initial $display("@Rb22_10 def"); end endcase
  case ((-9)) (63'sh60691b48b4ef78f2 ? L64 : 25): begin : h11 initial $display("@Rb22_11 hit"); end default: begin : d11 initial $display("@Rb22_11 def"); end endcase
  case (63'h0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h12 initial $display("@Rb22_12 hit"); end default: begin : d12 initial $display("@Rb22_12 def"); end endcase
  case (63'sh0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h13 initial $display("@Rb22_13 hit"); end default: begin : d13 initial $display("@Rb22_13 def"); end endcase
  case (64'h0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h14 initial $display("@Rb22_14 hit"); end default: begin : d14 initial $display("@Rb22_14 def"); end endcase
  case (64'h0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h15 initial $display("@Rb22_15 hit"); end default: begin : d15 initial $display("@Rb22_15 def"); end endcase
  case (0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h16 initial $display("@Rb22_16 hit"); end default: begin : d16 initial $display("@Rb22_16 def"); end endcase
  case (0) ((!P8) + (2'sh1 & 63'hc402abad001ce92)): begin : h17 initial $display("@Rb22_17 hit"); end default: begin : d17 initial $display("@Rb22_17 def"); end endcase
  case (1'h0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h18 initial $display("@Rb22_18 hit"); end default: begin : d18 initial $display("@Rb22_18 def"); end endcase
  case (1'sh0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h19 initial $display("@Rb22_19 hit"); end default: begin : d19 initial $display("@Rb22_19 def"); end endcase
  case (4'h0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h20 initial $display("@Rb22_20 hit"); end default: begin : d20 initial $display("@Rb22_20 def"); end endcase
  case (64'h0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h21 initial $display("@Rb22_21 hit"); end default: begin : d21 initial $display("@Rb22_21 def"); end endcase
  case (0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h22 initial $display("@Rb22_22 hit"); end default: begin : d22 initial $display("@Rb22_22 def"); end endcase
  case (0) $unsigned(((U32 ? 64'h4c1e8f66d1a42061 : 16'sh9fba) >= $signed(64'he0652e023438f5d5))): begin : h23 initial $display("@Rb22_23 hit"); end default: begin : d23 initial $display("@Rb22_23 def"); end endcase
  case (32'h4) (~I): begin : h24 initial $display("@Rb22_24 hit"); end default: begin : d24 initial $display("@Rb22_24 def"); end endcase
endmodule
