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
  case (33'sh1b6972a80) 33'h1b6972a80: begin : h0 initial $display("@Rc22_0 hit"); end default: begin : d0 initial $display("@Rc22_0 def"); end endcase
  case (36'h1b6972a80) 33'h1b6972a80: begin : h1 initial $display("@Rc22_1 hit"); end default: begin : d1 initial $display("@Rc22_1 def"); end endcase
  case (64'hffffffffb6972a80) 33'h1b6972a80: begin : h2 initial $display("@Rc22_2 hit"); end default: begin : d2 initial $display("@Rc22_2 def"); end endcase
  case ((-1231607168)) 33'h1b6972a80: begin : h3 initial $display("@Rc22_3 hit"); end default: begin : d3 initial $display("@Rc22_3 def"); end endcase
  case (64'hedd8ceec7dfe14a5) ((P128 ? 64'hedd8ceecf6309844 : (-3)) - (33'h1e0ca0e7f >> 2)): begin : h4 initial $display("@Rc22_4 hit"); end default: begin : d4 initial $display("@Rc22_4 def"); end endcase
  case (64'shedd8ceec7dfe14a5) ((P128 ? 64'hedd8ceecf6309844 : (-3)) - (33'h1e0ca0e7f >> 2)): begin : h5 initial $display("@Rc22_5 hit"); end default: begin : d5 initial $display("@Rc22_5 def"); end endcase
  case (64'hedd8ceec7dfe14a5) ((P128 ? 64'hedd8ceecf6309844 : (-3)) - (33'h1e0ca0e7f >> 2)): begin : h6 initial $display("@Rc22_6 hit"); end default: begin : d6 initial $display("@Rc22_6 def"); end endcase
  case (64'hedd8ceec7dfe14a5) ((P128 ? 64'hedd8ceecf6309844 : (-3)) - (33'h1e0ca0e7f >> 2)): begin : h7 initial $display("@Rc22_7 hit"); end default: begin : d7 initial $display("@Rc22_7 def"); end endcase
  case (32'hfffffff2) (-(28 >> 1)): begin : h8 initial $display("@Rc22_8 hit"); end default: begin : d8 initial $display("@Rc22_8 def"); end endcase
  case (32'shfffffff2) (-(28 >> 1)): begin : h9 initial $display("@Rc22_9 hit"); end default: begin : d9 initial $display("@Rc22_9 def"); end endcase
  case (35'hfffffff2) (-(28 >> 1)): begin : h10 initial $display("@Rc22_10 hit"); end default: begin : d10 initial $display("@Rc22_10 def"); end endcase
  case (64'hfffffffffffffff2) (-(28 >> 1)): begin : h11 initial $display("@Rc22_11 hit"); end default: begin : d11 initial $display("@Rc22_11 def"); end endcase
  case ((-14)) (-(28 >> 1)): begin : h12 initial $display("@Rc22_12 hit"); end default: begin : d12 initial $display("@Rc22_12 def"); end endcase
  case (4'h9) $unsigned($unsigned(4'sh9)): begin : h13 initial $display("@Rc22_13 hit"); end default: begin : d13 initial $display("@Rc22_13 def"); end endcase
  case (4'sh9) $unsigned($unsigned(4'sh9)): begin : h14 initial $display("@Rc22_14 hit"); end default: begin : d14 initial $display("@Rc22_14 def"); end endcase
  case (7'h9) $unsigned($unsigned(4'sh9)): begin : h15 initial $display("@Rc22_15 hit"); end default: begin : d15 initial $display("@Rc22_15 def"); end endcase
  case (64'hfffffffffffffff9) $unsigned($unsigned(4'sh9)): begin : h16 initial $display("@Rc22_16 hit"); end default: begin : d16 initial $display("@Rc22_16 def"); end endcase
  case ((-7)) $unsigned($unsigned(4'sh9)): begin : h17 initial $display("@Rc22_17 hit"); end default: begin : d17 initial $display("@Rc22_17 def"); end endcase
  case (9) $unsigned($unsigned(4'sh9)): begin : h18 initial $display("@Rc22_18 hit"); end default: begin : d18 initial $display("@Rc22_18 def"); end endcase
  case (6'h1d) s6_t'({33'((|16'h628d)), S4}): begin : h19 initial $display("@Rc22_19 hit"); end default: begin : d19 initial $display("@Rc22_19 def"); end endcase
  case (6'sh1d) s6_t'({33'((|16'h628d)), S4}): begin : h20 initial $display("@Rc22_20 hit"); end default: begin : d20 initial $display("@Rc22_20 def"); end endcase
  case (9'h1d) s6_t'({33'((|16'h628d)), S4}): begin : h21 initial $display("@Rc22_21 hit"); end default: begin : d21 initial $display("@Rc22_21 def"); end endcase
  case (64'h1d) s6_t'({33'((|16'h628d)), S4}): begin : h22 initial $display("@Rc22_22 hit"); end default: begin : d22 initial $display("@Rc22_22 def"); end endcase
  case (29) s6_t'({33'((|16'h628d)), S4}): begin : h23 initial $display("@Rc22_23 hit"); end default: begin : d23 initial $display("@Rc22_23 def"); end endcase
  case (29) s6_t'({33'((|16'h628d)), S4}): begin : h24 initial $display("@Rc22_24 hit"); end default: begin : d24 initial $display("@Rc22_24 def"); end endcase
endmodule
