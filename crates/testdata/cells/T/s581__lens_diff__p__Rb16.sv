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
  case (33'shd) (((P65 ? 2'sh1 : UN) ? {2{8'(2'h2)}} : (5'sh1c < UN)) ? S4 : $signed((33'sh624d15dc | S4))): begin : h0 initial $display("@Rb16_0 hit"); end default: begin : d0 initial $display("@Rb16_0 def"); end endcase
  case (36'hd) (((P65 ? 2'sh1 : UN) ? {2{8'(2'h2)}} : (5'sh1c < UN)) ? S4 : $signed((33'sh624d15dc | S4))): begin : h1 initial $display("@Rb16_1 hit"); end default: begin : d1 initial $display("@Rb16_1 def"); end endcase
  case (64'hfffffffe0000000d) (((P65 ? 2'sh1 : UN) ? {2{8'(2'h2)}} : (5'sh1c < UN)) ? S4 : $signed((33'sh624d15dc | S4))): begin : h2 initial $display("@Rb16_2 hit"); end default: begin : d2 initial $display("@Rb16_2 def"); end endcase
  case (13) (((P65 ? 2'sh1 : UN) ? {2{8'(2'h2)}} : (5'sh1c < UN)) ? S4 : $signed((33'sh624d15dc | S4))): begin : h3 initial $display("@Rb16_3 hit"); end default: begin : d3 initial $display("@Rb16_3 def"); end endcase
  case (8'hf6) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h4 initial $display("@Rb16_4 hit"); end default: begin : d4 initial $display("@Rb16_4 def"); end endcase
  case (8'shf6) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h5 initial $display("@Rb16_5 hit"); end default: begin : d5 initial $display("@Rb16_5 def"); end endcase
  case (11'hf6) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h6 initial $display("@Rb16_6 hit"); end default: begin : d6 initial $display("@Rb16_6 def"); end endcase
  case (64'hfffffffffffffff6) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h7 initial $display("@Rb16_7 hit"); end default: begin : d7 initial $display("@Rb16_7 def"); end endcase
  case ((-10)) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h8 initial $display("@Rb16_8 hit"); end default: begin : d8 initial $display("@Rb16_8 def"); end endcase
  case (246) (({8'(S4), 1'(UN)} ? (-4'ha) : (8 == 64'h798841727262ef51)) ^ ((~P8) * (31'sh7e052dd1 ? 1'sh0 : P4))): begin : h9 initial $display("@Rb16_9 hit"); end default: begin : d9 initial $display("@Rb16_9 def"); end endcase
  case (32'hd7e7e7fc) 32'hd7e7e7fc: begin : h10 initial $display("@Rb16_10 hit"); end default: begin : d10 initial $display("@Rb16_10 def"); end endcase
  case (32'shd7e7e7fc) 32'hd7e7e7fc: begin : h11 initial $display("@Rb16_11 hit"); end default: begin : d11 initial $display("@Rb16_11 def"); end endcase
  case (35'hd7e7e7fc) 32'hd7e7e7fc: begin : h12 initial $display("@Rb16_12 hit"); end default: begin : d12 initial $display("@Rb16_12 def"); end endcase
  case (64'hffffffffd7e7e7fc) 32'hd7e7e7fc: begin : h13 initial $display("@Rb16_13 hit"); end default: begin : d13 initial $display("@Rb16_13 def"); end endcase
  case ((-672667652)) 32'hd7e7e7fc: begin : h14 initial $display("@Rb16_14 hit"); end default: begin : d14 initial $display("@Rb16_14 def"); end endcase
  case (32'hfffffffb) $signed(I): begin : h15 initial $display("@Rb16_15 hit"); end default: begin : d15 initial $display("@Rb16_15 def"); end endcase
  case (32'shfffffffb) $signed(I): begin : h16 initial $display("@Rb16_16 hit"); end default: begin : d16 initial $display("@Rb16_16 def"); end endcase
  case (35'hfffffffb) $signed(I): begin : h17 initial $display("@Rb16_17 hit"); end default: begin : d17 initial $display("@Rb16_17 def"); end endcase
  case (64'hfffffffffffffffb) $signed(I): begin : h18 initial $display("@Rb16_18 hit"); end default: begin : d18 initial $display("@Rb16_18 def"); end endcase
  case ((-5)) $signed(I): begin : h19 initial $display("@Rb16_19 hit"); end default: begin : d19 initial $display("@Rb16_19 def"); end endcase
  case (16'hcc61) $unsigned(16'shcc61): begin : h20 initial $display("@Rb16_20 hit"); end default: begin : d20 initial $display("@Rb16_20 def"); end endcase
  case (16'shcc61) $unsigned(16'shcc61): begin : h21 initial $display("@Rb16_21 hit"); end default: begin : d21 initial $display("@Rb16_21 def"); end endcase
  case (19'hcc61) $unsigned(16'shcc61): begin : h22 initial $display("@Rb16_22 hit"); end default: begin : d22 initial $display("@Rb16_22 def"); end endcase
  case (64'hffffffffffffcc61) $unsigned(16'shcc61): begin : h23 initial $display("@Rb16_23 hit"); end default: begin : d23 initial $display("@Rb16_23 def"); end endcase
  case ((-13215)) $unsigned(16'shcc61): begin : h24 initial $display("@Rb16_24 hit"); end default: begin : d24 initial $display("@Rb16_24 def"); end endcase
endmodule
