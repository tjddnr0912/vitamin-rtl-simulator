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
  case ((-1)) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h0 initial $display("@Ra21_0 hit"); end default: begin : d0 initial $display("@Ra21_0 def"); end endcase
  case (1) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h1 initial $display("@Ra21_1 hit"); end default: begin : d1 initial $display("@Ra21_1 def"); end endcase
  case (1'h1) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h2 initial $display("@Ra21_2 hit"); end default: begin : d2 initial $display("@Ra21_2 def"); end endcase
  case (1'sh1) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h3 initial $display("@Ra21_3 hit"); end default: begin : d3 initial $display("@Ra21_3 def"); end endcase
  case (4'h1) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h4 initial $display("@Ra21_4 hit"); end default: begin : d4 initial $display("@Ra21_4 def"); end endcase
  case (64'hffffffffffffffff) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h5 initial $display("@Ra21_5 hit"); end default: begin : d5 initial $display("@Ra21_5 def"); end endcase
  case ((-1)) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h6 initial $display("@Ra21_6 hit"); end default: begin : d6 initial $display("@Ra21_6 def"); end endcase
  case (1) $unsigned(((31'sh4cbfbef8 + U32) && {8'sh9C, 4'(63'sh2e20c682a3151d0c)})): begin : h7 initial $display("@Ra21_7 hit"); end default: begin : d7 initial $display("@Ra21_7 def"); end endcase
  case (8'he8) (-8'h18): begin : h8 initial $display("@Ra21_8 hit"); end default: begin : d8 initial $display("@Ra21_8 def"); end endcase
  case (8'she8) (-8'h18): begin : h9 initial $display("@Ra21_9 hit"); end default: begin : d9 initial $display("@Ra21_9 def"); end endcase
  case (11'he8) (-8'h18): begin : h10 initial $display("@Ra21_10 hit"); end default: begin : d10 initial $display("@Ra21_10 def"); end endcase
  case (64'hffffffffffffffe8) (-8'h18): begin : h11 initial $display("@Ra21_11 hit"); end default: begin : d11 initial $display("@Ra21_11 def"); end endcase
  case ((-24)) (-8'h18): begin : h12 initial $display("@Ra21_12 hit"); end default: begin : d12 initial $display("@Ra21_12 def"); end endcase
  case (232) (-8'h18): begin : h13 initial $display("@Ra21_13 hit"); end default: begin : d13 initial $display("@Ra21_13 def"); end endcase
  case (6'h24) {2{3'({65'((!S65)), S8})}}: begin : h14 initial $display("@Ra21_14 hit"); end default: begin : d14 initial $display("@Ra21_14 def"); end endcase
  case (6'sh24) {2{3'({65'((!S65)), S8})}}: begin : h15 initial $display("@Ra21_15 hit"); end default: begin : d15 initial $display("@Ra21_15 def"); end endcase
  case (9'h24) {2{3'({65'((!S65)), S8})}}: begin : h16 initial $display("@Ra21_16 hit"); end default: begin : d16 initial $display("@Ra21_16 def"); end endcase
  case (64'hffffffffffffffe4) {2{3'({65'((!S65)), S8})}}: begin : h17 initial $display("@Ra21_17 hit"); end default: begin : d17 initial $display("@Ra21_17 def"); end endcase
  case ((-28)) {2{3'({65'((!S65)), S8})}}: begin : h18 initial $display("@Ra21_18 hit"); end default: begin : d18 initial $display("@Ra21_18 def"); end endcase
  case (36) {2{3'({65'((!S65)), S8})}}: begin : h19 initial $display("@Ra21_19 hit"); end default: begin : d19 initial $display("@Ra21_19 def"); end endcase
  case (1'h1) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h20 initial $display("@Ra21_20 hit"); end default: begin : d20 initial $display("@Ra21_20 def"); end endcase
  case (1'sh1) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h21 initial $display("@Ra21_21 hit"); end default: begin : d21 initial $display("@Ra21_21 def"); end endcase
  case (4'h1) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h22 initial $display("@Ra21_22 hit"); end default: begin : d22 initial $display("@Ra21_22 def"); end endcase
  case (64'hffffffffffffffff) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h23 initial $display("@Ra21_23 hit"); end default: begin : d23 initial $display("@Ra21_23 def"); end endcase
  case ((-1)) ((5'sh1a % 5'hb) || {2{4'(2)}}): begin : h24 initial $display("@Ra21_24 hit"); end default: begin : d24 initial $display("@Ra21_24 def"); end endcase
endmodule
