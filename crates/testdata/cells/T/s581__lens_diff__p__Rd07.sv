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
  case (0) signed'((&$onehot(4'hB))): begin : h0 initial $display("@Rd7_0 hit"); end default: begin : d0 initial $display("@Rd7_0 def"); end endcase
  case (0) signed'((&$onehot(4'hB))): begin : h1 initial $display("@Rd7_1 hit"); end default: begin : d1 initial $display("@Rd7_1 def"); end endcase
  case (8'h9c) signed'(S8): begin : h2 initial $display("@Rd7_2 hit"); end default: begin : d2 initial $display("@Rd7_2 def"); end endcase
  case (8'sh9c) signed'(S8): begin : h3 initial $display("@Rd7_3 hit"); end default: begin : d3 initial $display("@Rd7_3 def"); end endcase
  case (11'h9c) signed'(S8): begin : h4 initial $display("@Rd7_4 hit"); end default: begin : d4 initial $display("@Rd7_4 def"); end endcase
  case (64'hffffffffffffff9c) signed'(S8): begin : h5 initial $display("@Rd7_5 hit"); end default: begin : d5 initial $display("@Rd7_5 def"); end endcase
  case ((-100)) signed'(S8): begin : h6 initial $display("@Rd7_6 hit"); end default: begin : d6 initial $display("@Rd7_6 def"); end endcase
  case (156) signed'(S8): begin : h7 initial $display("@Rd7_7 hit"); end default: begin : d7 initial $display("@Rd7_7 def"); end endcase
  case (64'h8f0d067939b32a0) 64'h8f0d067939b32a0: begin : h8 initial $display("@Rd7_8 hit"); end default: begin : d8 initial $display("@Rd7_8 def"); end endcase
  case (64'sh8f0d067939b32a0) 64'h8f0d067939b32a0: begin : h9 initial $display("@Rd7_9 hit"); end default: begin : d9 initial $display("@Rd7_9 def"); end endcase
  case (64'h8f0d067939b32a0) 64'h8f0d067939b32a0: begin : h10 initial $display("@Rd7_10 hit"); end default: begin : d10 initial $display("@Rd7_10 def"); end endcase
  case (64'h8f0d067939b32a0) 64'h8f0d067939b32a0: begin : h11 initial $display("@Rd7_11 hit"); end default: begin : d11 initial $display("@Rd7_11 def"); end endcase
  case (32'hfffffff0) U32: begin : h12 initial $display("@Rd7_12 hit"); end default: begin : d12 initial $display("@Rd7_12 def"); end endcase
  case (32'shfffffff0) U32: begin : h13 initial $display("@Rd7_13 hit"); end default: begin : d13 initial $display("@Rd7_13 def"); end endcase
  case (35'hfffffff0) U32: begin : h14 initial $display("@Rd7_14 hit"); end default: begin : d14 initial $display("@Rd7_14 def"); end endcase
  case (64'hfffffffffffffff0) U32: begin : h15 initial $display("@Rd7_15 hit"); end default: begin : d15 initial $display("@Rd7_15 def"); end endcase
  case ((-16)) U32: begin : h16 initial $display("@Rd7_16 hit"); end default: begin : d16 initial $display("@Rd7_16 def"); end endcase
  case (6'h3f) 6'(33'((3'sh7 >> 0))): begin : h17 initial $display("@Rd7_17 hit"); end default: begin : d17 initial $display("@Rd7_17 def"); end endcase
  case (6'sh3f) 6'(33'((3'sh7 >> 0))): begin : h18 initial $display("@Rd7_18 hit"); end default: begin : d18 initial $display("@Rd7_18 def"); end endcase
  case (9'h3f) 6'(33'((3'sh7 >> 0))): begin : h19 initial $display("@Rd7_19 hit"); end default: begin : d19 initial $display("@Rd7_19 def"); end endcase
  case (64'hffffffffffffffff) 6'(33'((3'sh7 >> 0))): begin : h20 initial $display("@Rd7_20 hit"); end default: begin : d20 initial $display("@Rd7_20 def"); end endcase
  case ((-1)) 6'(33'((3'sh7 >> 0))): begin : h21 initial $display("@Rd7_21 hit"); end default: begin : d21 initial $display("@Rd7_21 def"); end endcase
  case (63) 6'(33'((3'sh7 >> 0))): begin : h22 initial $display("@Rd7_22 hit"); end default: begin : d22 initial $display("@Rd7_22 def"); end endcase
  case (32'h6) W6: begin : h23 initial $display("@Rd7_23 hit"); end default: begin : d23 initial $display("@Rd7_23 def"); end endcase
  case (32'sh6) W6: begin : h24 initial $display("@Rd7_24 hit"); end default: begin : d24 initial $display("@Rd7_24 def"); end endcase
endmodule
