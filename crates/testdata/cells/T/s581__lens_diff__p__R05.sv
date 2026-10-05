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
  case (1) ({2{65'(8'h8d)}} && 32'shd7925ffa): begin : h0 initial $display("@5_0 hit"); end default: begin : d0 initial $display("@5_0 def"); end endcase
  case (4'hc) $signed(P4): begin : h1 initial $display("@5_1 hit"); end default: begin : d1 initial $display("@5_1 def"); end endcase
  case (4'shc) $signed(P4): begin : h2 initial $display("@5_2 hit"); end default: begin : d2 initial $display("@5_2 def"); end endcase
  case (7'hc) $signed(P4): begin : h3 initial $display("@5_3 hit"); end default: begin : d3 initial $display("@5_3 def"); end endcase
  case ((-4)) $signed(P4): begin : h4 initial $display("@5_4 hit"); end default: begin : d4 initial $display("@5_4 def"); end endcase
  case (64'hfffffffffffffffc) $signed(P4): begin : h5 initial $display("@5_5 hit"); end default: begin : d5 initial $display("@5_5 def"); end endcase
  case (12) $signed(P4): begin : h6 initial $display("@5_6 hit"); end default: begin : d6 initial $display("@5_6 def"); end endcase
  case (32'he6666667) (-(I / UN)): begin : h7 initial $display("@5_7 hit"); end default: begin : d7 initial $display("@5_7 def"); end endcase
  case (32'she6666667) (-(I / UN)): begin : h8 initial $display("@5_8 hit"); end default: begin : d8 initial $display("@5_8 def"); end endcase
  case (35'he6666667) (-(I / UN)): begin : h9 initial $display("@5_9 hit"); end default: begin : d9 initial $display("@5_9 def"); end endcase
  case ((-429496729)) (-(I / UN)): begin : h10 initial $display("@5_10 hit"); end default: begin : d10 initial $display("@5_10 def"); end endcase
  case (64'hffffffffe6666667) (-(I / UN)): begin : h11 initial $display("@5_11 hit"); end default: begin : d11 initial $display("@5_11 def"); end endcase
  case (3865470567) (-(I / UN)): begin : h12 initial $display("@5_12 hit"); end default: begin : d12 initial $display("@5_12 def"); end endcase
  case (3'h7) 3'h7: begin : h13 initial $display("@5_13 hit"); end default: begin : d13 initial $display("@5_13 def"); end endcase
  case (3'sh7) 3'h7: begin : h14 initial $display("@5_14 hit"); end default: begin : d14 initial $display("@5_14 def"); end endcase
  case (6'h7) 3'h7: begin : h15 initial $display("@5_15 hit"); end default: begin : d15 initial $display("@5_15 def"); end endcase
  case ((-1)) 3'h7: begin : h16 initial $display("@5_16 hit"); end default: begin : d16 initial $display("@5_16 def"); end endcase
  case (64'hffffffffffffffff) 3'h7: begin : h17 initial $display("@5_17 hit"); end default: begin : d17 initial $display("@5_17 def"); end endcase
  case (7) 3'h7: begin : h18 initial $display("@5_18 hit"); end default: begin : d18 initial $display("@5_18 def"); end endcase
  case (3'h3) $signed(3'h3): begin : h19 initial $display("@5_19 hit"); end default: begin : d19 initial $display("@5_19 def"); end endcase
  case (3'sh3) $signed(3'h3): begin : h20 initial $display("@5_20 hit"); end default: begin : d20 initial $display("@5_20 def"); end endcase
  case (6'h3) $signed(3'h3): begin : h21 initial $display("@5_21 hit"); end default: begin : d21 initial $display("@5_21 def"); end endcase
  case (3) $signed(3'h3): begin : h22 initial $display("@5_22 hit"); end default: begin : d22 initial $display("@5_22 def"); end endcase
  case (64'h3) $signed(3'h3): begin : h23 initial $display("@5_23 hit"); end default: begin : d23 initial $display("@5_23 def"); end endcase
  case (3) $signed(3'h3): begin : h24 initial $display("@5_24 hit"); end default: begin : d24 initial $display("@5_24 def"); end endcase
endmodule
