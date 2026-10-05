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
  case (52321) $unsigned(16'shcc61): begin : h0 initial $display("@Rb17_0 hit"); end default: begin : d0 initial $display("@Rb17_0 def"); end endcase
  case (32'h0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h1 initial $display("@Rb17_1 hit"); end default: begin : d1 initial $display("@Rb17_1 def"); end endcase
  case (32'sh0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h2 initial $display("@Rb17_2 hit"); end default: begin : d2 initial $display("@Rb17_2 def"); end endcase
  case (35'h0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h3 initial $display("@Rb17_3 hit"); end default: begin : d3 initial $display("@Rb17_3 def"); end endcase
  case (64'h0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h4 initial $display("@Rb17_4 hit"); end default: begin : d4 initial $display("@Rb17_4 def"); end endcase
  case (0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h5 initial $display("@Rb17_5 hit"); end default: begin : d5 initial $display("@Rb17_5 def"); end endcase
  case (0) ((P8 ? U32 : S8) >>> ((-2) << 1)): begin : h6 initial $display("@Rb17_6 hit"); end default: begin : d6 initial $display("@Rb17_6 def"); end endcase
  case (3'h5) 3'h5: begin : h7 initial $display("@Rb17_7 hit"); end default: begin : d7 initial $display("@Rb17_7 def"); end endcase
  case (3'sh5) 3'h5: begin : h8 initial $display("@Rb17_8 hit"); end default: begin : d8 initial $display("@Rb17_8 def"); end endcase
  case (6'h5) 3'h5: begin : h9 initial $display("@Rb17_9 hit"); end default: begin : d9 initial $display("@Rb17_9 def"); end endcase
  case (64'hfffffffffffffffd) 3'h5: begin : h10 initial $display("@Rb17_10 hit"); end default: begin : d10 initial $display("@Rb17_10 def"); end endcase
  case ((-3)) 3'h5: begin : h11 initial $display("@Rb17_11 hit"); end default: begin : d11 initial $display("@Rb17_11 def"); end endcase
  case (5) 3'h5: begin : h12 initial $display("@Rb17_12 hit"); end default: begin : d12 initial $display("@Rb17_12 def"); end endcase
  case (41'hfffffffe64) (-{33'h1_0000_0001, S8}): begin : h13 initial $display("@Rb17_13 hit"); end default: begin : d13 initial $display("@Rb17_13 def"); end endcase
  case (41'shfffffffe64) (-{33'h1_0000_0001, S8}): begin : h14 initial $display("@Rb17_14 hit"); end default: begin : d14 initial $display("@Rb17_14 def"); end endcase
  case (44'hfffffffe64) (-{33'h1_0000_0001, S8}): begin : h15 initial $display("@Rb17_15 hit"); end default: begin : d15 initial $display("@Rb17_15 def"); end endcase
  case (64'hfffffffe64) (-{33'h1_0000_0001, S8}): begin : h16 initial $display("@Rb17_16 hit"); end default: begin : d16 initial $display("@Rb17_16 def"); end endcase
  case (8'h0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h17 initial $display("@Rb17_17 hit"); end default: begin : d17 initial $display("@Rb17_17 def"); end endcase
  case (8'sh0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h18 initial $display("@Rb17_18 hit"); end default: begin : d18 initial $display("@Rb17_18 def"); end endcase
  case (11'h0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h19 initial $display("@Rb17_19 hit"); end default: begin : d19 initial $display("@Rb17_19 def"); end endcase
  case (64'h0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h20 initial $display("@Rb17_20 hit"); end default: begin : d20 initial $display("@Rb17_20 def"); end endcase
  case (0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h21 initial $display("@Rb17_21 hit"); end default: begin : d21 initial $display("@Rb17_21 def"); end endcase
  case (0) (((S65 | L64) ? (2'h0 ? S4 : P8) : (S65 && P65)) >> ((L64 * S4) | (U32 - P8))): begin : h22 initial $display("@Rb17_22 hit"); end default: begin : d22 initial $display("@Rb17_22 def"); end endcase
  case (63'h0) ((~63'h53a7ba5a6c50d2cd) << (63'sh6d7586c12e7af5d << 4)): begin : h23 initial $display("@Rb17_23 hit"); end default: begin : d23 initial $display("@Rb17_23 def"); end endcase
  case (63'sh0) ((~63'h53a7ba5a6c50d2cd) << (63'sh6d7586c12e7af5d << 4)): begin : h24 initial $display("@Rb17_24 hit"); end default: begin : d24 initial $display("@Rb17_24 def"); end endcase
endmodule
