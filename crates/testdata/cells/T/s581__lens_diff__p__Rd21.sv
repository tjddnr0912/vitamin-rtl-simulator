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
  case (7'ha) UN: begin : h0 initial $display("@Rd21_0 hit"); end default: begin : d0 initial $display("@Rd21_0 def"); end endcase
  case (64'hfffffffffffffffa) UN: begin : h1 initial $display("@Rd21_1 hit"); end default: begin : d1 initial $display("@Rd21_1 def"); end endcase
  case ((-6)) UN: begin : h2 initial $display("@Rd21_2 hit"); end default: begin : d2 initial $display("@Rd21_2 def"); end endcase
  case (10) UN: begin : h3 initial $display("@Rd21_3 hit"); end default: begin : d3 initial $display("@Rd21_3 def"); end endcase
  case (6'h1) W6'((&((-5) && P128))): begin : h4 initial $display("@Rd21_4 hit"); end default: begin : d4 initial $display("@Rd21_4 def"); end endcase
  case (6'sh1) W6'((&((-5) && P128))): begin : h5 initial $display("@Rd21_5 hit"); end default: begin : d5 initial $display("@Rd21_5 def"); end endcase
  case (9'h1) W6'((&((-5) && P128))): begin : h6 initial $display("@Rd21_6 hit"); end default: begin : d6 initial $display("@Rd21_6 def"); end endcase
  case (64'h1) W6'((&((-5) && P128))): begin : h7 initial $display("@Rd21_7 hit"); end default: begin : d7 initial $display("@Rd21_7 def"); end endcase
  case (1) W6'((&((-5) && P128))): begin : h8 initial $display("@Rd21_8 hit"); end default: begin : d8 initial $display("@Rd21_8 def"); end endcase
  case (1) W6'((&((-5) && P128))): begin : h9 initial $display("@Rd21_9 hit"); end default: begin : d9 initial $display("@Rd21_9 def"); end endcase
  case (6'h6) s6_t'((14 ? W6 : 2)): begin : h10 initial $display("@Rd21_10 hit"); end default: begin : d10 initial $display("@Rd21_10 def"); end endcase
  case (6'sh6) s6_t'((14 ? W6 : 2)): begin : h11 initial $display("@Rd21_11 hit"); end default: begin : d11 initial $display("@Rd21_11 def"); end endcase
  case (9'h6) s6_t'((14 ? W6 : 2)): begin : h12 initial $display("@Rd21_12 hit"); end default: begin : d12 initial $display("@Rd21_12 def"); end endcase
  case (64'h6) s6_t'((14 ? W6 : 2)): begin : h13 initial $display("@Rd21_13 hit"); end default: begin : d13 initial $display("@Rd21_13 def"); end endcase
  case (6) s6_t'((14 ? W6 : 2)): begin : h14 initial $display("@Rd21_14 hit"); end default: begin : d14 initial $display("@Rd21_14 def"); end endcase
  case (6) s6_t'((14 ? W6 : 2)): begin : h15 initial $display("@Rd21_15 hit"); end default: begin : d15 initial $display("@Rd21_15 def"); end endcase
  case (12'h0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h16 initial $display("@Rd21_16 hit"); end default: begin : d16 initial $display("@Rd21_16 def"); end endcase
  case (12'sh0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h17 initial $display("@Rd21_17 hit"); end default: begin : d17 initial $display("@Rd21_17 def"); end endcase
  case (15'h0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h18 initial $display("@Rd21_18 hit"); end default: begin : d18 initial $display("@Rd21_18 def"); end endcase
  case (64'h0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h19 initial $display("@Rd21_19 hit"); end default: begin : d19 initial $display("@Rd21_19 def"); end endcase
  case (0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h20 initial $display("@Rd21_20 hit"); end default: begin : d20 initial $display("@Rd21_20 def"); end endcase
  case (0) (65'sh10e548570c1a549c1 ? (33'h8d2dff1d == 5'sh13) : {S8, 4'hB}): begin : h21 initial $display("@Rd21_21 hit"); end default: begin : d21 initial $display("@Rd21_21 def"); end endcase
  case (1'h1) ((16'shc644 >>> 0) || {2{P8}}): begin : h22 initial $display("@Rd21_22 hit"); end default: begin : d22 initial $display("@Rd21_22 def"); end endcase
  case (1'sh1) ((16'shc644 >>> 0) || {2{P8}}): begin : h23 initial $display("@Rd21_23 hit"); end default: begin : d23 initial $display("@Rd21_23 def"); end endcase
  case (4'h1) ((16'shc644 >>> 0) || {2{P8}}): begin : h24 initial $display("@Rd21_24 hit"); end default: begin : d24 initial $display("@Rd21_24 def"); end endcase
endmodule
