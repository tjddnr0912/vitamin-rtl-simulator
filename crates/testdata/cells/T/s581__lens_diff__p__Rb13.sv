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
  case (1) ({P65, S4} || {3'((-5)), P65}): begin : h0 initial $display("@Rb13_0 hit"); end default: begin : d0 initial $display("@Rb13_0 def"); end endcase
  case (32'h20) (P65 ? 32 : (-1)): begin : h1 initial $display("@Rb13_1 hit"); end default: begin : d1 initial $display("@Rb13_1 def"); end endcase
  case (32'sh20) (P65 ? 32 : (-1)): begin : h2 initial $display("@Rb13_2 hit"); end default: begin : d2 initial $display("@Rb13_2 def"); end endcase
  case (35'h20) (P65 ? 32 : (-1)): begin : h3 initial $display("@Rb13_3 hit"); end default: begin : d3 initial $display("@Rb13_3 def"); end endcase
  case (64'h20) (P65 ? 32 : (-1)): begin : h4 initial $display("@Rb13_4 hit"); end default: begin : d4 initial $display("@Rb13_4 def"); end endcase
  case (32) (P65 ? 32 : (-1)): begin : h5 initial $display("@Rb13_5 hit"); end default: begin : d5 initial $display("@Rb13_5 def"); end endcase
  case (32) (P65 ? 32 : (-1)): begin : h6 initial $display("@Rb13_6 hit"); end default: begin : d6 initial $display("@Rb13_6 def"); end endcase
  case (1'h1) (16'sh8174 < 5): begin : h7 initial $display("@Rb13_7 hit"); end default: begin : d7 initial $display("@Rb13_7 def"); end endcase
  case (1'sh1) (16'sh8174 < 5): begin : h8 initial $display("@Rb13_8 hit"); end default: begin : d8 initial $display("@Rb13_8 def"); end endcase
  case (4'h1) (16'sh8174 < 5): begin : h9 initial $display("@Rb13_9 hit"); end default: begin : d9 initial $display("@Rb13_9 def"); end endcase
  case (64'hffffffffffffffff) (16'sh8174 < 5): begin : h10 initial $display("@Rb13_10 hit"); end default: begin : d10 initial $display("@Rb13_10 def"); end endcase
  case ((-1)) (16'sh8174 < 5): begin : h11 initial $display("@Rb13_11 hit"); end default: begin : d11 initial $display("@Rb13_11 def"); end endcase
  case (1) (16'sh8174 < 5): begin : h12 initial $display("@Rb13_12 hit"); end default: begin : d12 initial $display("@Rb13_12 def"); end endcase
  case (16'h1cac) ({S4, 8'sh9C} - {2{P8}}): begin : h13 initial $display("@Rb13_13 hit"); end default: begin : d13 initial $display("@Rb13_13 def"); end endcase
  case (16'sh1cac) ({S4, 8'sh9C} - {2{P8}}): begin : h14 initial $display("@Rb13_14 hit"); end default: begin : d14 initial $display("@Rb13_14 def"); end endcase
  case (19'h1cac) ({S4, 8'sh9C} - {2{P8}}): begin : h15 initial $display("@Rb13_15 hit"); end default: begin : d15 initial $display("@Rb13_15 def"); end endcase
  case (64'h1cac) ({S4, 8'sh9C} - {2{P8}}): begin : h16 initial $display("@Rb13_16 hit"); end default: begin : d16 initial $display("@Rb13_16 def"); end endcase
  case (7340) ({S4, 8'sh9C} - {2{P8}}): begin : h17 initial $display("@Rb13_17 hit"); end default: begin : d17 initial $display("@Rb13_17 def"); end endcase
  case (7340) ({S4, 8'sh9C} - {2{P8}}): begin : h18 initial $display("@Rb13_18 hit"); end default: begin : d18 initial $display("@Rb13_18 def"); end endcase
  case (32'hfffffffb) (-5): begin : h19 initial $display("@Rb13_19 hit"); end default: begin : d19 initial $display("@Rb13_19 def"); end endcase
  case (32'shfffffffb) (-5): begin : h20 initial $display("@Rb13_20 hit"); end default: begin : d20 initial $display("@Rb13_20 def"); end endcase
  case (35'hfffffffb) (-5): begin : h21 initial $display("@Rb13_21 hit"); end default: begin : d21 initial $display("@Rb13_21 def"); end endcase
  case (64'hfffffffffffffffb) (-5): begin : h22 initial $display("@Rb13_22 hit"); end default: begin : d22 initial $display("@Rb13_22 def"); end endcase
  case ((-5)) (-5): begin : h23 initial $display("@Rb13_23 hit"); end default: begin : d23 initial $display("@Rb13_23 def"); end endcase
  case (1'h0) (!(S8 >>> 2)): begin : h24 initial $display("@Rb13_24 hit"); end default: begin : d24 initial $display("@Rb13_24 def"); end endcase
endmodule
