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
  case ((-16)) (s6_t'(U32) >> (L64 >= S4)): begin : h0 initial $display("@Rc15_0 hit"); end default: begin : d0 initial $display("@Rc15_0 def"); end endcase
  case (48) (s6_t'(U32) >> (L64 >= S4)): begin : h1 initial $display("@Rc15_1 hit"); end default: begin : d1 initial $display("@Rc15_1 def"); end endcase
  case (1'h1) $signed((UN && 63'h4f100f5847f675d5)): begin : h2 initial $display("@Rc15_2 hit"); end default: begin : d2 initial $display("@Rc15_2 def"); end endcase
  case (1'sh1) $signed((UN && 63'h4f100f5847f675d5)): begin : h3 initial $display("@Rc15_3 hit"); end default: begin : d3 initial $display("@Rc15_3 def"); end endcase
  case (4'h1) $signed((UN && 63'h4f100f5847f675d5)): begin : h4 initial $display("@Rc15_4 hit"); end default: begin : d4 initial $display("@Rc15_4 def"); end endcase
  case (64'hffffffffffffffff) $signed((UN && 63'h4f100f5847f675d5)): begin : h5 initial $display("@Rc15_5 hit"); end default: begin : d5 initial $display("@Rc15_5 def"); end endcase
  case ((-1)) $signed((UN && 63'h4f100f5847f675d5)): begin : h6 initial $display("@Rc15_6 hit"); end default: begin : d6 initial $display("@Rc15_6 def"); end endcase
  case (1) $signed((UN && 63'h4f100f5847f675d5)): begin : h7 initial $display("@Rc15_7 hit"); end default: begin : d7 initial $display("@Rc15_7 def"); end endcase
  case (8'hdd) {2{S4}}: begin : h8 initial $display("@Rc15_8 hit"); end default: begin : d8 initial $display("@Rc15_8 def"); end endcase
  case (8'shdd) {2{S4}}: begin : h9 initial $display("@Rc15_9 hit"); end default: begin : d9 initial $display("@Rc15_9 def"); end endcase
  case (11'hdd) {2{S4}}: begin : h10 initial $display("@Rc15_10 hit"); end default: begin : d10 initial $display("@Rc15_10 def"); end endcase
  case (64'hffffffffffffffdd) {2{S4}}: begin : h11 initial $display("@Rc15_11 hit"); end default: begin : d11 initial $display("@Rc15_11 def"); end endcase
  case ((-35)) {2{S4}}: begin : h12 initial $display("@Rc15_12 hit"); end default: begin : d12 initial $display("@Rc15_12 def"); end endcase
  case (221) {2{S4}}: begin : h13 initial $display("@Rc15_13 hit"); end default: begin : d13 initial $display("@Rc15_13 def"); end endcase
  case (12'hb9c) {4'hB, S8}: begin : h14 initial $display("@Rc15_14 hit"); end default: begin : d14 initial $display("@Rc15_14 def"); end endcase
  case (12'shb9c) {4'hB, S8}: begin : h15 initial $display("@Rc15_15 hit"); end default: begin : d15 initial $display("@Rc15_15 def"); end endcase
  case (15'hb9c) {4'hB, S8}: begin : h16 initial $display("@Rc15_16 hit"); end default: begin : d16 initial $display("@Rc15_16 def"); end endcase
  case (64'hfffffffffffffb9c) {4'hB, S8}: begin : h17 initial $display("@Rc15_17 hit"); end default: begin : d17 initial $display("@Rc15_17 def"); end endcase
  case ((-1124)) {4'hB, S8}: begin : h18 initial $display("@Rc15_18 hit"); end default: begin : d18 initial $display("@Rc15_18 def"); end endcase
  case (2972) {4'hB, S8}: begin : h19 initial $display("@Rc15_19 hit"); end default: begin : d19 initial $display("@Rc15_19 def"); end endcase
  case (36'h5d8) {33'({2{4'hB}}), 3'((~|$unsigned(33)))}: begin : h20 initial $display("@Rc15_20 hit"); end default: begin : d20 initial $display("@Rc15_20 def"); end endcase
  case (36'sh5d8) {33'({2{4'hB}}), 3'((~|$unsigned(33)))}: begin : h21 initial $display("@Rc15_21 hit"); end default: begin : d21 initial $display("@Rc15_21 def"); end endcase
  case (39'h5d8) {33'({2{4'hB}}), 3'((~|$unsigned(33)))}: begin : h22 initial $display("@Rc15_22 hit"); end default: begin : d22 initial $display("@Rc15_22 def"); end endcase
  case (64'h5d8) {33'({2{4'hB}}), 3'((~|$unsigned(33)))}: begin : h23 initial $display("@Rc15_23 hit"); end default: begin : d23 initial $display("@Rc15_23 def"); end endcase
  case (1496) {33'({2{4'hB}}), 3'((~|$unsigned(33)))}: begin : h24 initial $display("@Rc15_24 hit"); end default: begin : d24 initial $display("@Rc15_24 def"); end endcase
endmodule
