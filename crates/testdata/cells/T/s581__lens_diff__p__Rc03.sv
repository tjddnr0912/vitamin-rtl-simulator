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
  case (32'shffffffff) (-1): begin : h0 initial $display("@Rc3_0 hit"); end default: begin : d0 initial $display("@Rc3_0 def"); end endcase
  case (35'hffffffff) (-1): begin : h1 initial $display("@Rc3_1 hit"); end default: begin : d1 initial $display("@Rc3_1 def"); end endcase
  case (64'hffffffffffffffff) (-1): begin : h2 initial $display("@Rc3_2 hit"); end default: begin : d2 initial $display("@Rc3_2 def"); end endcase
  case ((-1)) (-1): begin : h3 initial $display("@Rc3_3 hit"); end default: begin : d3 initial $display("@Rc3_3 def"); end endcase
  case (1'h1) (~&65'sh1d8f595a072c10f11): begin : h4 initial $display("@Rc3_4 hit"); end default: begin : d4 initial $display("@Rc3_4 def"); end endcase
  case (1'sh1) (~&65'sh1d8f595a072c10f11): begin : h5 initial $display("@Rc3_5 hit"); end default: begin : d5 initial $display("@Rc3_5 def"); end endcase
  case (4'h1) (~&65'sh1d8f595a072c10f11): begin : h6 initial $display("@Rc3_6 hit"); end default: begin : d6 initial $display("@Rc3_6 def"); end endcase
  case (64'hffffffffffffffff) (~&65'sh1d8f595a072c10f11): begin : h7 initial $display("@Rc3_7 hit"); end default: begin : d7 initial $display("@Rc3_7 def"); end endcase
  case ((-1)) (~&65'sh1d8f595a072c10f11): begin : h8 initial $display("@Rc3_8 hit"); end default: begin : d8 initial $display("@Rc3_8 def"); end endcase
  case (1) (~&65'sh1d8f595a072c10f11): begin : h9 initial $display("@Rc3_9 hit"); end default: begin : d9 initial $display("@Rc3_9 def"); end endcase
  case (32'h0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h10 initial $display("@Rc3_10 hit"); end default: begin : d10 initial $display("@Rc3_10 def"); end endcase
  case (32'sh0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h11 initial $display("@Rc3_11 hit"); end default: begin : d11 initial $display("@Rc3_11 def"); end endcase
  case (35'h0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h12 initial $display("@Rc3_12 hit"); end default: begin : d12 initial $display("@Rc3_12 def"); end endcase
  case (64'h0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h13 initial $display("@Rc3_13 hit"); end default: begin : d13 initial $display("@Rc3_13 def"); end endcase
  case (0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h14 initial $display("@Rc3_14 hit"); end default: begin : d14 initial $display("@Rc3_14 def"); end endcase
  case (0) ($onehot0(8'sh9C) % (-signed'(U32))): begin : h15 initial $display("@Rc3_15 hit"); end default: begin : d15 initial $display("@Rc3_15 def"); end endcase
  case (8'hf0) P8: begin : h16 initial $display("@Rc3_16 hit"); end default: begin : d16 initial $display("@Rc3_16 def"); end endcase
  case (8'shf0) P8: begin : h17 initial $display("@Rc3_17 hit"); end default: begin : d17 initial $display("@Rc3_17 def"); end endcase
  case (11'hf0) P8: begin : h18 initial $display("@Rc3_18 hit"); end default: begin : d18 initial $display("@Rc3_18 def"); end endcase
  case (64'hfffffffffffffff0) P8: begin : h19 initial $display("@Rc3_19 hit"); end default: begin : d19 initial $display("@Rc3_19 def"); end endcase
  case ((-16)) P8: begin : h20 initial $display("@Rc3_20 hit"); end default: begin : d20 initial $display("@Rc3_20 def"); end endcase
  case (240) P8: begin : h21 initial $display("@Rc3_21 hit"); end default: begin : d21 initial $display("@Rc3_21 def"); end endcase
  case (1'h1) ({1'((-4)), P65} && (U32 && UN)): begin : h22 initial $display("@Rc3_22 hit"); end default: begin : d22 initial $display("@Rc3_22 def"); end endcase
  case (1'sh1) ({1'((-4)), P65} && (U32 && UN)): begin : h23 initial $display("@Rc3_23 hit"); end default: begin : d23 initial $display("@Rc3_23 def"); end endcase
  case (4'h1) ({1'((-4)), P65} && (U32 && UN)): begin : h24 initial $display("@Rc3_24 hit"); end default: begin : d24 initial $display("@Rc3_24 def"); end endcase
endmodule
