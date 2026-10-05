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
  case (64'hfffffffffffffff7) L64: begin : h0 initial $display("@Rb25_0 hit"); end default: begin : d0 initial $display("@Rb25_0 def"); end endcase
  case (64'shfffffffffffffff7) L64: begin : h1 initial $display("@Rb25_1 hit"); end default: begin : d1 initial $display("@Rb25_1 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h2 initial $display("@Rb25_2 hit"); end default: begin : d2 initial $display("@Rb25_2 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h3 initial $display("@Rb25_3 hit"); end default: begin : d3 initial $display("@Rb25_3 def"); end endcase
  case ((-9)) L64: begin : h4 initial $display("@Rb25_4 hit"); end default: begin : d4 initial $display("@Rb25_4 def"); end endcase
  case (64'h9) (-$signed(L64)): begin : h5 initial $display("@Rb25_5 hit"); end default: begin : d5 initial $display("@Rb25_5 def"); end endcase
  case (64'sh9) (-$signed(L64)): begin : h6 initial $display("@Rb25_6 hit"); end default: begin : d6 initial $display("@Rb25_6 def"); end endcase
  case (64'h9) (-$signed(L64)): begin : h7 initial $display("@Rb25_7 hit"); end default: begin : d7 initial $display("@Rb25_7 def"); end endcase
  case (64'h9) (-$signed(L64)): begin : h8 initial $display("@Rb25_8 hit"); end default: begin : d8 initial $display("@Rb25_8 def"); end endcase
  case (9) (-$signed(L64)): begin : h9 initial $display("@Rb25_9 hit"); end default: begin : d9 initial $display("@Rb25_9 def"); end endcase
  case (9) (-$signed(L64)): begin : h10 initial $display("@Rb25_10 hit"); end default: begin : d10 initial $display("@Rb25_10 def"); end endcase
  case (3'h3) 3'h3: begin : h11 initial $display("@Rb25_11 hit"); end default: begin : d11 initial $display("@Rb25_11 def"); end endcase
  case (3'sh3) 3'h3: begin : h12 initial $display("@Rb25_12 hit"); end default: begin : d12 initial $display("@Rb25_12 def"); end endcase
  case (6'h3) 3'h3: begin : h13 initial $display("@Rb25_13 hit"); end default: begin : d13 initial $display("@Rb25_13 def"); end endcase
  case (64'h3) 3'h3: begin : h14 initial $display("@Rb25_14 hit"); end default: begin : d14 initial $display("@Rb25_14 def"); end endcase
  case (3) 3'h3: begin : h15 initial $display("@Rb25_15 hit"); end default: begin : d15 initial $display("@Rb25_15 def"); end endcase
  case (3) 3'h3: begin : h16 initial $display("@Rb25_16 hit"); end default: begin : d16 initial $display("@Rb25_16 def"); end endcase
  case (32'h198ee7d8) 32'h198ee7d8: begin : h17 initial $display("@Rb25_17 hit"); end default: begin : d17 initial $display("@Rb25_17 def"); end endcase
  case (32'sh198ee7d8) 32'h198ee7d8: begin : h18 initial $display("@Rb25_18 hit"); end default: begin : d18 initial $display("@Rb25_18 def"); end endcase
  case (35'h198ee7d8) 32'h198ee7d8: begin : h19 initial $display("@Rb25_19 hit"); end default: begin : d19 initial $display("@Rb25_19 def"); end endcase
  case (64'h198ee7d8) 32'h198ee7d8: begin : h20 initial $display("@Rb25_20 hit"); end default: begin : d20 initial $display("@Rb25_20 def"); end endcase
  case (428795864) 32'h198ee7d8: begin : h21 initial $display("@Rb25_21 hit"); end default: begin : d21 initial $display("@Rb25_21 def"); end endcase
  case (428795864) 32'h198ee7d8: begin : h22 initial $display("@Rb25_22 hit"); end default: begin : d22 initial $display("@Rb25_22 def"); end endcase
  case (4'ha) UN: begin : h23 initial $display("@Rb25_23 hit"); end default: begin : d23 initial $display("@Rb25_23 def"); end endcase
  case (4'sha) UN: begin : h24 initial $display("@Rb25_24 hit"); end default: begin : d24 initial $display("@Rb25_24 def"); end endcase
endmodule
