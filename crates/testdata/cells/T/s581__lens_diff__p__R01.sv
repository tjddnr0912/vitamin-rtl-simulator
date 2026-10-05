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
  case (1'sh1) ((I == 3'sh0) || U32): begin : h0 initial $display("@1_0 hit"); end default: begin : d0 initial $display("@1_0 def"); end endcase
  case (4'h1) ((I == 3'sh0) || U32): begin : h1 initial $display("@1_1 hit"); end default: begin : d1 initial $display("@1_1 def"); end endcase
  case ((-1)) ((I == 3'sh0) || U32): begin : h2 initial $display("@1_2 hit"); end default: begin : d2 initial $display("@1_2 def"); end endcase
  case (64'hffffffffffffffff) ((I == 3'sh0) || U32): begin : h3 initial $display("@1_3 hit"); end default: begin : d3 initial $display("@1_3 def"); end endcase
  case (1) ((I == 3'sh0) || U32): begin : h4 initial $display("@1_4 hit"); end default: begin : d4 initial $display("@1_4 def"); end endcase
  case (32'h0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h5 initial $display("@1_5 hit"); end default: begin : d5 initial $display("@1_5 def"); end endcase
  case (32'sh0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h6 initial $display("@1_6 hit"); end default: begin : d6 initial $display("@1_6 def"); end endcase
  case (35'h0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h7 initial $display("@1_7 hit"); end default: begin : d7 initial $display("@1_7 def"); end endcase
  case (0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h8 initial $display("@1_8 hit"); end default: begin : d8 initial $display("@1_8 def"); end endcase
  case (64'h0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h9 initial $display("@1_9 hit"); end default: begin : d9 initial $display("@1_9 def"); end endcase
  case (0) ((-(-7)) / (32'h3c5cab3 * S8)): begin : h10 initial $display("@1_10 hit"); end default: begin : d10 initial $display("@1_10 def"); end endcase
  case (64'h0) ({2{8'(P4)}} / ((-4) | L64)): begin : h11 initial $display("@1_11 hit"); end default: begin : d11 initial $display("@1_11 def"); end endcase
  case (64'sh0) ({2{8'(P4)}} / ((-4) | L64)): begin : h12 initial $display("@1_12 hit"); end default: begin : d12 initial $display("@1_12 def"); end endcase
  case (64'h0) ({2{8'(P4)}} / ((-4) | L64)): begin : h13 initial $display("@1_13 hit"); end default: begin : d13 initial $display("@1_13 def"); end endcase
  case (0) ({2{8'(P4)}} / ((-4) | L64)): begin : h14 initial $display("@1_14 hit"); end default: begin : d14 initial $display("@1_14 def"); end endcase
  case (64'h0) ({2{8'(P4)}} / ((-4) | L64)): begin : h15 initial $display("@1_15 hit"); end default: begin : d15 initial $display("@1_15 def"); end endcase
  case (0) ({2{8'(P4)}} / ((-4) | L64)): begin : h16 initial $display("@1_16 hit"); end default: begin : d16 initial $display("@1_16 def"); end endcase
  case (32'h2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h17 initial $display("@1_17 hit"); end default: begin : d17 initial $display("@1_17 def"); end endcase
  case (32'sh2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h18 initial $display("@1_18 hit"); end default: begin : d18 initial $display("@1_18 def"); end endcase
  case (35'h2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h19 initial $display("@1_19 hit"); end default: begin : d19 initial $display("@1_19 def"); end endcase
  case (2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h20 initial $display("@1_20 hit"); end default: begin : d20 initial $display("@1_20 def"); end endcase
  case (64'h2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h21 initial $display("@1_21 hit"); end default: begin : d21 initial $display("@1_21 def"); end endcase
  case (2) ((3'h4 >> 3) ? (-12) : $unsigned(5'h2)): begin : h22 initial $display("@1_22 hit"); end default: begin : d22 initial $display("@1_22 def"); end endcase
  case (32'h1111111) ((UN / 19) | (U32 / P8)): begin : h23 initial $display("@1_23 hit"); end default: begin : d23 initial $display("@1_23 def"); end endcase
  case (32'sh1111111) ((UN / 19) | (U32 / P8)): begin : h24 initial $display("@1_24 hit"); end default: begin : d24 initial $display("@1_24 def"); end endcase
endmodule
