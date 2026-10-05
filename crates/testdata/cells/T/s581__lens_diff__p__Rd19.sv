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
  case (4'h0) (^$signed(S8)): begin : h0 initial $display("@Rd19_0 hit"); end default: begin : d0 initial $display("@Rd19_0 def"); end endcase
  case (64'h0) (^$signed(S8)): begin : h1 initial $display("@Rd19_1 hit"); end default: begin : d1 initial $display("@Rd19_1 def"); end endcase
  case (0) (^$signed(S8)): begin : h2 initial $display("@Rd19_2 hit"); end default: begin : d2 initial $display("@Rd19_2 def"); end endcase
  case (0) (^$signed(S8)): begin : h3 initial $display("@Rd19_3 hit"); end default: begin : d3 initial $display("@Rd19_3 def"); end endcase
  case (6'h0) W6'(((UN / P128) >= $unsigned(P8))): begin : h4 initial $display("@Rd19_4 hit"); end default: begin : d4 initial $display("@Rd19_4 def"); end endcase
  case (6'sh0) W6'(((UN / P128) >= $unsigned(P8))): begin : h5 initial $display("@Rd19_5 hit"); end default: begin : d5 initial $display("@Rd19_5 def"); end endcase
  case (9'h0) W6'(((UN / P128) >= $unsigned(P8))): begin : h6 initial $display("@Rd19_6 hit"); end default: begin : d6 initial $display("@Rd19_6 def"); end endcase
  case (64'h0) W6'(((UN / P128) >= $unsigned(P8))): begin : h7 initial $display("@Rd19_7 hit"); end default: begin : d7 initial $display("@Rd19_7 def"); end endcase
  case (0) W6'(((UN / P128) >= $unsigned(P8))): begin : h8 initial $display("@Rd19_8 hit"); end default: begin : d8 initial $display("@Rd19_8 def"); end endcase
  case (0) W6'(((UN / P128) >= $unsigned(P8))): begin : h9 initial $display("@Rd19_9 hit"); end default: begin : d9 initial $display("@Rd19_9 def"); end endcase
  case (8'h1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h10 initial $display("@Rd19_10 hit"); end default: begin : d10 initial $display("@Rd19_10 def"); end endcase
  case (8'sh1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h11 initial $display("@Rd19_11 hit"); end default: begin : d11 initial $display("@Rd19_11 def"); end endcase
  case (11'h1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h12 initial $display("@Rd19_12 hit"); end default: begin : d12 initial $display("@Rd19_12 def"); end endcase
  case (64'h1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h13 initial $display("@Rd19_13 hit"); end default: begin : d13 initial $display("@Rd19_13 def"); end endcase
  case (1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h14 initial $display("@Rd19_14 hit"); end default: begin : d14 initial $display("@Rd19_14 def"); end endcase
  case (1) ($unsigned(4'sh2) ? (U32 < 65'h1824988681e7b5bb0) : {2{4'(5'h10)}}): begin : h15 initial $display("@Rd19_15 hit"); end default: begin : d15 initial $display("@Rd19_15 def"); end endcase
  case (32'h6) $unsigned(W6): begin : h16 initial $display("@Rd19_16 hit"); end default: begin : d16 initial $display("@Rd19_16 def"); end endcase
  case (32'sh6) $unsigned(W6): begin : h17 initial $display("@Rd19_17 hit"); end default: begin : d17 initial $display("@Rd19_17 def"); end endcase
  case (35'h6) $unsigned(W6): begin : h18 initial $display("@Rd19_18 hit"); end default: begin : d18 initial $display("@Rd19_18 def"); end endcase
  case (64'h6) $unsigned(W6): begin : h19 initial $display("@Rd19_19 hit"); end default: begin : d19 initial $display("@Rd19_19 def"); end endcase
  case (6) $unsigned(W6): begin : h20 initial $display("@Rd19_20 hit"); end default: begin : d20 initial $display("@Rd19_20 def"); end endcase
  case (6) $unsigned(W6): begin : h21 initial $display("@Rd19_21 hit"); end default: begin : d21 initial $display("@Rd19_21 def"); end endcase
  case (8'hfe) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h22 initial $display("@Rd19_22 hit"); end default: begin : d22 initial $display("@Rd19_22 def"); end endcase
  case (8'shfe) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h23 initial $display("@Rd19_23 hit"); end default: begin : d23 initial $display("@Rd19_23 def"); end endcase
  case (11'hfe) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h24 initial $display("@Rd19_24 hit"); end default: begin : d24 initial $display("@Rd19_24 def"); end endcase
endmodule
