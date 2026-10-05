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
  case (1'sh1) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h0 initial $display("@Ra28_0 hit"); end default: begin : d0 initial $display("@Ra28_0 def"); end endcase
  case (4'h1) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h1 initial $display("@Ra28_1 hit"); end default: begin : d1 initial $display("@Ra28_1 def"); end endcase
  case (64'hffffffffffffffff) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h2 initial $display("@Ra28_2 hit"); end default: begin : d2 initial $display("@Ra28_2 def"); end endcase
  case ((-1)) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h3 initial $display("@Ra28_3 hit"); end default: begin : d3 initial $display("@Ra28_3 def"); end endcase
  case (1) (65'sh13d895a436694b89e >= 33'he9ab5979): begin : h4 initial $display("@Ra28_4 hit"); end default: begin : d4 initial $display("@Ra28_4 def"); end endcase
  case (32'hfffffff0) (U32 >> 0): begin : h5 initial $display("@Ra28_5 hit"); end default: begin : d5 initial $display("@Ra28_5 def"); end endcase
  case (32'shfffffff0) (U32 >> 0): begin : h6 initial $display("@Ra28_6 hit"); end default: begin : d6 initial $display("@Ra28_6 def"); end endcase
  case (35'hfffffff0) (U32 >> 0): begin : h7 initial $display("@Ra28_7 hit"); end default: begin : d7 initial $display("@Ra28_7 def"); end endcase
  case (64'hfffffffffffffff0) (U32 >> 0): begin : h8 initial $display("@Ra28_8 hit"); end default: begin : d8 initial $display("@Ra28_8 def"); end endcase
  case ((-16)) (U32 >> 0): begin : h9 initial $display("@Ra28_9 hit"); end default: begin : d9 initial $display("@Ra28_9 def"); end endcase
  case (1'h0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h10 initial $display("@Ra28_10 hit"); end default: begin : d10 initial $display("@Ra28_10 def"); end endcase
  case (1'sh0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h11 initial $display("@Ra28_11 hit"); end default: begin : d11 initial $display("@Ra28_11 def"); end endcase
  case (4'h0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h12 initial $display("@Ra28_12 hit"); end default: begin : d12 initial $display("@Ra28_12 def"); end endcase
  case (64'h0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h13 initial $display("@Ra28_13 hit"); end default: begin : d13 initial $display("@Ra28_13 def"); end endcase
  case (0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h14 initial $display("@Ra28_14 hit"); end default: begin : d14 initial $display("@Ra28_14 def"); end endcase
  case (0) ((P8 >= 65'ha21a26727427bc76) - (!63'h456897e4a0d4f2e3)): begin : h15 initial $display("@Ra28_15 hit"); end default: begin : d15 initial $display("@Ra28_15 def"); end endcase
  case (4'hc) $unsigned(P4): begin : h16 initial $display("@Ra28_16 hit"); end default: begin : d16 initial $display("@Ra28_16 def"); end endcase
  case (4'shc) $unsigned(P4): begin : h17 initial $display("@Ra28_17 hit"); end default: begin : d17 initial $display("@Ra28_17 def"); end endcase
  case (7'hc) $unsigned(P4): begin : h18 initial $display("@Ra28_18 hit"); end default: begin : d18 initial $display("@Ra28_18 def"); end endcase
  case (64'hfffffffffffffffc) $unsigned(P4): begin : h19 initial $display("@Ra28_19 hit"); end default: begin : d19 initial $display("@Ra28_19 def"); end endcase
  case ((-4)) $unsigned(P4): begin : h20 initial $display("@Ra28_20 hit"); end default: begin : d20 initial $display("@Ra28_20 def"); end endcase
  case (12) $unsigned(P4): begin : h21 initial $display("@Ra28_21 hit"); end default: begin : d21 initial $display("@Ra28_21 def"); end endcase
  case (2'h1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h22 initial $display("@Ra28_22 hit"); end default: begin : d22 initial $display("@Ra28_22 def"); end endcase
  case (2'sh1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h23 initial $display("@Ra28_23 hit"); end default: begin : d23 initial $display("@Ra28_23 def"); end endcase
  case (5'h1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h24 initial $display("@Ra28_24 hit"); end default: begin : d24 initial $display("@Ra28_24 def"); end endcase
endmodule
