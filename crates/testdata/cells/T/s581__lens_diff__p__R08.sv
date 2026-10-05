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
  case (4'h0) (!(5'h0 < S4)): begin : h0 initial $display("@8_0 hit"); end default: begin : d0 initial $display("@8_0 def"); end endcase
  case (0) (!(5'h0 < S4)): begin : h1 initial $display("@8_1 hit"); end default: begin : d1 initial $display("@8_1 def"); end endcase
  case (64'h0) (!(5'h0 < S4)): begin : h2 initial $display("@8_2 hit"); end default: begin : d2 initial $display("@8_2 def"); end endcase
  case (0) (!(5'h0 < S4)): begin : h3 initial $display("@8_3 hit"); end default: begin : d3 initial $display("@8_3 def"); end endcase
  case (37'h1000000010) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h4 initial $display("@8_4 hit"); end default: begin : d4 initial $display("@8_4 def"); end endcase
  case (37'sh1000000010) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h5 initial $display("@8_5 hit"); end default: begin : d5 initial $display("@8_5 def"); end endcase
  case (40'h1000000010) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h6 initial $display("@8_6 hit"); end default: begin : d6 initial $display("@8_6 def"); end endcase
  case ((-68719476720)) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h7 initial $display("@8_7 hit"); end default: begin : d7 initial $display("@8_7 def"); end endcase
  case (64'hfffffff000000010) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h8 initial $display("@8_8 hit"); end default: begin : d8 initial $display("@8_8 def"); end endcase
  case (68719476752) {33'h1_0000_0001, 4'((P65 << (-1)))}: begin : h9 initial $display("@8_9 hit"); end default: begin : d9 initial $display("@8_9 def"); end endcase
  case (1'h1) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h10 initial $display("@8_10 hit"); end default: begin : d10 initial $display("@8_10 def"); end endcase
  case (1'sh1) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h11 initial $display("@8_11 hit"); end default: begin : d11 initial $display("@8_11 def"); end endcase
  case (4'h1) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h12 initial $display("@8_12 hit"); end default: begin : d12 initial $display("@8_12 def"); end endcase
  case ((-1)) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h13 initial $display("@8_13 hit"); end default: begin : d13 initial $display("@8_13 def"); end endcase
  case (64'hffffffffffffffff) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h14 initial $display("@8_14 hit"); end default: begin : d14 initial $display("@8_14 def"); end endcase
  case (1) ((8'sha8 >> 0) >= (!32'shfe2fb31a)): begin : h15 initial $display("@8_15 hit"); end default: begin : d15 initial $display("@8_15 def"); end endcase
  case (11'h4e1) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h16 initial $display("@8_16 hit"); end default: begin : d16 initial $display("@8_16 def"); end endcase
  case (11'sh4e1) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h17 initial $display("@8_17 hit"); end default: begin : d17 initial $display("@8_17 def"); end endcase
  case (14'h4e1) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h18 initial $display("@8_18 hit"); end default: begin : d18 initial $display("@8_18 def"); end endcase
  case ((-799)) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h19 initial $display("@8_19 hit"); end default: begin : d19 initial $display("@8_19 def"); end endcase
  case (64'hfffffffffffffce1) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h20 initial $display("@8_20 hit"); end default: begin : d20 initial $display("@8_20 def"); end endcase
  case (1249) {8'sh9C, 3'(64'h2b854f6a961aa2f9)}: begin : h21 initial $display("@8_21 hit"); end default: begin : d21 initial $display("@8_21 def"); end endcase
  case (32'h8) 8: begin : h22 initial $display("@8_22 hit"); end default: begin : d22 initial $display("@8_22 def"); end endcase
  case (32'sh8) 8: begin : h23 initial $display("@8_23 hit"); end default: begin : d23 initial $display("@8_23 def"); end endcase
  case (35'h8) 8: begin : h24 initial $display("@8_24 hit"); end default: begin : d24 initial $display("@8_24 def"); end endcase
endmodule
