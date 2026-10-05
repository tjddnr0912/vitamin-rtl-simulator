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
  case (64'h0) $signed((32'hfe85dfb1 < 40)): begin : h0 initial $display("@Ra20_0 hit"); end default: begin : d0 initial $display("@Ra20_0 def"); end endcase
  case (0) $signed((32'hfe85dfb1 < 40)): begin : h1 initial $display("@Ra20_1 hit"); end default: begin : d1 initial $display("@Ra20_1 def"); end endcase
  case (0) $signed((32'hfe85dfb1 < 40)): begin : h2 initial $display("@Ra20_2 hit"); end default: begin : d2 initial $display("@Ra20_2 def"); end endcase
  case (1'h0) ($signed(3'sh5) >= (L64 | (-7))): begin : h3 initial $display("@Ra20_3 hit"); end default: begin : d3 initial $display("@Ra20_3 def"); end endcase
  case (1'sh0) ($signed(3'sh5) >= (L64 | (-7))): begin : h4 initial $display("@Ra20_4 hit"); end default: begin : d4 initial $display("@Ra20_4 def"); end endcase
  case (4'h0) ($signed(3'sh5) >= (L64 | (-7))): begin : h5 initial $display("@Ra20_5 hit"); end default: begin : d5 initial $display("@Ra20_5 def"); end endcase
  case (64'h0) ($signed(3'sh5) >= (L64 | (-7))): begin : h6 initial $display("@Ra20_6 hit"); end default: begin : d6 initial $display("@Ra20_6 def"); end endcase
  case (0) ($signed(3'sh5) >= (L64 | (-7))): begin : h7 initial $display("@Ra20_7 hit"); end default: begin : d7 initial $display("@Ra20_7 def"); end endcase
  case (0) ($signed(3'sh5) >= (L64 | (-7))): begin : h8 initial $display("@Ra20_8 hit"); end default: begin : d8 initial $display("@Ra20_8 def"); end endcase
  case (31'h5c639866) 31'sh5c639866: begin : h9 initial $display("@Ra20_9 hit"); end default: begin : d9 initial $display("@Ra20_9 def"); end endcase
  case (31'sh5c639866) 31'sh5c639866: begin : h10 initial $display("@Ra20_10 hit"); end default: begin : d10 initial $display("@Ra20_10 def"); end endcase
  case (34'h5c639866) 31'sh5c639866: begin : h11 initial $display("@Ra20_11 hit"); end default: begin : d11 initial $display("@Ra20_11 def"); end endcase
  case (64'hffffffffdc639866) 31'sh5c639866: begin : h12 initial $display("@Ra20_12 hit"); end default: begin : d12 initial $display("@Ra20_12 def"); end endcase
  case ((-597452698)) 31'sh5c639866: begin : h13 initial $display("@Ra20_13 hit"); end default: begin : d13 initial $display("@Ra20_13 def"); end endcase
  case (1550030950) 31'sh5c639866: begin : h14 initial $display("@Ra20_14 hit"); end default: begin : d14 initial $display("@Ra20_14 def"); end endcase
  case (1'h0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h15 initial $display("@Ra20_15 hit"); end default: begin : d15 initial $display("@Ra20_15 def"); end endcase
  case (1'sh0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h16 initial $display("@Ra20_16 hit"); end default: begin : d16 initial $display("@Ra20_16 def"); end endcase
  case (4'h0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h17 initial $display("@Ra20_17 hit"); end default: begin : d17 initial $display("@Ra20_17 def"); end endcase
  case (64'h0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h18 initial $display("@Ra20_18 hit"); end default: begin : d18 initial $display("@Ra20_18 def"); end endcase
  case (0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h19 initial $display("@Ra20_19 hit"); end default: begin : d19 initial $display("@Ra20_19 def"); end endcase
  case (0) ((16'h15de == I) < (!65'hd4d1e96987d88917)): begin : h20 initial $display("@Ra20_20 hit"); end default: begin : d20 initial $display("@Ra20_20 def"); end endcase
  case (1'h1) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h21 initial $display("@Ra20_21 hit"); end default: begin : d21 initial $display("@Ra20_21 def"); end endcase
  case (1'sh1) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h22 initial $display("@Ra20_22 hit"); end default: begin : d22 initial $display("@Ra20_22 def"); end endcase
  case (4'h1) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h23 initial $display("@Ra20_23 hit"); end default: begin : d23 initial $display("@Ra20_23 def"); end endcase
  case (64'hffffffffffffffff) ((4'hc * 32'hd6bbcb67) || {33'(63'hf28789a8e18a929), 8'sh9C}): begin : h24 initial $display("@Ra20_24 hit"); end default: begin : d24 initial $display("@Ra20_24 def"); end endcase
endmodule
