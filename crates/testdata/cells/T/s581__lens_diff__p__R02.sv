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
  case (35'h1111111) ((UN / 19) | (U32 / P8)): begin : h0 initial $display("@2_0 hit"); end default: begin : d0 initial $display("@2_0 def"); end endcase
  case (17895697) ((UN / 19) | (U32 / P8)): begin : h1 initial $display("@2_1 hit"); end default: begin : d1 initial $display("@2_1 def"); end endcase
  case (64'h1111111) ((UN / 19) | (U32 / P8)): begin : h2 initial $display("@2_2 hit"); end default: begin : d2 initial $display("@2_2 def"); end endcase
  case (17895697) ((UN / 19) | (U32 / P8)): begin : h3 initial $display("@2_3 hit"); end default: begin : d3 initial $display("@2_3 def"); end endcase
  case (4'h3) (-S4): begin : h4 initial $display("@2_4 hit"); end default: begin : d4 initial $display("@2_4 def"); end endcase
  case (4'sh3) (-S4): begin : h5 initial $display("@2_5 hit"); end default: begin : d5 initial $display("@2_5 def"); end endcase
  case (7'h3) (-S4): begin : h6 initial $display("@2_6 hit"); end default: begin : d6 initial $display("@2_6 def"); end endcase
  case (3) (-S4): begin : h7 initial $display("@2_7 hit"); end default: begin : d7 initial $display("@2_7 def"); end endcase
  case (64'h3) (-S4): begin : h8 initial $display("@2_8 hit"); end default: begin : d8 initial $display("@2_8 def"); end endcase
  case (3) (-S4): begin : h9 initial $display("@2_9 hit"); end default: begin : d9 initial $display("@2_9 def"); end endcase
  case (64'h1b) $signed((L64 * 5'sh1d)): begin : h10 initial $display("@2_10 hit"); end default: begin : d10 initial $display("@2_10 def"); end endcase
  case (64'sh1b) $signed((L64 * 5'sh1d)): begin : h11 initial $display("@2_11 hit"); end default: begin : d11 initial $display("@2_11 def"); end endcase
  case (64'h1b) $signed((L64 * 5'sh1d)): begin : h12 initial $display("@2_12 hit"); end default: begin : d12 initial $display("@2_12 def"); end endcase
  case (27) $signed((L64 * 5'sh1d)): begin : h13 initial $display("@2_13 hit"); end default: begin : d13 initial $display("@2_13 def"); end endcase
  case (64'h1b) $signed((L64 * 5'sh1d)): begin : h14 initial $display("@2_14 hit"); end default: begin : d14 initial $display("@2_14 def"); end endcase
  case (27) $signed((L64 * 5'sh1d)): begin : h15 initial $display("@2_15 hit"); end default: begin : d15 initial $display("@2_15 def"); end endcase
  case (32'hffffffbd) (S8 | 33): begin : h16 initial $display("@2_16 hit"); end default: begin : d16 initial $display("@2_16 def"); end endcase
  case (32'shffffffbd) (S8 | 33): begin : h17 initial $display("@2_17 hit"); end default: begin : d17 initial $display("@2_17 def"); end endcase
  case (35'hffffffbd) (S8 | 33): begin : h18 initial $display("@2_18 hit"); end default: begin : d18 initial $display("@2_18 def"); end endcase
  case ((-67)) (S8 | 33): begin : h19 initial $display("@2_19 hit"); end default: begin : d19 initial $display("@2_19 def"); end endcase
  case (64'hffffffffffffffbd) (S8 | 33): begin : h20 initial $display("@2_20 hit"); end default: begin : d20 initial $display("@2_20 def"); end endcase
  case (4294967229) (S8 | 33): begin : h21 initial $display("@2_21 hit"); end default: begin : d21 initial $display("@2_21 def"); end endcase
  case (8'h0) {2{4'(U32)}}: begin : h22 initial $display("@2_22 hit"); end default: begin : d22 initial $display("@2_22 def"); end endcase
  case (8'sh0) {2{4'(U32)}}: begin : h23 initial $display("@2_23 hit"); end default: begin : d23 initial $display("@2_23 def"); end endcase
  case (11'h0) {2{4'(U32)}}: begin : h24 initial $display("@2_24 hit"); end default: begin : d24 initial $display("@2_24 def"); end endcase
endmodule
