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
  case (1'sh1) $unsigned(((S4 >>> 2) && (S65 >> 4))): begin : h0 initial $display("@Ra12_0 hit"); end default: begin : d0 initial $display("@Ra12_0 def"); end endcase
  case (4'h1) $unsigned(((S4 >>> 2) && (S65 >> 4))): begin : h1 initial $display("@Ra12_1 hit"); end default: begin : d1 initial $display("@Ra12_1 def"); end endcase
  case (64'hffffffffffffffff) $unsigned(((S4 >>> 2) && (S65 >> 4))): begin : h2 initial $display("@Ra12_2 hit"); end default: begin : d2 initial $display("@Ra12_2 def"); end endcase
  case ((-1)) $unsigned(((S4 >>> 2) && (S65 >> 4))): begin : h3 initial $display("@Ra12_3 hit"); end default: begin : d3 initial $display("@Ra12_3 def"); end endcase
  case (1) $unsigned(((S4 >>> 2) && (S65 >> 4))): begin : h4 initial $display("@Ra12_4 hit"); end default: begin : d4 initial $display("@Ra12_4 def"); end endcase
  case (6'h13) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h5 initial $display("@Ra12_5 hit"); end default: begin : d5 initial $display("@Ra12_5 def"); end endcase
  case (6'sh13) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h6 initial $display("@Ra12_6 hit"); end default: begin : d6 initial $display("@Ra12_6 def"); end endcase
  case (9'h13) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h7 initial $display("@Ra12_7 hit"); end default: begin : d7 initial $display("@Ra12_7 def"); end endcase
  case (64'h13) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h8 initial $display("@Ra12_8 hit"); end default: begin : d8 initial $display("@Ra12_8 def"); end endcase
  case (19) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h9 initial $display("@Ra12_9 hit"); end default: begin : d9 initial $display("@Ra12_9 def"); end endcase
  case (19) ({3'(S8), 3'(4'sh6)} >> (S4 && 2'sh3)): begin : h10 initial $display("@Ra12_10 hit"); end default: begin : d10 initial $display("@Ra12_10 def"); end endcase
  case (32'hfffffffb) I: begin : h11 initial $display("@Ra12_11 hit"); end default: begin : d11 initial $display("@Ra12_11 def"); end endcase
  case (32'shfffffffb) I: begin : h12 initial $display("@Ra12_12 hit"); end default: begin : d12 initial $display("@Ra12_12 def"); end endcase
  case (35'hfffffffb) I: begin : h13 initial $display("@Ra12_13 hit"); end default: begin : d13 initial $display("@Ra12_13 def"); end endcase
  case (64'hfffffffffffffffb) I: begin : h14 initial $display("@Ra12_14 hit"); end default: begin : d14 initial $display("@Ra12_14 def"); end endcase
  case ((-5)) I: begin : h15 initial $display("@Ra12_15 hit"); end default: begin : d15 initial $display("@Ra12_15 def"); end endcase
  case (8'hd3) (-$signed(8'h2d)): begin : h16 initial $display("@Ra12_16 hit"); end default: begin : d16 initial $display("@Ra12_16 def"); end endcase
  case (8'shd3) (-$signed(8'h2d)): begin : h17 initial $display("@Ra12_17 hit"); end default: begin : d17 initial $display("@Ra12_17 def"); end endcase
  case (11'hd3) (-$signed(8'h2d)): begin : h18 initial $display("@Ra12_18 hit"); end default: begin : d18 initial $display("@Ra12_18 def"); end endcase
  case (64'hffffffffffffffd3) (-$signed(8'h2d)): begin : h19 initial $display("@Ra12_19 hit"); end default: begin : d19 initial $display("@Ra12_19 def"); end endcase
  case ((-45)) (-$signed(8'h2d)): begin : h20 initial $display("@Ra12_20 hit"); end default: begin : d20 initial $display("@Ra12_20 def"); end endcase
  case (211) (-$signed(8'h2d)): begin : h21 initial $display("@Ra12_21 hit"); end default: begin : d21 initial $display("@Ra12_21 def"); end endcase
  case (8'hdd) {2{S4}}: begin : h22 initial $display("@Ra12_22 hit"); end default: begin : d22 initial $display("@Ra12_22 def"); end endcase
  case (8'shdd) {2{S4}}: begin : h23 initial $display("@Ra12_23 hit"); end default: begin : d23 initial $display("@Ra12_23 def"); end endcase
  case (11'hdd) {2{S4}}: begin : h24 initial $display("@Ra12_24 hit"); end default: begin : d24 initial $display("@Ra12_24 def"); end endcase
endmodule
