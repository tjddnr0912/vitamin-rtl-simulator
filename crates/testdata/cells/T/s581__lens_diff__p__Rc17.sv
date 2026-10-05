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
  case (37'h1b00000001) {S4, 33'h1_0000_0001}: begin : h0 initial $display("@Rc17_0 hit"); end default: begin : d0 initial $display("@Rc17_0 def"); end endcase
  case (37'sh1b00000001) {S4, 33'h1_0000_0001}: begin : h1 initial $display("@Rc17_1 hit"); end default: begin : d1 initial $display("@Rc17_1 def"); end endcase
  case (40'h1b00000001) {S4, 33'h1_0000_0001}: begin : h2 initial $display("@Rc17_2 hit"); end default: begin : d2 initial $display("@Rc17_2 def"); end endcase
  case (64'hfffffffb00000001) {S4, 33'h1_0000_0001}: begin : h3 initial $display("@Rc17_3 hit"); end default: begin : d3 initial $display("@Rc17_3 def"); end endcase
  case (3'h6) $signed(3'h6): begin : h4 initial $display("@Rc17_4 hit"); end default: begin : d4 initial $display("@Rc17_4 def"); end endcase
  case (3'sh6) $signed(3'h6): begin : h5 initial $display("@Rc17_5 hit"); end default: begin : d5 initial $display("@Rc17_5 def"); end endcase
  case (6'h6) $signed(3'h6): begin : h6 initial $display("@Rc17_6 hit"); end default: begin : d6 initial $display("@Rc17_6 def"); end endcase
  case (64'hfffffffffffffffe) $signed(3'h6): begin : h7 initial $display("@Rc17_7 hit"); end default: begin : d7 initial $display("@Rc17_7 def"); end endcase
  case ((-2)) $signed(3'h6): begin : h8 initial $display("@Rc17_8 hit"); end default: begin : d8 initial $display("@Rc17_8 def"); end endcase
  case (6) $signed(3'h6): begin : h9 initial $display("@Rc17_9 hit"); end default: begin : d9 initial $display("@Rc17_9 def"); end endcase
  case (8'h9c) $unsigned(S8): begin : h10 initial $display("@Rc17_10 hit"); end default: begin : d10 initial $display("@Rc17_10 def"); end endcase
  case (8'sh9c) $unsigned(S8): begin : h11 initial $display("@Rc17_11 hit"); end default: begin : d11 initial $display("@Rc17_11 def"); end endcase
  case (11'h9c) $unsigned(S8): begin : h12 initial $display("@Rc17_12 hit"); end default: begin : d12 initial $display("@Rc17_12 def"); end endcase
  case (64'hffffffffffffff9c) $unsigned(S8): begin : h13 initial $display("@Rc17_13 hit"); end default: begin : d13 initial $display("@Rc17_13 def"); end endcase
  case ((-100)) $unsigned(S8): begin : h14 initial $display("@Rc17_14 hit"); end default: begin : d14 initial $display("@Rc17_14 def"); end endcase
  case (156) $unsigned(S8): begin : h15 initial $display("@Rc17_15 hit"); end default: begin : d15 initial $display("@Rc17_15 def"); end endcase
  case (3'h6) 3'h6: begin : h16 initial $display("@Rc17_16 hit"); end default: begin : d16 initial $display("@Rc17_16 def"); end endcase
  case (3'sh6) 3'h6: begin : h17 initial $display("@Rc17_17 hit"); end default: begin : d17 initial $display("@Rc17_17 def"); end endcase
  case (6'h6) 3'h6: begin : h18 initial $display("@Rc17_18 hit"); end default: begin : d18 initial $display("@Rc17_18 def"); end endcase
  case (64'hfffffffffffffffe) 3'h6: begin : h19 initial $display("@Rc17_19 hit"); end default: begin : d19 initial $display("@Rc17_19 def"); end endcase
  case ((-2)) 3'h6: begin : h20 initial $display("@Rc17_20 hit"); end default: begin : d20 initial $display("@Rc17_20 def"); end endcase
  case (6) 3'h6: begin : h21 initial $display("@Rc17_21 hit"); end default: begin : d21 initial $display("@Rc17_21 def"); end endcase
  case (16'hf0f0) $signed({2{P8}}): begin : h22 initial $display("@Rc17_22 hit"); end default: begin : d22 initial $display("@Rc17_22 def"); end endcase
  case (16'shf0f0) $signed({2{P8}}): begin : h23 initial $display("@Rc17_23 hit"); end default: begin : d23 initial $display("@Rc17_23 def"); end endcase
  case (19'hf0f0) $signed({2{P8}}): begin : h24 initial $display("@Rc17_24 hit"); end default: begin : d24 initial $display("@Rc17_24 def"); end endcase
endmodule
