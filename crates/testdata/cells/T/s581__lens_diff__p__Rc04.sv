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
  case (64'hffffffffffffffff) ({1'((-4)), P65} && (U32 && UN)): begin : h0 initial $display("@Rc4_0 hit"); end default: begin : d0 initial $display("@Rc4_0 def"); end endcase
  case ((-1)) ({1'((-4)), P65} && (U32 && UN)): begin : h1 initial $display("@Rc4_1 hit"); end default: begin : d1 initial $display("@Rc4_1 def"); end endcase
  case (1) ({1'((-4)), P65} && (U32 && UN)): begin : h2 initial $display("@Rc4_2 hit"); end default: begin : d2 initial $display("@Rc4_2 def"); end endcase
  case (64'h6e52cb7a5704f1) (64'h6e52cb7a570441 | P8): begin : h3 initial $display("@Rc4_3 hit"); end default: begin : d3 initial $display("@Rc4_3 def"); end endcase
  case (64'sh6e52cb7a5704f1) (64'h6e52cb7a570441 | P8): begin : h4 initial $display("@Rc4_4 hit"); end default: begin : d4 initial $display("@Rc4_4 def"); end endcase
  case (64'h6e52cb7a5704f1) (64'h6e52cb7a570441 | P8): begin : h5 initial $display("@Rc4_5 hit"); end default: begin : d5 initial $display("@Rc4_5 def"); end endcase
  case (64'h6e52cb7a5704f1) (64'h6e52cb7a570441 | P8): begin : h6 initial $display("@Rc4_6 hit"); end default: begin : d6 initial $display("@Rc4_6 def"); end endcase
  case (6'ha) W6'(5'ha): begin : h7 initial $display("@Rc4_7 hit"); end default: begin : d7 initial $display("@Rc4_7 def"); end endcase
  case (6'sha) W6'(5'ha): begin : h8 initial $display("@Rc4_8 hit"); end default: begin : d8 initial $display("@Rc4_8 def"); end endcase
  case (9'ha) W6'(5'ha): begin : h9 initial $display("@Rc4_9 hit"); end default: begin : d9 initial $display("@Rc4_9 def"); end endcase
  case (64'ha) W6'(5'ha): begin : h10 initial $display("@Rc4_10 hit"); end default: begin : d10 initial $display("@Rc4_10 def"); end endcase
  case (10) W6'(5'ha): begin : h11 initial $display("@Rc4_11 hit"); end default: begin : d11 initial $display("@Rc4_11 def"); end endcase
  case (10) W6'(5'ha): begin : h12 initial $display("@Rc4_12 hit"); end default: begin : d12 initial $display("@Rc4_12 def"); end endcase
  case (32'h2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h13 initial $display("@Rc4_13 hit"); end default: begin : d13 initial $display("@Rc4_13 def"); end endcase
  case (32'sh2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h14 initial $display("@Rc4_14 hit"); end default: begin : d14 initial $display("@Rc4_14 def"); end endcase
  case (35'h2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h15 initial $display("@Rc4_15 hit"); end default: begin : d15 initial $display("@Rc4_15 def"); end endcase
  case (64'h2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h16 initial $display("@Rc4_16 hit"); end default: begin : d16 initial $display("@Rc4_16 def"); end endcase
  case (2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h17 initial $display("@Rc4_17 hit"); end default: begin : d17 initial $display("@Rc4_17 def"); end endcase
  case (2) $clog2(((4'h6 + (-2)) | $onehot0(S65))): begin : h18 initial $display("@Rc4_18 hit"); end default: begin : d18 initial $display("@Rc4_18 def"); end endcase
  case (7'h10) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h19 initial $display("@Rc4_19 hit"); end default: begin : d19 initial $display("@Rc4_19 def"); end endcase
  case (7'sh10) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h20 initial $display("@Rc4_20 hit"); end default: begin : d20 initial $display("@Rc4_20 def"); end endcase
  case (10'h10) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h21 initial $display("@Rc4_21 hit"); end default: begin : d21 initial $display("@Rc4_21 def"); end endcase
  case (64'h10) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h22 initial $display("@Rc4_22 hit"); end default: begin : d22 initial $display("@Rc4_22 def"); end endcase
  case (16) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h23 initial $display("@Rc4_23 hit"); end default: begin : d23 initial $display("@Rc4_23 def"); end endcase
  case (16) {3'($unsigned(2'h1)), 4'(32'h54d6660)}: begin : h24 initial $display("@Rc4_24 hit"); end default: begin : d24 initial $display("@Rc4_24 def"); end endcase
endmodule
