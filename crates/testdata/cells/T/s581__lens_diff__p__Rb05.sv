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
  case (1'h0) ($unsigned(U32) < (S4 + S8)): begin : h0 initial $display("@Rb5_0 hit"); end default: begin : d0 initial $display("@Rb5_0 def"); end endcase
  case (1'sh0) ($unsigned(U32) < (S4 + S8)): begin : h1 initial $display("@Rb5_1 hit"); end default: begin : d1 initial $display("@Rb5_1 def"); end endcase
  case (4'h0) ($unsigned(U32) < (S4 + S8)): begin : h2 initial $display("@Rb5_2 hit"); end default: begin : d2 initial $display("@Rb5_2 def"); end endcase
  case (64'h0) ($unsigned(U32) < (S4 + S8)): begin : h3 initial $display("@Rb5_3 hit"); end default: begin : d3 initial $display("@Rb5_3 def"); end endcase
  case (0) ($unsigned(U32) < (S4 + S8)): begin : h4 initial $display("@Rb5_4 hit"); end default: begin : d4 initial $display("@Rb5_4 def"); end endcase
  case (0) ($unsigned(U32) < (S4 + S8)): begin : h5 initial $display("@Rb5_5 hit"); end default: begin : d5 initial $display("@Rb5_5 def"); end endcase
  case (1'h0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h6 initial $display("@Rb5_6 hit"); end default: begin : d6 initial $display("@Rb5_6 def"); end endcase
  case (1'sh0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h7 initial $display("@Rb5_7 hit"); end default: begin : d7 initial $display("@Rb5_7 def"); end endcase
  case (4'h0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h8 initial $display("@Rb5_8 hit"); end default: begin : d8 initial $display("@Rb5_8 def"); end endcase
  case (64'h0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h9 initial $display("@Rb5_9 hit"); end default: begin : d9 initial $display("@Rb5_9 def"); end endcase
  case (0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h10 initial $display("@Rb5_10 hit"); end default: begin : d10 initial $display("@Rb5_10 def"); end endcase
  case (0) ((63'sh54288dfd1d2cbba1 == 32'hc4864db2) >>> (S4 >>> 4)): begin : h11 initial $display("@Rb5_11 hit"); end default: begin : d11 initial $display("@Rb5_11 def"); end endcase
  case (4'h0) (UN >> 4): begin : h12 initial $display("@Rb5_12 hit"); end default: begin : d12 initial $display("@Rb5_12 def"); end endcase
  case (4'sh0) (UN >> 4): begin : h13 initial $display("@Rb5_13 hit"); end default: begin : d13 initial $display("@Rb5_13 def"); end endcase
  case (7'h0) (UN >> 4): begin : h14 initial $display("@Rb5_14 hit"); end default: begin : d14 initial $display("@Rb5_14 def"); end endcase
  case (64'h0) (UN >> 4): begin : h15 initial $display("@Rb5_15 hit"); end default: begin : d15 initial $display("@Rb5_15 def"); end endcase
  case (0) (UN >> 4): begin : h16 initial $display("@Rb5_16 hit"); end default: begin : d16 initial $display("@Rb5_16 def"); end endcase
  case (0) (UN >> 4): begin : h17 initial $display("@Rb5_17 hit"); end default: begin : d17 initial $display("@Rb5_17 def"); end endcase
  case (37'h300000001) {4'(65'h1cb44d50b6d027a31), 33'h1_0000_0001}: begin : h18 initial $display("@Rb5_18 hit"); end default: begin : d18 initial $display("@Rb5_18 def"); end endcase
  case (37'sh300000001) {4'(65'h1cb44d50b6d027a31), 33'h1_0000_0001}: begin : h19 initial $display("@Rb5_19 hit"); end default: begin : d19 initial $display("@Rb5_19 def"); end endcase
  case (40'h300000001) {4'(65'h1cb44d50b6d027a31), 33'h1_0000_0001}: begin : h20 initial $display("@Rb5_20 hit"); end default: begin : d20 initial $display("@Rb5_20 def"); end endcase
  case (64'h300000001) {4'(65'h1cb44d50b6d027a31), 33'h1_0000_0001}: begin : h21 initial $display("@Rb5_21 hit"); end default: begin : d21 initial $display("@Rb5_21 def"); end endcase
  case (8'h9c) S8: begin : h22 initial $display("@Rb5_22 hit"); end default: begin : d22 initial $display("@Rb5_22 def"); end endcase
  case (8'sh9c) S8: begin : h23 initial $display("@Rb5_23 hit"); end default: begin : d23 initial $display("@Rb5_23 def"); end endcase
  case (11'h9c) S8: begin : h24 initial $display("@Rb5_24 hit"); end default: begin : d24 initial $display("@Rb5_24 def"); end endcase
endmodule
