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
  case (4'sha) UN: begin : h0 initial $display("@Rb20_0 hit"); end default: begin : d0 initial $display("@Rb20_0 def"); end endcase
  case (7'ha) UN: begin : h1 initial $display("@Rb20_1 hit"); end default: begin : d1 initial $display("@Rb20_1 def"); end endcase
  case (64'hfffffffffffffffa) UN: begin : h2 initial $display("@Rb20_2 hit"); end default: begin : d2 initial $display("@Rb20_2 def"); end endcase
  case ((-6)) UN: begin : h3 initial $display("@Rb20_3 hit"); end default: begin : d3 initial $display("@Rb20_3 def"); end endcase
  case (10) UN: begin : h4 initial $display("@Rb20_4 hit"); end default: begin : d4 initial $display("@Rb20_4 def"); end endcase
  case (32'ha) $signed((21 >> 1)): begin : h5 initial $display("@Rb20_5 hit"); end default: begin : d5 initial $display("@Rb20_5 def"); end endcase
  case (32'sha) $signed((21 >> 1)): begin : h6 initial $display("@Rb20_6 hit"); end default: begin : d6 initial $display("@Rb20_6 def"); end endcase
  case (35'ha) $signed((21 >> 1)): begin : h7 initial $display("@Rb20_7 hit"); end default: begin : d7 initial $display("@Rb20_7 def"); end endcase
  case (64'ha) $signed((21 >> 1)): begin : h8 initial $display("@Rb20_8 hit"); end default: begin : d8 initial $display("@Rb20_8 def"); end endcase
  case (10) $signed((21 >> 1)): begin : h9 initial $display("@Rb20_9 hit"); end default: begin : d9 initial $display("@Rb20_9 def"); end endcase
  case (10) $signed((21 >> 1)): begin : h10 initial $display("@Rb20_10 hit"); end default: begin : d10 initial $display("@Rb20_10 def"); end endcase
  case (33'he69dad54) (33'she69dad51 ^ 5): begin : h11 initial $display("@Rb20_11 hit"); end default: begin : d11 initial $display("@Rb20_11 def"); end endcase
  case (33'she69dad54) (33'she69dad51 ^ 5): begin : h12 initial $display("@Rb20_12 hit"); end default: begin : d12 initial $display("@Rb20_12 def"); end endcase
  case (36'he69dad54) (33'she69dad51 ^ 5): begin : h13 initial $display("@Rb20_13 hit"); end default: begin : d13 initial $display("@Rb20_13 def"); end endcase
  case (64'he69dad54) (33'she69dad51 ^ 5): begin : h14 initial $display("@Rb20_14 hit"); end default: begin : d14 initial $display("@Rb20_14 def"); end endcase
  case (1'h1) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h15 initial $display("@Rb20_15 hit"); end default: begin : d15 initial $display("@Rb20_15 def"); end endcase
  case (1'sh1) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h16 initial $display("@Rb20_16 hit"); end default: begin : d16 initial $display("@Rb20_16 def"); end endcase
  case (4'h1) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h17 initial $display("@Rb20_17 hit"); end default: begin : d17 initial $display("@Rb20_17 def"); end endcase
  case (64'hffffffffffffffff) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h18 initial $display("@Rb20_18 hit"); end default: begin : d18 initial $display("@Rb20_18 def"); end endcase
  case ((-1)) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h19 initial $display("@Rb20_19 hit"); end default: begin : d19 initial $display("@Rb20_19 def"); end endcase
  case (1) (~((5'h10 >= P65) << (~33'h1381c5175))): begin : h20 initial $display("@Rb20_20 hit"); end default: begin : d20 initial $display("@Rb20_20 def"); end endcase
  case (1'h1) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h21 initial $display("@Rb20_21 hit"); end default: begin : d21 initial $display("@Rb20_21 def"); end endcase
  case (1'sh1) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h22 initial $display("@Rb20_22 hit"); end default: begin : d22 initial $display("@Rb20_22 def"); end endcase
  case (4'h1) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h23 initial $display("@Rb20_23 hit"); end default: begin : d23 initial $display("@Rb20_23 def"); end endcase
  case (64'hffffffffffffffff) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h24 initial $display("@Rb20_24 hit"); end default: begin : d24 initial $display("@Rb20_24 def"); end endcase
endmodule
