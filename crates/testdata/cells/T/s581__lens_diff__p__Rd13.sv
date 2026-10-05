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
  case (1'h0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h0 initial $display("@Rd13_0 hit"); end default: begin : d0 initial $display("@Rd13_0 def"); end endcase
  case (1'sh0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h1 initial $display("@Rd13_1 hit"); end default: begin : d1 initial $display("@Rd13_1 def"); end endcase
  case (4'h0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h2 initial $display("@Rd13_2 hit"); end default: begin : d2 initial $display("@Rd13_2 def"); end endcase
  case (64'h0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h3 initial $display("@Rd13_3 hit"); end default: begin : d3 initial $display("@Rd13_3 def"); end endcase
  case (0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h4 initial $display("@Rd13_4 hit"); end default: begin : d4 initial $display("@Rd13_4 def"); end endcase
  case (0) signed'(((S65 / W6) >= $unsigned(L64))): begin : h5 initial $display("@Rd13_5 hit"); end default: begin : d5 initial $display("@Rd13_5 def"); end endcase
  case (64'h5d7237fbfffffffa) ($unsigned((-8)) | unsigned'(64'sh5d7237fbdf56e5aa)): begin : h6 initial $display("@Rd13_6 hit"); end default: begin : d6 initial $display("@Rd13_6 def"); end endcase
  case (64'sh5d7237fbfffffffa) ($unsigned((-8)) | unsigned'(64'sh5d7237fbdf56e5aa)): begin : h7 initial $display("@Rd13_7 hit"); end default: begin : d7 initial $display("@Rd13_7 def"); end endcase
  case (64'h5d7237fbfffffffa) ($unsigned((-8)) | unsigned'(64'sh5d7237fbdf56e5aa)): begin : h8 initial $display("@Rd13_8 hit"); end default: begin : d8 initial $display("@Rd13_8 def"); end endcase
  case (64'h5d7237fbfffffffa) ($unsigned((-8)) | unsigned'(64'sh5d7237fbdf56e5aa)): begin : h9 initial $display("@Rd13_9 hit"); end default: begin : d9 initial $display("@Rd13_9 def"); end endcase
  case (4'h1) $signed((S4 ** 0)): begin : h10 initial $display("@Rd13_10 hit"); end default: begin : d10 initial $display("@Rd13_10 def"); end endcase
  case (4'sh1) $signed((S4 ** 0)): begin : h11 initial $display("@Rd13_11 hit"); end default: begin : d11 initial $display("@Rd13_11 def"); end endcase
  case (7'h1) $signed((S4 ** 0)): begin : h12 initial $display("@Rd13_12 hit"); end default: begin : d12 initial $display("@Rd13_12 def"); end endcase
  case (64'h1) $signed((S4 ** 0)): begin : h13 initial $display("@Rd13_13 hit"); end default: begin : d13 initial $display("@Rd13_13 def"); end endcase
  case (1) $signed((S4 ** 0)): begin : h14 initial $display("@Rd13_14 hit"); end default: begin : d14 initial $display("@Rd13_14 def"); end endcase
  case (1) $signed((S4 ** 0)): begin : h15 initial $display("@Rd13_15 hit"); end default: begin : d15 initial $display("@Rd13_15 def"); end endcase
  case (1'h1) (|$signed(UN)): begin : h16 initial $display("@Rd13_16 hit"); end default: begin : d16 initial $display("@Rd13_16 def"); end endcase
  case (1'sh1) (|$signed(UN)): begin : h17 initial $display("@Rd13_17 hit"); end default: begin : d17 initial $display("@Rd13_17 def"); end endcase
  case (4'h1) (|$signed(UN)): begin : h18 initial $display("@Rd13_18 hit"); end default: begin : d18 initial $display("@Rd13_18 def"); end endcase
  case (64'hffffffffffffffff) (|$signed(UN)): begin : h19 initial $display("@Rd13_19 hit"); end default: begin : d19 initial $display("@Rd13_19 def"); end endcase
  case ((-1)) (|$signed(UN)): begin : h20 initial $display("@Rd13_20 hit"); end default: begin : d20 initial $display("@Rd13_20 def"); end endcase
  case (1) (|$signed(UN)): begin : h21 initial $display("@Rd13_21 hit"); end default: begin : d21 initial $display("@Rd13_21 def"); end endcase
  case (4'he) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h22 initial $display("@Rd13_22 hit"); end default: begin : d22 initial $display("@Rd13_22 def"); end endcase
  case (4'she) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h23 initial $display("@Rd13_23 hit"); end default: begin : d23 initial $display("@Rd13_23 def"); end endcase
  case (7'he) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h24 initial $display("@Rd13_24 hit"); end default: begin : d24 initial $display("@Rd13_24 def"); end endcase
endmodule
