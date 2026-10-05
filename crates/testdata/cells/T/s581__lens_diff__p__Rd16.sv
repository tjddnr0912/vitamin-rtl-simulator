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
  case (33'sh88e80010) (33'sh98fa1459 & signed'(32'heeede012)): begin : h0 initial $display("@Rd16_0 hit"); end default: begin : d0 initial $display("@Rd16_0 def"); end endcase
  case (36'h88e80010) (33'sh98fa1459 & signed'(32'heeede012)): begin : h1 initial $display("@Rd16_1 hit"); end default: begin : d1 initial $display("@Rd16_1 def"); end endcase
  case (64'h88e80010) (33'sh98fa1459 & signed'(32'heeede012)): begin : h2 initial $display("@Rd16_2 hit"); end default: begin : d2 initial $display("@Rd16_2 def"); end endcase
  case (64'hfffffffffffffff7) unsigned'($signed(L64)): begin : h3 initial $display("@Rd16_3 hit"); end default: begin : d3 initial $display("@Rd16_3 def"); end endcase
  case (64'shfffffffffffffff7) unsigned'($signed(L64)): begin : h4 initial $display("@Rd16_4 hit"); end default: begin : d4 initial $display("@Rd16_4 def"); end endcase
  case (64'hfffffffffffffff7) unsigned'($signed(L64)): begin : h5 initial $display("@Rd16_5 hit"); end default: begin : d5 initial $display("@Rd16_5 def"); end endcase
  case (64'hfffffffffffffff7) unsigned'($signed(L64)): begin : h6 initial $display("@Rd16_6 hit"); end default: begin : d6 initial $display("@Rd16_6 def"); end endcase
  case ((-9)) unsigned'($signed(L64)): begin : h7 initial $display("@Rd16_7 hit"); end default: begin : d7 initial $display("@Rd16_7 def"); end endcase
  case (32'h7) (31 >>> 2): begin : h8 initial $display("@Rd16_8 hit"); end default: begin : d8 initial $display("@Rd16_8 def"); end endcase
  case (32'sh7) (31 >>> 2): begin : h9 initial $display("@Rd16_9 hit"); end default: begin : d9 initial $display("@Rd16_9 def"); end endcase
  case (35'h7) (31 >>> 2): begin : h10 initial $display("@Rd16_10 hit"); end default: begin : d10 initial $display("@Rd16_10 def"); end endcase
  case (64'h7) (31 >>> 2): begin : h11 initial $display("@Rd16_11 hit"); end default: begin : d11 initial $display("@Rd16_11 def"); end endcase
  case (7) (31 >>> 2): begin : h12 initial $display("@Rd16_12 hit"); end default: begin : d12 initial $display("@Rd16_12 def"); end endcase
  case (7) (31 >>> 2): begin : h13 initial $display("@Rd16_13 hit"); end default: begin : d13 initial $display("@Rd16_13 def"); end endcase
  case (1'h0) (~^16'h391f): begin : h14 initial $display("@Rd16_14 hit"); end default: begin : d14 initial $display("@Rd16_14 def"); end endcase
  case (1'sh0) (~^16'h391f): begin : h15 initial $display("@Rd16_15 hit"); end default: begin : d15 initial $display("@Rd16_15 def"); end endcase
  case (4'h0) (~^16'h391f): begin : h16 initial $display("@Rd16_16 hit"); end default: begin : d16 initial $display("@Rd16_16 def"); end endcase
  case (64'h0) (~^16'h391f): begin : h17 initial $display("@Rd16_17 hit"); end default: begin : d17 initial $display("@Rd16_17 def"); end endcase
  case (0) (~^16'h391f): begin : h18 initial $display("@Rd16_18 hit"); end default: begin : d18 initial $display("@Rd16_18 def"); end endcase
  case (0) (~^16'h391f): begin : h19 initial $display("@Rd16_19 hit"); end default: begin : d19 initial $display("@Rd16_19 def"); end endcase
  case (32'hfffffffb) (S65 ? I : 2'h1): begin : h20 initial $display("@Rd16_20 hit"); end default: begin : d20 initial $display("@Rd16_20 def"); end endcase
  case (32'shfffffffb) (S65 ? I : 2'h1): begin : h21 initial $display("@Rd16_21 hit"); end default: begin : d21 initial $display("@Rd16_21 def"); end endcase
  case (35'hfffffffb) (S65 ? I : 2'h1): begin : h22 initial $display("@Rd16_22 hit"); end default: begin : d22 initial $display("@Rd16_22 def"); end endcase
  case (64'hfffffffffffffffb) (S65 ? I : 2'h1): begin : h23 initial $display("@Rd16_23 hit"); end default: begin : d23 initial $display("@Rd16_23 def"); end endcase
  case ((-5)) (S65 ? I : 2'h1): begin : h24 initial $display("@Rd16_24 hit"); end default: begin : d24 initial $display("@Rd16_24 def"); end endcase
endmodule
