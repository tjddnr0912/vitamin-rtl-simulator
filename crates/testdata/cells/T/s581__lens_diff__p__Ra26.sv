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
  case ((-4)) P4: begin : h0 initial $display("@Ra26_0 hit"); end default: begin : d0 initial $display("@Ra26_0 def"); end endcase
  case (12) P4: begin : h1 initial $display("@Ra26_1 hit"); end default: begin : d1 initial $display("@Ra26_1 def"); end endcase
  case (1'h1) (S8 || U32): begin : h2 initial $display("@Ra26_2 hit"); end default: begin : d2 initial $display("@Ra26_2 def"); end endcase
  case (1'sh1) (S8 || U32): begin : h3 initial $display("@Ra26_3 hit"); end default: begin : d3 initial $display("@Ra26_3 def"); end endcase
  case (4'h1) (S8 || U32): begin : h4 initial $display("@Ra26_4 hit"); end default: begin : d4 initial $display("@Ra26_4 def"); end endcase
  case (64'hffffffffffffffff) (S8 || U32): begin : h5 initial $display("@Ra26_5 hit"); end default: begin : d5 initial $display("@Ra26_5 def"); end endcase
  case ((-1)) (S8 || U32): begin : h6 initial $display("@Ra26_6 hit"); end default: begin : d6 initial $display("@Ra26_6 def"); end endcase
  case (1) (S8 || U32): begin : h7 initial $display("@Ra26_7 hit"); end default: begin : d7 initial $display("@Ra26_7 def"); end endcase
  case (1'h1) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h8 initial $display("@Ra26_8 hit"); end default: begin : d8 initial $display("@Ra26_8 def"); end endcase
  case (1'sh1) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h9 initial $display("@Ra26_9 hit"); end default: begin : d9 initial $display("@Ra26_9 def"); end endcase
  case (4'h1) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h10 initial $display("@Ra26_10 hit"); end default: begin : d10 initial $display("@Ra26_10 def"); end endcase
  case (64'hffffffffffffffff) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h11 initial $display("@Ra26_11 hit"); end default: begin : d11 initial $display("@Ra26_11 def"); end endcase
  case ((-1)) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h12 initial $display("@Ra26_12 hit"); end default: begin : d12 initial $display("@Ra26_12 def"); end endcase
  case (1) ({33'h1_0000_0001, 3'(P65)} >= (UN / P8)): begin : h13 initial $display("@Ra26_13 hit"); end default: begin : d13 initial $display("@Ra26_13 def"); end endcase
  case (1'h0) ((13 || 2'sh0) >= 4'sh4): begin : h14 initial $display("@Ra26_14 hit"); end default: begin : d14 initial $display("@Ra26_14 def"); end endcase
  case (1'sh0) ((13 || 2'sh0) >= 4'sh4): begin : h15 initial $display("@Ra26_15 hit"); end default: begin : d15 initial $display("@Ra26_15 def"); end endcase
  case (4'h0) ((13 || 2'sh0) >= 4'sh4): begin : h16 initial $display("@Ra26_16 hit"); end default: begin : d16 initial $display("@Ra26_16 def"); end endcase
  case (64'h0) ((13 || 2'sh0) >= 4'sh4): begin : h17 initial $display("@Ra26_17 hit"); end default: begin : d17 initial $display("@Ra26_17 def"); end endcase
  case (0) ((13 || 2'sh0) >= 4'sh4): begin : h18 initial $display("@Ra26_18 hit"); end default: begin : d18 initial $display("@Ra26_18 def"); end endcase
  case (0) ((13 || 2'sh0) >= 4'sh4): begin : h19 initial $display("@Ra26_19 hit"); end default: begin : d19 initial $display("@Ra26_19 def"); end endcase
  case (1'h0) (!P4): begin : h20 initial $display("@Ra26_20 hit"); end default: begin : d20 initial $display("@Ra26_20 def"); end endcase
  case (1'sh0) (!P4): begin : h21 initial $display("@Ra26_21 hit"); end default: begin : d21 initial $display("@Ra26_21 def"); end endcase
  case (4'h0) (!P4): begin : h22 initial $display("@Ra26_22 hit"); end default: begin : d22 initial $display("@Ra26_22 def"); end endcase
  case (64'h0) (!P4): begin : h23 initial $display("@Ra26_23 hit"); end default: begin : d23 initial $display("@Ra26_23 def"); end endcase
  case (0) (!P4): begin : h24 initial $display("@Ra26_24 hit"); end default: begin : d24 initial $display("@Ra26_24 def"); end endcase
endmodule
