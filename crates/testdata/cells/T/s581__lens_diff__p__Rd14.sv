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
  case (64'hfffffffffffffffe) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h0 initial $display("@Rd14_0 hit"); end default: begin : d0 initial $display("@Rd14_0 def"); end endcase
  case ((-2)) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h1 initial $display("@Rd14_1 hit"); end default: begin : d1 initial $display("@Rd14_1 def"); end endcase
  case (14) ((P4 ^ 2'h2) * (31'hf608bb6 && S8)): begin : h2 initial $display("@Rd14_2 hit"); end default: begin : d2 initial $display("@Rd14_2 def"); end endcase
  case (11'h7b1) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h3 initial $display("@Rd14_3 hit"); end default: begin : d3 initial $display("@Rd14_3 def"); end endcase
  case (11'sh7b1) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h4 initial $display("@Rd14_4 hit"); end default: begin : d4 initial $display("@Rd14_4 def"); end endcase
  case (14'h7b1) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h5 initial $display("@Rd14_5 hit"); end default: begin : d5 initial $display("@Rd14_5 def"); end endcase
  case (64'hffffffffffffffb1) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h6 initial $display("@Rd14_6 hit"); end default: begin : d6 initial $display("@Rd14_6 def"); end endcase
  case ((-79)) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h7 initial $display("@Rd14_7 hit"); end default: begin : d7 initial $display("@Rd14_7 def"); end endcase
  case (1969) {8'((3'h6 | P8)), 3'({4'(P65), 8'(33)})}: begin : h8 initial $display("@Rd14_8 hit"); end default: begin : d8 initial $display("@Rd14_8 def"); end endcase
  case (1'h1) (~^(2'h3 ? 2'sh3 : UN)): begin : h9 initial $display("@Rd14_9 hit"); end default: begin : d9 initial $display("@Rd14_9 def"); end endcase
  case (1'sh1) (~^(2'h3 ? 2'sh3 : UN)): begin : h10 initial $display("@Rd14_10 hit"); end default: begin : d10 initial $display("@Rd14_10 def"); end endcase
  case (4'h1) (~^(2'h3 ? 2'sh3 : UN)): begin : h11 initial $display("@Rd14_11 hit"); end default: begin : d11 initial $display("@Rd14_11 def"); end endcase
  case (64'hffffffffffffffff) (~^(2'h3 ? 2'sh3 : UN)): begin : h12 initial $display("@Rd14_12 hit"); end default: begin : d12 initial $display("@Rd14_12 def"); end endcase
  case ((-1)) (~^(2'h3 ? 2'sh3 : UN)): begin : h13 initial $display("@Rd14_13 hit"); end default: begin : d13 initial $display("@Rd14_13 def"); end endcase
  case (1) (~^(2'h3 ? 2'sh3 : UN)): begin : h14 initial $display("@Rd14_14 hit"); end default: begin : d14 initial $display("@Rd14_14 def"); end endcase
  case (41'h40b) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h15 initial $display("@Rd14_15 hit"); end default: begin : d15 initial $display("@Rd14_15 def"); end endcase
  case (41'sh40b) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h16 initial $display("@Rd14_16 hit"); end default: begin : d16 initial $display("@Rd14_16 def"); end endcase
  case (44'h40b) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h17 initial $display("@Rd14_17 hit"); end default: begin : d17 initial $display("@Rd14_17 def"); end endcase
  case (64'h40b) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h18 initial $display("@Rd14_18 hit"); end default: begin : d18 initial $display("@Rd14_18 def"); end endcase
  case (1035) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h19 initial $display("@Rd14_19 hit"); end default: begin : d19 initial $display("@Rd14_19 def"); end endcase
  case (1035) ((1'sh1 ? 2'h1 : S8) | {33'(4'sh4), 8'(UN)}): begin : h20 initial $display("@Rd14_20 hit"); end default: begin : d20 initial $display("@Rd14_20 def"); end endcase
  case (12'hdb0) {S4, 8'($signed((31'h957cff6 << 3)))}: begin : h21 initial $display("@Rd14_21 hit"); end default: begin : d21 initial $display("@Rd14_21 def"); end endcase
  case (12'shdb0) {S4, 8'($signed((31'h957cff6 << 3)))}: begin : h22 initial $display("@Rd14_22 hit"); end default: begin : d22 initial $display("@Rd14_22 def"); end endcase
  case (15'hdb0) {S4, 8'($signed((31'h957cff6 << 3)))}: begin : h23 initial $display("@Rd14_23 hit"); end default: begin : d23 initial $display("@Rd14_23 def"); end endcase
  case (64'hfffffffffffffdb0) {S4, 8'($signed((31'h957cff6 << 3)))}: begin : h24 initial $display("@Rd14_24 hit"); end default: begin : d24 initial $display("@Rd14_24 def"); end endcase
endmodule
