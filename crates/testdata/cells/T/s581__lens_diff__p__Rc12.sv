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
  case (48) W6'((P8 >>> 0)): begin : h0 initial $display("@Rc12_0 hit"); end default: begin : d0 initial $display("@Rc12_0 def"); end endcase
  case (8'h1) ((8'hb / 4'h1) & (|P8)): begin : h1 initial $display("@Rc12_1 hit"); end default: begin : d1 initial $display("@Rc12_1 def"); end endcase
  case (8'sh1) ((8'hb / 4'h1) & (|P8)): begin : h2 initial $display("@Rc12_2 hit"); end default: begin : d2 initial $display("@Rc12_2 def"); end endcase
  case (11'h1) ((8'hb / 4'h1) & (|P8)): begin : h3 initial $display("@Rc12_3 hit"); end default: begin : d3 initial $display("@Rc12_3 def"); end endcase
  case (64'h1) ((8'hb / 4'h1) & (|P8)): begin : h4 initial $display("@Rc12_4 hit"); end default: begin : d4 initial $display("@Rc12_4 def"); end endcase
  case (1) ((8'hb / 4'h1) & (|P8)): begin : h5 initial $display("@Rc12_5 hit"); end default: begin : d5 initial $display("@Rc12_5 def"); end endcase
  case (1) ((8'hb / 4'h1) & (|P8)): begin : h6 initial $display("@Rc12_6 hit"); end default: begin : d6 initial $display("@Rc12_6 def"); end endcase
  case (1'h1) (I || P65): begin : h7 initial $display("@Rc12_7 hit"); end default: begin : d7 initial $display("@Rc12_7 def"); end endcase
  case (1'sh1) (I || P65): begin : h8 initial $display("@Rc12_8 hit"); end default: begin : d8 initial $display("@Rc12_8 def"); end endcase
  case (4'h1) (I || P65): begin : h9 initial $display("@Rc12_9 hit"); end default: begin : d9 initial $display("@Rc12_9 def"); end endcase
  case (64'hffffffffffffffff) (I || P65): begin : h10 initial $display("@Rc12_10 hit"); end default: begin : d10 initial $display("@Rc12_10 def"); end endcase
  case ((-1)) (I || P65): begin : h11 initial $display("@Rc12_11 hit"); end default: begin : d11 initial $display("@Rc12_11 def"); end endcase
  case (1) (I || P65): begin : h12 initial $display("@Rc12_12 hit"); end default: begin : d12 initial $display("@Rc12_12 def"); end endcase
  case (32'hfffffffb) (((-5) ? I : P4) ** 1): begin : h13 initial $display("@Rc12_13 hit"); end default: begin : d13 initial $display("@Rc12_13 def"); end endcase
  case (32'shfffffffb) (((-5) ? I : P4) ** 1): begin : h14 initial $display("@Rc12_14 hit"); end default: begin : d14 initial $display("@Rc12_14 def"); end endcase
  case (35'hfffffffb) (((-5) ? I : P4) ** 1): begin : h15 initial $display("@Rc12_15 hit"); end default: begin : d15 initial $display("@Rc12_15 def"); end endcase
  case (64'hfffffffffffffffb) (((-5) ? I : P4) ** 1): begin : h16 initial $display("@Rc12_16 hit"); end default: begin : d16 initial $display("@Rc12_16 def"); end endcase
  case ((-5)) (((-5) ? I : P4) ** 1): begin : h17 initial $display("@Rc12_17 hit"); end default: begin : d17 initial $display("@Rc12_17 def"); end endcase
  case (1'h1) (^(-8)): begin : h18 initial $display("@Rc12_18 hit"); end default: begin : d18 initial $display("@Rc12_18 def"); end endcase
  case (1'sh1) (^(-8)): begin : h19 initial $display("@Rc12_19 hit"); end default: begin : d19 initial $display("@Rc12_19 def"); end endcase
  case (4'h1) (^(-8)): begin : h20 initial $display("@Rc12_20 hit"); end default: begin : d20 initial $display("@Rc12_20 def"); end endcase
  case (64'hffffffffffffffff) (^(-8)): begin : h21 initial $display("@Rc12_21 hit"); end default: begin : d21 initial $display("@Rc12_21 def"); end endcase
  case ((-1)) (^(-8)): begin : h22 initial $display("@Rc12_22 hit"); end default: begin : d22 initial $display("@Rc12_22 def"); end endcase
  case (1) (^(-8)): begin : h23 initial $display("@Rc12_23 hit"); end default: begin : d23 initial $display("@Rc12_23 def"); end endcase
  case (1'h0) $signed((~|UN)): begin : h24 initial $display("@Rc12_24 hit"); end default: begin : d24 initial $display("@Rc12_24 def"); end endcase
endmodule
