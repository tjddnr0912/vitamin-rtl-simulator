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
  case (32'h96800) (32'sh8d18011 / P8): begin : h0 initial $display("@Ra6_0 hit"); end default: begin : d0 initial $display("@Ra6_0 def"); end endcase
  case (32'sh96800) (32'sh8d18011 / P8): begin : h1 initial $display("@Ra6_1 hit"); end default: begin : d1 initial $display("@Ra6_1 def"); end endcase
  case (35'h96800) (32'sh8d18011 / P8): begin : h2 initial $display("@Ra6_2 hit"); end default: begin : d2 initial $display("@Ra6_2 def"); end endcase
  case (64'h96800) (32'sh8d18011 / P8): begin : h3 initial $display("@Ra6_3 hit"); end default: begin : d3 initial $display("@Ra6_3 def"); end endcase
  case (616448) (32'sh8d18011 / P8): begin : h4 initial $display("@Ra6_4 hit"); end default: begin : d4 initial $display("@Ra6_4 def"); end endcase
  case (616448) (32'sh8d18011 / P8): begin : h5 initial $display("@Ra6_5 hit"); end default: begin : d5 initial $display("@Ra6_5 def"); end endcase
  case (32'hfffffffb) (I << 0): begin : h6 initial $display("@Ra6_6 hit"); end default: begin : d6 initial $display("@Ra6_6 def"); end endcase
  case (32'shfffffffb) (I << 0): begin : h7 initial $display("@Ra6_7 hit"); end default: begin : d7 initial $display("@Ra6_7 def"); end endcase
  case (35'hfffffffb) (I << 0): begin : h8 initial $display("@Ra6_8 hit"); end default: begin : d8 initial $display("@Ra6_8 def"); end endcase
  case (64'hfffffffffffffffb) (I << 0): begin : h9 initial $display("@Ra6_9 hit"); end default: begin : d9 initial $display("@Ra6_9 def"); end endcase
  case ((-5)) (I << 0): begin : h10 initial $display("@Ra6_10 hit"); end default: begin : d10 initial $display("@Ra6_10 def"); end endcase
  case (64'hfffffffce) ((16'sh4de2 * 32'h27be9ab1) ? {33'(S65), 3'(3'sh6)} : (L64 | 37)): begin : h11 initial $display("@Ra6_11 hit"); end default: begin : d11 initial $display("@Ra6_11 def"); end endcase
  case (64'shfffffffce) ((16'sh4de2 * 32'h27be9ab1) ? {33'(S65), 3'(3'sh6)} : (L64 | 37)): begin : h12 initial $display("@Ra6_12 hit"); end default: begin : d12 initial $display("@Ra6_12 def"); end endcase
  case (64'hfffffffce) ((16'sh4de2 * 32'h27be9ab1) ? {33'(S65), 3'(3'sh6)} : (L64 | 37)): begin : h13 initial $display("@Ra6_13 hit"); end default: begin : d13 initial $display("@Ra6_13 def"); end endcase
  case (64'hfffffffce) ((16'sh4de2 * 32'h27be9ab1) ? {33'(S65), 3'(3'sh6)} : (L64 | 37)): begin : h14 initial $display("@Ra6_14 hit"); end default: begin : d14 initial $display("@Ra6_14 def"); end endcase
  case (1'h1) (-(S4 < 32'sh4d2be09)): begin : h15 initial $display("@Ra6_15 hit"); end default: begin : d15 initial $display("@Ra6_15 def"); end endcase
  case (1'sh1) (-(S4 < 32'sh4d2be09)): begin : h16 initial $display("@Ra6_16 hit"); end default: begin : d16 initial $display("@Ra6_16 def"); end endcase
  case (4'h1) (-(S4 < 32'sh4d2be09)): begin : h17 initial $display("@Ra6_17 hit"); end default: begin : d17 initial $display("@Ra6_17 def"); end endcase
  case (64'hffffffffffffffff) (-(S4 < 32'sh4d2be09)): begin : h18 initial $display("@Ra6_18 hit"); end default: begin : d18 initial $display("@Ra6_18 def"); end endcase
  case ((-1)) (-(S4 < 32'sh4d2be09)): begin : h19 initial $display("@Ra6_19 hit"); end default: begin : d19 initial $display("@Ra6_19 def"); end endcase
  case (1) (-(S4 < 32'sh4d2be09)): begin : h20 initial $display("@Ra6_20 hit"); end default: begin : d20 initial $display("@Ra6_20 def"); end endcase
  case (32'hd61e1e1c) ((-32'h1789819f) * {8'(I), S8}): begin : h21 initial $display("@Ra6_21 hit"); end default: begin : d21 initial $display("@Ra6_21 def"); end endcase
  case (32'shd61e1e1c) ((-32'h1789819f) * {8'(I), S8}): begin : h22 initial $display("@Ra6_22 hit"); end default: begin : d22 initial $display("@Ra6_22 def"); end endcase
  case (35'hd61e1e1c) ((-32'h1789819f) * {8'(I), S8}): begin : h23 initial $display("@Ra6_23 hit"); end default: begin : d23 initial $display("@Ra6_23 def"); end endcase
  case (64'hffffffffd61e1e1c) ((-32'h1789819f) * {8'(I), S8}): begin : h24 initial $display("@Ra6_24 hit"); end default: begin : d24 initial $display("@Ra6_24 def"); end endcase
endmodule
