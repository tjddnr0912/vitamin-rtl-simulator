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
  case (0) {2{4'(U32)}}: begin : h0 initial $display("@3_0 hit"); end default: begin : d0 initial $display("@3_0 def"); end endcase
  case (64'h0) {2{4'(U32)}}: begin : h1 initial $display("@3_1 hit"); end default: begin : d1 initial $display("@3_1 def"); end endcase
  case (0) {2{4'(U32)}}: begin : h2 initial $display("@3_2 hit"); end default: begin : d2 initial $display("@3_2 def"); end endcase
  case (1'h0) (!((-2) + L64)): begin : h3 initial $display("@3_3 hit"); end default: begin : d3 initial $display("@3_3 def"); end endcase
  case (1'sh0) (!((-2) + L64)): begin : h4 initial $display("@3_4 hit"); end default: begin : d4 initial $display("@3_4 def"); end endcase
  case (4'h0) (!((-2) + L64)): begin : h5 initial $display("@3_5 hit"); end default: begin : d5 initial $display("@3_5 def"); end endcase
  case (0) (!((-2) + L64)): begin : h6 initial $display("@3_6 hit"); end default: begin : d6 initial $display("@3_6 def"); end endcase
  case (64'h0) (!((-2) + L64)): begin : h7 initial $display("@3_7 hit"); end default: begin : d7 initial $display("@3_7 def"); end endcase
  case (0) (!((-2) + L64)): begin : h8 initial $display("@3_8 hit"); end default: begin : d8 initial $display("@3_8 def"); end endcase
  case (64'hfffffffffffffffd) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h9 initial $display("@3_9 hit"); end default: begin : d9 initial $display("@3_9 def"); end endcase
  case (64'shfffffffffffffffd) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h10 initial $display("@3_10 hit"); end default: begin : d10 initial $display("@3_10 def"); end endcase
  case (64'hfffffffffffffffd) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h11 initial $display("@3_11 hit"); end default: begin : d11 initial $display("@3_11 def"); end endcase
  case ((-3)) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h12 initial $display("@3_12 hit"); end default: begin : d12 initial $display("@3_12 def"); end endcase
  case (64'hfffffffffffffffd) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h13 initial $display("@3_13 hit"); end default: begin : d13 initial $display("@3_13 def"); end endcase
  case (18446744073709551613) (P8 ? (-3) : 64'sh1c8c5a917225eaa): begin : h14 initial $display("@3_14 hit"); end default: begin : d14 initial $display("@3_14 def"); end endcase
  case (3'h6) (-3'h2): begin : h15 initial $display("@3_15 hit"); end default: begin : d15 initial $display("@3_15 def"); end endcase
  case (3'sh6) (-3'h2): begin : h16 initial $display("@3_16 hit"); end default: begin : d16 initial $display("@3_16 def"); end endcase
  case (6'h6) (-3'h2): begin : h17 initial $display("@3_17 hit"); end default: begin : d17 initial $display("@3_17 def"); end endcase
  case ((-2)) (-3'h2): begin : h18 initial $display("@3_18 hit"); end default: begin : d18 initial $display("@3_18 def"); end endcase
  case (64'hfffffffffffffffe) (-3'h2): begin : h19 initial $display("@3_19 hit"); end default: begin : d19 initial $display("@3_19 def"); end endcase
  case (6) (-3'h2): begin : h20 initial $display("@3_20 hit"); end default: begin : d20 initial $display("@3_20 def"); end endcase
  case (64'h556dbbef4fabcad8) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h21 initial $display("@3_21 hit"); end default: begin : d21 initial $display("@3_21 def"); end endcase
  case (64'sh556dbbef4fabcad8) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h22 initial $display("@3_22 hit"); end default: begin : d22 initial $display("@3_22 def"); end endcase
  case (64'h556dbbef4fabcad8) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h23 initial $display("@3_23 hit"); end default: begin : d23 initial $display("@3_23 def"); end endcase
  case (6155782902193572568) (((-L64) * (P4 ? 3'h7 : 4'sh0)) ^ (~(64'haa924410b0543514 ^ P4))): begin : h24 initial $display("@3_24 hit"); end default: begin : d24 initial $display("@3_24 def"); end endcase
endmodule
