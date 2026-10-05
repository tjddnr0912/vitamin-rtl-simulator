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
  case (1'sh1) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h0 initial $display("@Ra19_0 hit"); end default: begin : d0 initial $display("@Ra19_0 def"); end endcase
  case (4'h1) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h1 initial $display("@Ra19_1 hit"); end default: begin : d1 initial $display("@Ra19_1 def"); end endcase
  case (64'hffffffffffffffff) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h2 initial $display("@Ra19_2 hit"); end default: begin : d2 initial $display("@Ra19_2 def"); end endcase
  case ((-1)) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h3 initial $display("@Ra19_3 hit"); end default: begin : d3 initial $display("@Ra19_3 def"); end endcase
  case (1) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h4 initial $display("@Ra19_4 hit"); end default: begin : d4 initial $display("@Ra19_4 def"); end endcase
  case (32'h6) 6: begin : h5 initial $display("@Ra19_5 hit"); end default: begin : d5 initial $display("@Ra19_5 def"); end endcase
  case (32'sh6) 6: begin : h6 initial $display("@Ra19_6 hit"); end default: begin : d6 initial $display("@Ra19_6 def"); end endcase
  case (35'h6) 6: begin : h7 initial $display("@Ra19_7 hit"); end default: begin : d7 initial $display("@Ra19_7 def"); end endcase
  case (64'h6) 6: begin : h8 initial $display("@Ra19_8 hit"); end default: begin : d8 initial $display("@Ra19_8 def"); end endcase
  case (6) 6: begin : h9 initial $display("@Ra19_9 hit"); end default: begin : d9 initial $display("@Ra19_9 def"); end endcase
  case (6) 6: begin : h10 initial $display("@Ra19_10 hit"); end default: begin : d10 initial $display("@Ra19_10 def"); end endcase
  case (1'h1) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h11 initial $display("@Ra19_11 hit"); end default: begin : d11 initial $display("@Ra19_11 def"); end endcase
  case (1'sh1) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h12 initial $display("@Ra19_12 hit"); end default: begin : d12 initial $display("@Ra19_12 def"); end endcase
  case (4'h1) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h13 initial $display("@Ra19_13 hit"); end default: begin : d13 initial $display("@Ra19_13 def"); end endcase
  case (64'hffffffffffffffff) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h14 initial $display("@Ra19_14 hit"); end default: begin : d14 initial $display("@Ra19_14 def"); end endcase
  case ((-1)) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h15 initial $display("@Ra19_15 hit"); end default: begin : d15 initial $display("@Ra19_15 def"); end endcase
  case (1) ((P65 >= 32'h4bd4a21c) | (1'h1 >= 63'h70556d5de14cbde5)): begin : h16 initial $display("@Ra19_16 hit"); end default: begin : d16 initial $display("@Ra19_16 def"); end endcase
  case (32'hfffffffb) ((P65 ^ 8'h63) ? I : (S8 == UN)): begin : h17 initial $display("@Ra19_17 hit"); end default: begin : d17 initial $display("@Ra19_17 def"); end endcase
  case (32'shfffffffb) ((P65 ^ 8'h63) ? I : (S8 == UN)): begin : h18 initial $display("@Ra19_18 hit"); end default: begin : d18 initial $display("@Ra19_18 def"); end endcase
  case (35'hfffffffb) ((P65 ^ 8'h63) ? I : (S8 == UN)): begin : h19 initial $display("@Ra19_19 hit"); end default: begin : d19 initial $display("@Ra19_19 def"); end endcase
  case (64'hfffffffffffffffb) ((P65 ^ 8'h63) ? I : (S8 == UN)): begin : h20 initial $display("@Ra19_20 hit"); end default: begin : d20 initial $display("@Ra19_20 def"); end endcase
  case ((-5)) ((P65 ^ 8'h63) ? I : (S8 == UN)): begin : h21 initial $display("@Ra19_21 hit"); end default: begin : d21 initial $display("@Ra19_21 def"); end endcase
  case (1'h0) $signed((32'hfe85dfb1 < 40)): begin : h22 initial $display("@Ra19_22 hit"); end default: begin : d22 initial $display("@Ra19_22 def"); end endcase
  case (1'sh0) $signed((32'hfe85dfb1 < 40)): begin : h23 initial $display("@Ra19_23 hit"); end default: begin : d23 initial $display("@Ra19_23 def"); end endcase
  case (4'h0) $signed((32'hfe85dfb1 < 40)): begin : h24 initial $display("@Ra19_24 hit"); end default: begin : d24 initial $display("@Ra19_24 def"); end endcase
endmodule
