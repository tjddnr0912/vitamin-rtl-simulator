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
  case (0) ((64'he28af60465f42986 >> 0) < (U32 * 1'sh0)): begin : h0 initial $display("@Ra5_0 hit"); end default: begin : d0 initial $display("@Ra5_0 def"); end endcase
  case (0) ((64'he28af60465f42986 >> 0) < (U32 * 1'sh0)): begin : h1 initial $display("@Ra5_1 hit"); end default: begin : d1 initial $display("@Ra5_1 def"); end endcase
  case (32'hfffffffe) ($signed((-11)) >>> 3): begin : h2 initial $display("@Ra5_2 hit"); end default: begin : d2 initial $display("@Ra5_2 def"); end endcase
  case (32'shfffffffe) ($signed((-11)) >>> 3): begin : h3 initial $display("@Ra5_3 hit"); end default: begin : d3 initial $display("@Ra5_3 def"); end endcase
  case (35'hfffffffe) ($signed((-11)) >>> 3): begin : h4 initial $display("@Ra5_4 hit"); end default: begin : d4 initial $display("@Ra5_4 def"); end endcase
  case (64'hfffffffffffffffe) ($signed((-11)) >>> 3): begin : h5 initial $display("@Ra5_5 hit"); end default: begin : d5 initial $display("@Ra5_5 def"); end endcase
  case ((-2)) ($signed((-11)) >>> 3): begin : h6 initial $display("@Ra5_6 hit"); end default: begin : d6 initial $display("@Ra5_6 def"); end endcase
  case (8'hfe) ((8'she ^ S4) >>> 3): begin : h7 initial $display("@Ra5_7 hit"); end default: begin : d7 initial $display("@Ra5_7 def"); end endcase
  case (8'shfe) ((8'she ^ S4) >>> 3): begin : h8 initial $display("@Ra5_8 hit"); end default: begin : d8 initial $display("@Ra5_8 def"); end endcase
  case (11'hfe) ((8'she ^ S4) >>> 3): begin : h9 initial $display("@Ra5_9 hit"); end default: begin : d9 initial $display("@Ra5_9 def"); end endcase
  case (64'hfffffffffffffffe) ((8'she ^ S4) >>> 3): begin : h10 initial $display("@Ra5_10 hit"); end default: begin : d10 initial $display("@Ra5_10 def"); end endcase
  case ((-2)) ((8'she ^ S4) >>> 3): begin : h11 initial $display("@Ra5_11 hit"); end default: begin : d11 initial $display("@Ra5_11 def"); end endcase
  case (254) ((8'she ^ S4) >>> 3): begin : h12 initial $display("@Ra5_12 hit"); end default: begin : d12 initial $display("@Ra5_12 def"); end endcase
  case (5'h0) (5'h1 >> (-5)): begin : h13 initial $display("@Ra5_13 hit"); end default: begin : d13 initial $display("@Ra5_13 def"); end endcase
  case (5'sh0) (5'h1 >> (-5)): begin : h14 initial $display("@Ra5_14 hit"); end default: begin : d14 initial $display("@Ra5_14 def"); end endcase
  case (8'h0) (5'h1 >> (-5)): begin : h15 initial $display("@Ra5_15 hit"); end default: begin : d15 initial $display("@Ra5_15 def"); end endcase
  case (64'h0) (5'h1 >> (-5)): begin : h16 initial $display("@Ra5_16 hit"); end default: begin : d16 initial $display("@Ra5_16 def"); end endcase
  case (0) (5'h1 >> (-5)): begin : h17 initial $display("@Ra5_17 hit"); end default: begin : d17 initial $display("@Ra5_17 def"); end endcase
  case (0) (5'h1 >> (-5)): begin : h18 initial $display("@Ra5_18 hit"); end default: begin : d18 initial $display("@Ra5_18 def"); end endcase
  case (8'h0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h19 initial $display("@Ra5_19 hit"); end default: begin : d19 initial $display("@Ra5_19 def"); end endcase
  case (8'sh0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h20 initial $display("@Ra5_20 hit"); end default: begin : d20 initial $display("@Ra5_20 def"); end endcase
  case (11'h0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h21 initial $display("@Ra5_21 hit"); end default: begin : d21 initial $display("@Ra5_21 def"); end endcase
  case (64'h0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h22 initial $display("@Ra5_22 hit"); end default: begin : d22 initial $display("@Ra5_22 def"); end endcase
  case (0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h23 initial $display("@Ra5_23 hit"); end default: begin : d23 initial $display("@Ra5_23 def"); end endcase
  case (0) ((P8 >>> 3) >>> (-32'h83a4e629)): begin : h24 initial $display("@Ra5_24 hit"); end default: begin : d24 initial $display("@Ra5_24 def"); end endcase
endmodule
