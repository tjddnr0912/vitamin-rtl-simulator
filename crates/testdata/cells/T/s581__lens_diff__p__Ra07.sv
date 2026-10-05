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
  case ((-702669284)) ((-32'h1789819f) * {8'(I), S8}): begin : h0 initial $display("@Ra7_0 hit"); end default: begin : d0 initial $display("@Ra7_0 def"); end endcase
  case (12'ha01) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h1 initial $display("@Ra7_1 hit"); end default: begin : d1 initial $display("@Ra7_1 def"); end endcase
  case (12'sha01) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h2 initial $display("@Ra7_2 hit"); end default: begin : d2 initial $display("@Ra7_2 def"); end endcase
  case (15'ha01) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h3 initial $display("@Ra7_3 hit"); end default: begin : d3 initial $display("@Ra7_3 def"); end endcase
  case (64'hfffffffffffffa01) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h4 initial $display("@Ra7_4 hit"); end default: begin : d4 initial $display("@Ra7_4 def"); end endcase
  case ((-1535)) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h5 initial $display("@Ra7_5 hit"); end default: begin : d5 initial $display("@Ra7_5 def"); end endcase
  case (2561) {8'((63'sh4ef9012f0bf7a4bd << 5)), 4'({P8, 1'(63'sh567d96af37bac233)})}: begin : h6 initial $display("@Ra7_6 hit"); end default: begin : d6 initial $display("@Ra7_6 def"); end endcase
  case (1'h1) ((L64 << 3) || (~L64)): begin : h7 initial $display("@Ra7_7 hit"); end default: begin : d7 initial $display("@Ra7_7 def"); end endcase
  case (1'sh1) ((L64 << 3) || (~L64)): begin : h8 initial $display("@Ra7_8 hit"); end default: begin : d8 initial $display("@Ra7_8 def"); end endcase
  case (4'h1) ((L64 << 3) || (~L64)): begin : h9 initial $display("@Ra7_9 hit"); end default: begin : d9 initial $display("@Ra7_9 def"); end endcase
  case (64'hffffffffffffffff) ((L64 << 3) || (~L64)): begin : h10 initial $display("@Ra7_10 hit"); end default: begin : d10 initial $display("@Ra7_10 def"); end endcase
  case ((-1)) ((L64 << 3) || (~L64)): begin : h11 initial $display("@Ra7_11 hit"); end default: begin : d11 initial $display("@Ra7_11 def"); end endcase
  case (1) ((L64 << 3) || (~L64)): begin : h12 initial $display("@Ra7_12 hit"); end default: begin : d12 initial $display("@Ra7_12 def"); end endcase
  case (2'h0) (2'h0 >>> 2): begin : h13 initial $display("@Ra7_13 hit"); end default: begin : d13 initial $display("@Ra7_13 def"); end endcase
  case (2'sh0) (2'h0 >>> 2): begin : h14 initial $display("@Ra7_14 hit"); end default: begin : d14 initial $display("@Ra7_14 def"); end endcase
  case (5'h0) (2'h0 >>> 2): begin : h15 initial $display("@Ra7_15 hit"); end default: begin : d15 initial $display("@Ra7_15 def"); end endcase
  case (64'h0) (2'h0 >>> 2): begin : h16 initial $display("@Ra7_16 hit"); end default: begin : d16 initial $display("@Ra7_16 def"); end endcase
  case (0) (2'h0 >>> 2): begin : h17 initial $display("@Ra7_17 hit"); end default: begin : d17 initial $display("@Ra7_17 def"); end endcase
  case (0) (2'h0 >>> 2): begin : h18 initial $display("@Ra7_18 hit"); end default: begin : d18 initial $display("@Ra7_18 def"); end endcase
  case (64'h0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h19 initial $display("@Ra7_19 hit"); end default: begin : d19 initial $display("@Ra7_19 def"); end endcase
  case (64'sh0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h20 initial $display("@Ra7_20 hit"); end default: begin : d20 initial $display("@Ra7_20 def"); end endcase
  case (64'h0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h21 initial $display("@Ra7_21 hit"); end default: begin : d21 initial $display("@Ra7_21 def"); end endcase
  case (64'h0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h22 initial $display("@Ra7_22 hit"); end default: begin : d22 initial $display("@Ra7_22 def"); end endcase
  case (0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h23 initial $display("@Ra7_23 hit"); end default: begin : d23 initial $display("@Ra7_23 def"); end endcase
  case (0) (((U32 ? 2'h0 : 5'sh2) * (I - P4)) / $signed((L64 % U32))): begin : h24 initial $display("@Ra7_24 hit"); end default: begin : d24 initial $display("@Ra7_24 def"); end endcase
endmodule
