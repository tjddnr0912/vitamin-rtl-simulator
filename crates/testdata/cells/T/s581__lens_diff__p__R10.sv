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
  case (64'hfffffffffffffff5) ((4'sh1 + 25) ? (5 + U32) : (31 && S65)): begin : h0 initial $display("@10_0 hit"); end default: begin : d0 initial $display("@10_0 def"); end endcase
  case (4294967285) ((4'sh1 + 25) ? (5 + U32) : (31 && S65)): begin : h1 initial $display("@10_1 hit"); end default: begin : d1 initial $display("@10_1 def"); end endcase
  case (1'h0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h2 initial $display("@10_2 hit"); end default: begin : d2 initial $display("@10_2 def"); end endcase
  case (1'sh0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h3 initial $display("@10_3 hit"); end default: begin : d3 initial $display("@10_3 def"); end endcase
  case (4'h0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h4 initial $display("@10_4 hit"); end default: begin : d4 initial $display("@10_4 def"); end endcase
  case (0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h5 initial $display("@10_5 hit"); end default: begin : d5 initial $display("@10_5 def"); end endcase
  case (64'h0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h6 initial $display("@10_6 hit"); end default: begin : d6 initial $display("@10_6 def"); end endcase
  case (0) ((-65'h121bfcbf52ae7c300) < (-2'h0)): begin : h7 initial $display("@10_7 hit"); end default: begin : d7 initial $display("@10_7 def"); end endcase
  case (2'h1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h8 initial $display("@10_8 hit"); end default: begin : d8 initial $display("@10_8 def"); end endcase
  case (2'sh1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h9 initial $display("@10_9 hit"); end default: begin : d9 initial $display("@10_9 def"); end endcase
  case (5'h1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h10 initial $display("@10_10 hit"); end default: begin : d10 initial $display("@10_10 def"); end endcase
  case (1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h11 initial $display("@10_11 hit"); end default: begin : d11 initial $display("@10_11 def"); end endcase
  case (64'h1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h12 initial $display("@10_12 hit"); end default: begin : d12 initial $display("@10_12 def"); end endcase
  case (1) (2'h3 & (32'sh3288ed7 >= P8)): begin : h13 initial $display("@10_13 hit"); end default: begin : d13 initial $display("@10_13 def"); end endcase
  case (64'h8) (~L64): begin : h14 initial $display("@10_14 hit"); end default: begin : d14 initial $display("@10_14 def"); end endcase
  case (64'sh8) (~L64): begin : h15 initial $display("@10_15 hit"); end default: begin : d15 initial $display("@10_15 def"); end endcase
  case (64'h8) (~L64): begin : h16 initial $display("@10_16 hit"); end default: begin : d16 initial $display("@10_16 def"); end endcase
  case (8) (~L64): begin : h17 initial $display("@10_17 hit"); end default: begin : d17 initial $display("@10_17 def"); end endcase
  case (64'h8) (~L64): begin : h18 initial $display("@10_18 hit"); end default: begin : d18 initial $display("@10_18 def"); end endcase
  case (8) (~L64): begin : h19 initial $display("@10_19 hit"); end default: begin : d19 initial $display("@10_19 def"); end endcase
  case (32'hfffffff0) U32: begin : h20 initial $display("@10_20 hit"); end default: begin : d20 initial $display("@10_20 def"); end endcase
  case (32'shfffffff0) U32: begin : h21 initial $display("@10_21 hit"); end default: begin : d21 initial $display("@10_21 def"); end endcase
  case (35'hfffffff0) U32: begin : h22 initial $display("@10_22 hit"); end default: begin : d22 initial $display("@10_22 def"); end endcase
  case ((-16)) U32: begin : h23 initial $display("@10_23 hit"); end default: begin : d23 initial $display("@10_23 def"); end endcase
  case (64'hfffffffffffffff0) U32: begin : h24 initial $display("@10_24 hit"); end default: begin : d24 initial $display("@10_24 def"); end endcase
endmodule
