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
  case (64'hffffffffffffff9c) S8: begin : h0 initial $display("@Rc2_0 hit"); end default: begin : d0 initial $display("@Rc2_0 def"); end endcase
  case ((-100)) S8: begin : h1 initial $display("@Rc2_1 hit"); end default: begin : d1 initial $display("@Rc2_1 def"); end endcase
  case (156) S8: begin : h2 initial $display("@Rc2_2 hit"); end default: begin : d2 initial $display("@Rc2_2 def"); end endcase
  case (16'h0) {2{8'((W6 & 5'sh11))}}: begin : h3 initial $display("@Rc2_3 hit"); end default: begin : d3 initial $display("@Rc2_3 def"); end endcase
  case (16'sh0) {2{8'((W6 & 5'sh11))}}: begin : h4 initial $display("@Rc2_4 hit"); end default: begin : d4 initial $display("@Rc2_4 def"); end endcase
  case (19'h0) {2{8'((W6 & 5'sh11))}}: begin : h5 initial $display("@Rc2_5 hit"); end default: begin : d5 initial $display("@Rc2_5 def"); end endcase
  case (64'h0) {2{8'((W6 & 5'sh11))}}: begin : h6 initial $display("@Rc2_6 hit"); end default: begin : d6 initial $display("@Rc2_6 def"); end endcase
  case (0) {2{8'((W6 & 5'sh11))}}: begin : h7 initial $display("@Rc2_7 hit"); end default: begin : d7 initial $display("@Rc2_7 def"); end endcase
  case (0) {2{8'((W6 & 5'sh11))}}: begin : h8 initial $display("@Rc2_8 hit"); end default: begin : d8 initial $display("@Rc2_8 def"); end endcase
  case (64'hffffffff00000014) ((64'hae482612c9570a1c & 38) - (U32 >> 0)): begin : h9 initial $display("@Rc2_9 hit"); end default: begin : d9 initial $display("@Rc2_9 def"); end endcase
  case (64'shffffffff00000014) ((64'hae482612c9570a1c & 38) - (U32 >> 0)): begin : h10 initial $display("@Rc2_10 hit"); end default: begin : d10 initial $display("@Rc2_10 def"); end endcase
  case (64'hffffffff00000014) ((64'hae482612c9570a1c & 38) - (U32 >> 0)): begin : h11 initial $display("@Rc2_11 hit"); end default: begin : d11 initial $display("@Rc2_11 def"); end endcase
  case (64'hffffffff00000014) ((64'hae482612c9570a1c & 38) - (U32 >> 0)): begin : h12 initial $display("@Rc2_12 hit"); end default: begin : d12 initial $display("@Rc2_12 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h13 initial $display("@Rc2_13 hit"); end default: begin : d13 initial $display("@Rc2_13 def"); end endcase
  case (64'shfffffffffffffff7) L64: begin : h14 initial $display("@Rc2_14 hit"); end default: begin : d14 initial $display("@Rc2_14 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h15 initial $display("@Rc2_15 hit"); end default: begin : d15 initial $display("@Rc2_15 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h16 initial $display("@Rc2_16 hit"); end default: begin : d16 initial $display("@Rc2_16 def"); end endcase
  case ((-9)) L64: begin : h17 initial $display("@Rc2_17 hit"); end default: begin : d17 initial $display("@Rc2_17 def"); end endcase
  case (32'h5) $clog2(26): begin : h18 initial $display("@Rc2_18 hit"); end default: begin : d18 initial $display("@Rc2_18 def"); end endcase
  case (32'sh5) $clog2(26): begin : h19 initial $display("@Rc2_19 hit"); end default: begin : d19 initial $display("@Rc2_19 def"); end endcase
  case (35'h5) $clog2(26): begin : h20 initial $display("@Rc2_20 hit"); end default: begin : d20 initial $display("@Rc2_20 def"); end endcase
  case (64'h5) $clog2(26): begin : h21 initial $display("@Rc2_21 hit"); end default: begin : d21 initial $display("@Rc2_21 def"); end endcase
  case (5) $clog2(26): begin : h22 initial $display("@Rc2_22 hit"); end default: begin : d22 initial $display("@Rc2_22 def"); end endcase
  case (5) $clog2(26): begin : h23 initial $display("@Rc2_23 hit"); end default: begin : d23 initial $display("@Rc2_23 def"); end endcase
  case (32'hffffffff) (-1): begin : h24 initial $display("@Rc2_24 hit"); end default: begin : d24 initial $display("@Rc2_24 def"); end endcase
endmodule
