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
  case (3'h5) 3'sh5: begin : h0 initial $display("@Rb2_0 hit"); end default: begin : d0 initial $display("@Rb2_0 def"); end endcase
  case (3'sh5) 3'sh5: begin : h1 initial $display("@Rb2_1 hit"); end default: begin : d1 initial $display("@Rb2_1 def"); end endcase
  case (6'h5) 3'sh5: begin : h2 initial $display("@Rb2_2 hit"); end default: begin : d2 initial $display("@Rb2_2 def"); end endcase
  case (64'hfffffffffffffffd) 3'sh5: begin : h3 initial $display("@Rb2_3 hit"); end default: begin : d3 initial $display("@Rb2_3 def"); end endcase
  case ((-3)) 3'sh5: begin : h4 initial $display("@Rb2_4 hit"); end default: begin : d4 initial $display("@Rb2_4 def"); end endcase
  case (5) 3'sh5: begin : h5 initial $display("@Rb2_5 hit"); end default: begin : d5 initial $display("@Rb2_5 def"); end endcase
  case (1'h0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h6 initial $display("@Rb2_6 hit"); end default: begin : d6 initial $display("@Rb2_6 def"); end endcase
  case (1'sh0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h7 initial $display("@Rb2_7 hit"); end default: begin : d7 initial $display("@Rb2_7 def"); end endcase
  case (4'h0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h8 initial $display("@Rb2_8 hit"); end default: begin : d8 initial $display("@Rb2_8 def"); end endcase
  case (64'h0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h9 initial $display("@Rb2_9 hit"); end default: begin : d9 initial $display("@Rb2_9 def"); end endcase
  case (0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h10 initial $display("@Rb2_10 hit"); end default: begin : d10 initial $display("@Rb2_10 def"); end endcase
  case (0) (((U32 / (-5)) >= {65'(34), 3'(P8)}) && P65): begin : h11 initial $display("@Rb2_11 hit"); end default: begin : d11 initial $display("@Rb2_11 def"); end endcase
  case (1'h0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h12 initial $display("@Rb2_12 hit"); end default: begin : d12 initial $display("@Rb2_12 def"); end endcase
  case (1'sh0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h13 initial $display("@Rb2_13 hit"); end default: begin : d13 initial $display("@Rb2_13 def"); end endcase
  case (4'h0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h14 initial $display("@Rb2_14 hit"); end default: begin : d14 initial $display("@Rb2_14 def"); end endcase
  case (64'h0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h15 initial $display("@Rb2_15 hit"); end default: begin : d15 initial $display("@Rb2_15 def"); end endcase
  case (0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h16 initial $display("@Rb2_16 hit"); end default: begin : d16 initial $display("@Rb2_16 def"); end endcase
  case (0) $unsigned(((-19) == 64'h31aa3d014b906d61)): begin : h17 initial $display("@Rb2_17 hit"); end default: begin : d17 initial $display("@Rb2_17 def"); end endcase
  case (1'h1) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h18 initial $display("@Rb2_18 hit"); end default: begin : d18 initial $display("@Rb2_18 def"); end endcase
  case (1'sh1) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h19 initial $display("@Rb2_19 hit"); end default: begin : d19 initial $display("@Rb2_19 def"); end endcase
  case (4'h1) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h20 initial $display("@Rb2_20 hit"); end default: begin : d20 initial $display("@Rb2_20 def"); end endcase
  case (64'hffffffffffffffff) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h21 initial $display("@Rb2_21 hit"); end default: begin : d21 initial $display("@Rb2_21 def"); end endcase
  case ((-1)) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h22 initial $display("@Rb2_22 hit"); end default: begin : d22 initial $display("@Rb2_22 def"); end endcase
  case (1) (((P8 && 64'h8fd78f206b0cbbac) ^ (S65 < S65)) || {8'sh9C, 33'((8'h1 >>> 3))}): begin : h23 initial $display("@Rb2_23 hit"); end default: begin : d23 initial $display("@Rb2_23 def"); end endcase
  case (1'h0) (!I): begin : h24 initial $display("@Rb2_24 hit"); end default: begin : d24 initial $display("@Rb2_24 def"); end endcase
endmodule
