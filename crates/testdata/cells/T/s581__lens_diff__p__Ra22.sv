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
  case (1) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h0 initial $display("@Ra22_0 hit"); end default: begin : d0 initial $display("@Ra22_0 def"); end endcase
  case (12'hc10) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h1 initial $display("@Ra22_1 hit"); end default: begin : d1 initial $display("@Ra22_1 def"); end endcase
  case (12'shc10) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h2 initial $display("@Ra22_2 hit"); end default: begin : d2 initial $display("@Ra22_2 def"); end endcase
  case (15'hc10) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h3 initial $display("@Ra22_3 hit"); end default: begin : d3 initial $display("@Ra22_3 def"); end endcase
  case (64'hfffffffffffffc10) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h4 initial $display("@Ra22_4 hit"); end default: begin : d4 initial $display("@Ra22_4 def"); end endcase
  case ((-1008)) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h5 initial $display("@Ra22_5 hit"); end default: begin : d5 initial $display("@Ra22_5 def"); end endcase
  case (3088) {P4, 8'((-(65'h180373ba8c9fdac3d ? P8 : 2)))}: begin : h6 initial $display("@Ra22_6 hit"); end default: begin : d6 initial $display("@Ra22_6 def"); end endcase
  case (32'hffffffed) $signed((-(S4 ? 19 : 5'h5))): begin : h7 initial $display("@Ra22_7 hit"); end default: begin : d7 initial $display("@Ra22_7 def"); end endcase
  case (32'shffffffed) $signed((-(S4 ? 19 : 5'h5))): begin : h8 initial $display("@Ra22_8 hit"); end default: begin : d8 initial $display("@Ra22_8 def"); end endcase
  case (35'hffffffed) $signed((-(S4 ? 19 : 5'h5))): begin : h9 initial $display("@Ra22_9 hit"); end default: begin : d9 initial $display("@Ra22_9 def"); end endcase
  case (64'hffffffffffffffed) $signed((-(S4 ? 19 : 5'h5))): begin : h10 initial $display("@Ra22_10 hit"); end default: begin : d10 initial $display("@Ra22_10 def"); end endcase
  case ((-19)) $signed((-(S4 ? 19 : 5'h5))): begin : h11 initial $display("@Ra22_11 hit"); end default: begin : d11 initial $display("@Ra22_11 def"); end endcase
  case (1'h0) (!$signed(32'sh1e6cc084)): begin : h12 initial $display("@Ra22_12 hit"); end default: begin : d12 initial $display("@Ra22_12 def"); end endcase
  case (1'sh0) (!$signed(32'sh1e6cc084)): begin : h13 initial $display("@Ra22_13 hit"); end default: begin : d13 initial $display("@Ra22_13 def"); end endcase
  case (4'h0) (!$signed(32'sh1e6cc084)): begin : h14 initial $display("@Ra22_14 hit"); end default: begin : d14 initial $display("@Ra22_14 def"); end endcase
  case (64'h0) (!$signed(32'sh1e6cc084)): begin : h15 initial $display("@Ra22_15 hit"); end default: begin : d15 initial $display("@Ra22_15 def"); end endcase
  case (0) (!$signed(32'sh1e6cc084)): begin : h16 initial $display("@Ra22_16 hit"); end default: begin : d16 initial $display("@Ra22_16 def"); end endcase
  case (0) (!$signed(32'sh1e6cc084)): begin : h17 initial $display("@Ra22_17 hit"); end default: begin : d17 initial $display("@Ra22_17 def"); end endcase
  case (63'h1d745ab086490306) $unsigned(((~63'h628ba54f79b6fcb9) & (~P8))): begin : h18 initial $display("@Ra22_18 hit"); end default: begin : d18 initial $display("@Ra22_18 def"); end endcase
  case (63'sh1d745ab086490306) $unsigned(((~63'h628ba54f79b6fcb9) & (~P8))): begin : h19 initial $display("@Ra22_19 hit"); end default: begin : d19 initial $display("@Ra22_19 def"); end endcase
  case (64'h1d745ab086490306) $unsigned(((~63'h628ba54f79b6fcb9) & (~P8))): begin : h20 initial $display("@Ra22_20 hit"); end default: begin : d20 initial $display("@Ra22_20 def"); end endcase
  case (64'h1d745ab086490306) $unsigned(((~63'h628ba54f79b6fcb9) & (~P8))): begin : h21 initial $display("@Ra22_21 hit"); end default: begin : d21 initial $display("@Ra22_21 def"); end endcase
  case (4'h9) (S4 * S4): begin : h22 initial $display("@Ra22_22 hit"); end default: begin : d22 initial $display("@Ra22_22 def"); end endcase
  case (4'sh9) (S4 * S4): begin : h23 initial $display("@Ra22_23 hit"); end default: begin : d23 initial $display("@Ra22_23 def"); end endcase
  case (7'h9) (S4 * S4): begin : h24 initial $display("@Ra22_24 hit"); end default: begin : d24 initial $display("@Ra22_24 def"); end endcase
endmodule
