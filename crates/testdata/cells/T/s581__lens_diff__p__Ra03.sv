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
  case (8'h9c) S8: begin : h0 initial $display("@Ra3_0 hit"); end default: begin : d0 initial $display("@Ra3_0 def"); end endcase
  case (8'sh9c) S8: begin : h1 initial $display("@Ra3_1 hit"); end default: begin : d1 initial $display("@Ra3_1 def"); end endcase
  case (11'h9c) S8: begin : h2 initial $display("@Ra3_2 hit"); end default: begin : d2 initial $display("@Ra3_2 def"); end endcase
  case (64'hffffffffffffff9c) S8: begin : h3 initial $display("@Ra3_3 hit"); end default: begin : d3 initial $display("@Ra3_3 def"); end endcase
  case ((-100)) S8: begin : h4 initial $display("@Ra3_4 hit"); end default: begin : d4 initial $display("@Ra3_4 def"); end endcase
  case (156) S8: begin : h5 initial $display("@Ra3_5 hit"); end default: begin : d5 initial $display("@Ra3_5 def"); end endcase
  case (1'h1) (S8 || 33'h17abec539): begin : h6 initial $display("@Ra3_6 hit"); end default: begin : d6 initial $display("@Ra3_6 def"); end endcase
  case (1'sh1) (S8 || 33'h17abec539): begin : h7 initial $display("@Ra3_7 hit"); end default: begin : d7 initial $display("@Ra3_7 def"); end endcase
  case (4'h1) (S8 || 33'h17abec539): begin : h8 initial $display("@Ra3_8 hit"); end default: begin : d8 initial $display("@Ra3_8 def"); end endcase
  case (64'hffffffffffffffff) (S8 || 33'h17abec539): begin : h9 initial $display("@Ra3_9 hit"); end default: begin : d9 initial $display("@Ra3_9 def"); end endcase
  case ((-1)) (S8 || 33'h17abec539): begin : h10 initial $display("@Ra3_10 hit"); end default: begin : d10 initial $display("@Ra3_10 def"); end endcase
  case (1) (S8 || 33'h17abec539): begin : h11 initial $display("@Ra3_11 hit"); end default: begin : d11 initial $display("@Ra3_11 def"); end endcase
  case (32'hfffffff8) ((-8) | $unsigned({4'(1'sh1), P8})): begin : h12 initial $display("@Ra3_12 hit"); end default: begin : d12 initial $display("@Ra3_12 def"); end endcase
  case (32'shfffffff8) ((-8) | $unsigned({4'(1'sh1), P8})): begin : h13 initial $display("@Ra3_13 hit"); end default: begin : d13 initial $display("@Ra3_13 def"); end endcase
  case (35'hfffffff8) ((-8) | $unsigned({4'(1'sh1), P8})): begin : h14 initial $display("@Ra3_14 hit"); end default: begin : d14 initial $display("@Ra3_14 def"); end endcase
  case (64'hfffffffffffffff8) ((-8) | $unsigned({4'(1'sh1), P8})): begin : h15 initial $display("@Ra3_15 hit"); end default: begin : d15 initial $display("@Ra3_15 def"); end endcase
  case ((-8)) ((-8) | $unsigned({4'(1'sh1), P8})): begin : h16 initial $display("@Ra3_16 hit"); end default: begin : d16 initial $display("@Ra3_16 def"); end endcase
  case (8'h9c) S8: begin : h17 initial $display("@Ra3_17 hit"); end default: begin : d17 initial $display("@Ra3_17 def"); end endcase
  case (8'sh9c) S8: begin : h18 initial $display("@Ra3_18 hit"); end default: begin : d18 initial $display("@Ra3_18 def"); end endcase
  case (11'h9c) S8: begin : h19 initial $display("@Ra3_19 hit"); end default: begin : d19 initial $display("@Ra3_19 def"); end endcase
  case (64'hffffffffffffff9c) S8: begin : h20 initial $display("@Ra3_20 hit"); end default: begin : d20 initial $display("@Ra3_20 def"); end endcase
  case ((-100)) S8: begin : h21 initial $display("@Ra3_21 hit"); end default: begin : d21 initial $display("@Ra3_21 def"); end endcase
  case (156) S8: begin : h22 initial $display("@Ra3_22 hit"); end default: begin : d22 initial $display("@Ra3_22 def"); end endcase
  case (1'h0) (!(33'h844a7034 | 32'hdf703017)): begin : h23 initial $display("@Ra3_23 hit"); end default: begin : d23 initial $display("@Ra3_23 def"); end endcase
  case (1'sh0) (!(33'h844a7034 | 32'hdf703017)): begin : h24 initial $display("@Ra3_24 hit"); end default: begin : d24 initial $display("@Ra3_24 def"); end endcase
endmodule
