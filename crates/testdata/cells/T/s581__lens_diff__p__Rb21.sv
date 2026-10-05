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
  case ((-1)) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h0 initial $display("@Rb21_0 hit"); end default: begin : d0 initial $display("@Rb21_0 def"); end endcase
  case (1) (64'h3baa839d4af0bda5 || (I >> 5)): begin : h1 initial $display("@Rb21_1 hit"); end default: begin : d1 initial $display("@Rb21_1 def"); end endcase
  case (64'h342cf7571477cc50) ((-64'hcbd308a8eb8832c0) - $unsigned(P8)): begin : h2 initial $display("@Rb21_2 hit"); end default: begin : d2 initial $display("@Rb21_2 def"); end endcase
  case (64'sh342cf7571477cc50) ((-64'hcbd308a8eb8832c0) - $unsigned(P8)): begin : h3 initial $display("@Rb21_3 hit"); end default: begin : d3 initial $display("@Rb21_3 def"); end endcase
  case (64'h342cf7571477cc50) ((-64'hcbd308a8eb8832c0) - $unsigned(P8)): begin : h4 initial $display("@Rb21_4 hit"); end default: begin : d4 initial $display("@Rb21_4 def"); end endcase
  case (64'h342cf7571477cc50) ((-64'hcbd308a8eb8832c0) - $unsigned(P8)): begin : h5 initial $display("@Rb21_5 hit"); end default: begin : d5 initial $display("@Rb21_5 def"); end endcase
  case (1'h0) (!(I % L64)): begin : h6 initial $display("@Rb21_6 hit"); end default: begin : d6 initial $display("@Rb21_6 def"); end endcase
  case (1'sh0) (!(I % L64)): begin : h7 initial $display("@Rb21_7 hit"); end default: begin : d7 initial $display("@Rb21_7 def"); end endcase
  case (4'h0) (!(I % L64)): begin : h8 initial $display("@Rb21_8 hit"); end default: begin : d8 initial $display("@Rb21_8 def"); end endcase
  case (64'h0) (!(I % L64)): begin : h9 initial $display("@Rb21_9 hit"); end default: begin : d9 initial $display("@Rb21_9 def"); end endcase
  case (0) (!(I % L64)): begin : h10 initial $display("@Rb21_10 hit"); end default: begin : d10 initial $display("@Rb21_10 def"); end endcase
  case (0) (!(I % L64)): begin : h11 initial $display("@Rb21_11 hit"); end default: begin : d11 initial $display("@Rb21_11 def"); end endcase
  case (63'hb7e1646c93183) (63'h16fc2c8d926307f5 >>> $signed(9)): begin : h12 initial $display("@Rb21_12 hit"); end default: begin : d12 initial $display("@Rb21_12 def"); end endcase
  case (63'shb7e1646c93183) (63'h16fc2c8d926307f5 >>> $signed(9)): begin : h13 initial $display("@Rb21_13 hit"); end default: begin : d13 initial $display("@Rb21_13 def"); end endcase
  case (64'hb7e1646c93183) (63'h16fc2c8d926307f5 >>> $signed(9)): begin : h14 initial $display("@Rb21_14 hit"); end default: begin : d14 initial $display("@Rb21_14 def"); end endcase
  case (64'hb7e1646c93183) (63'h16fc2c8d926307f5 >>> $signed(9)): begin : h15 initial $display("@Rb21_15 hit"); end default: begin : d15 initial $display("@Rb21_15 def"); end endcase
  case (37'h100000000f) ({33'h1_0000_0001, 4'hB} - (P4 >> 0)): begin : h16 initial $display("@Rb21_16 hit"); end default: begin : d16 initial $display("@Rb21_16 def"); end endcase
  case (37'sh100000000f) ({33'h1_0000_0001, 4'hB} - (P4 >> 0)): begin : h17 initial $display("@Rb21_17 hit"); end default: begin : d17 initial $display("@Rb21_17 def"); end endcase
  case (40'h100000000f) ({33'h1_0000_0001, 4'hB} - (P4 >> 0)): begin : h18 initial $display("@Rb21_18 hit"); end default: begin : d18 initial $display("@Rb21_18 def"); end endcase
  case (64'hfffffff00000000f) ({33'h1_0000_0001, 4'hB} - (P4 >> 0)): begin : h19 initial $display("@Rb21_19 hit"); end default: begin : d19 initial $display("@Rb21_19 def"); end endcase
  case (1'h1) (UN && 38): begin : h20 initial $display("@Rb21_20 hit"); end default: begin : d20 initial $display("@Rb21_20 def"); end endcase
  case (1'sh1) (UN && 38): begin : h21 initial $display("@Rb21_21 hit"); end default: begin : d21 initial $display("@Rb21_21 def"); end endcase
  case (4'h1) (UN && 38): begin : h22 initial $display("@Rb21_22 hit"); end default: begin : d22 initial $display("@Rb21_22 def"); end endcase
  case (64'hffffffffffffffff) (UN && 38): begin : h23 initial $display("@Rb21_23 hit"); end default: begin : d23 initial $display("@Rb21_23 def"); end endcase
  case ((-1)) (UN && 38): begin : h24 initial $display("@Rb21_24 hit"); end default: begin : d24 initial $display("@Rb21_24 def"); end endcase
endmodule
