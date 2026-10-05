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
  typedef logic signed [5:0] s6_t;
  localparam int W6 = 6;
  localparam logic [127:0] P128 = {64'hFFFF_FFFF_FFFF_FFFF, 64'h0123_4567_89AB_CDEF};
  case (64'h0) (!(-7)): begin : h0 initial $display("@Rc10_0 hit"); end default: begin : d0 initial $display("@Rc10_0 def"); end endcase
  case (0) (!(-7)): begin : h1 initial $display("@Rc10_1 hit"); end default: begin : d1 initial $display("@Rc10_1 def"); end endcase
  case (0) (!(-7)): begin : h2 initial $display("@Rc10_2 hit"); end default: begin : d2 initial $display("@Rc10_2 def"); end endcase
  case (3'h0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h3 initial $display("@Rc10_3 hit"); end default: begin : d3 initial $display("@Rc10_3 def"); end endcase
  case (3'sh0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h4 initial $display("@Rc10_4 hit"); end default: begin : d4 initial $display("@Rc10_4 def"); end endcase
  case (6'h0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h5 initial $display("@Rc10_5 hit"); end default: begin : d5 initial $display("@Rc10_5 def"); end endcase
  case (64'h0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h6 initial $display("@Rc10_6 hit"); end default: begin : d6 initial $display("@Rc10_6 def"); end endcase
  case (0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h7 initial $display("@Rc10_7 hit"); end default: begin : d7 initial $display("@Rc10_7 def"); end endcase
  case (0) ((63'sh701bc645862dd85 < 16'sh766b) ? 3'(P128) : (~|16'h15e7)): begin : h8 initial $display("@Rc10_8 hit"); end default: begin : d8 initial $display("@Rc10_8 def"); end endcase
  case (64'h9c) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h9 initial $display("@Rc10_9 hit"); end default: begin : d9 initial $display("@Rc10_9 def"); end endcase
  case (64'sh9c) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h10 initial $display("@Rc10_10 hit"); end default: begin : d10 initial $display("@Rc10_10 def"); end endcase
  case (64'h9c) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h11 initial $display("@Rc10_11 hit"); end default: begin : d11 initial $display("@Rc10_11 def"); end endcase
  case (64'h9c) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h12 initial $display("@Rc10_12 hit"); end default: begin : d12 initial $display("@Rc10_12 def"); end endcase
  case (156) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h13 initial $display("@Rc10_13 hit"); end default: begin : d13 initial $display("@Rc10_13 def"); end endcase
  case (156) (unsigned'(1'h0) ? (64'h5cdbef30dd50085 << 3) : {1'(16'sha3b4), S8}): begin : h14 initial $display("@Rc10_14 hit"); end default: begin : d14 initial $display("@Rc10_14 def"); end endcase
  case (4'ha) (UN << 0): begin : h15 initial $display("@Rc10_15 hit"); end default: begin : d15 initial $display("@Rc10_15 def"); end endcase
  case (4'sha) (UN << 0): begin : h16 initial $display("@Rc10_16 hit"); end default: begin : d16 initial $display("@Rc10_16 def"); end endcase
  case (7'ha) (UN << 0): begin : h17 initial $display("@Rc10_17 hit"); end default: begin : d17 initial $display("@Rc10_17 def"); end endcase
  case (64'hfffffffffffffffa) (UN << 0): begin : h18 initial $display("@Rc10_18 hit"); end default: begin : d18 initial $display("@Rc10_18 def"); end endcase
  case ((-6)) (UN << 0): begin : h19 initial $display("@Rc10_19 hit"); end default: begin : d19 initial $display("@Rc10_19 def"); end endcase
  case (10) (UN << 0): begin : h20 initial $display("@Rc10_20 hit"); end default: begin : d20 initial $display("@Rc10_20 def"); end endcase
  case (4'h1) $unsigned($unsigned(4'h1)): begin : h21 initial $display("@Rc10_21 hit"); end default: begin : d21 initial $display("@Rc10_21 def"); end endcase
  case (4'sh1) $unsigned($unsigned(4'h1)): begin : h22 initial $display("@Rc10_22 hit"); end default: begin : d22 initial $display("@Rc10_22 def"); end endcase
  case (7'h1) $unsigned($unsigned(4'h1)): begin : h23 initial $display("@Rc10_23 hit"); end default: begin : d23 initial $display("@Rc10_23 def"); end endcase
  case (64'h1) $unsigned($unsigned(4'h1)): begin : h24 initial $display("@Rc10_24 hit"); end default: begin : d24 initial $display("@Rc10_24 def"); end endcase
endmodule
