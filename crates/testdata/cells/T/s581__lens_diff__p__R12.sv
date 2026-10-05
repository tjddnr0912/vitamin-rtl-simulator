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
  case (8'h0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h0 initial $display("@12_0 hit"); end default: begin : d0 initial $display("@12_0 def"); end endcase
  case (8'sh0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h1 initial $display("@12_1 hit"); end default: begin : d1 initial $display("@12_1 def"); end endcase
  case (11'h0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h2 initial $display("@12_2 hit"); end default: begin : d2 initial $display("@12_2 def"); end endcase
  case (0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h3 initial $display("@12_3 hit"); end default: begin : d3 initial $display("@12_3 def"); end endcase
  case (64'h0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h4 initial $display("@12_4 hit"); end default: begin : d4 initial $display("@12_4 def"); end endcase
  case (0) $unsigned(((!16'h1ad7) & 8'h95)): begin : h5 initial $display("@12_5 hit"); end default: begin : d5 initial $display("@12_5 def"); end endcase
  case (1'h0) (!(P65 & P65)): begin : h6 initial $display("@12_6 hit"); end default: begin : d6 initial $display("@12_6 def"); end endcase
  case (1'sh0) (!(P65 & P65)): begin : h7 initial $display("@12_7 hit"); end default: begin : d7 initial $display("@12_7 def"); end endcase
  case (4'h0) (!(P65 & P65)): begin : h8 initial $display("@12_8 hit"); end default: begin : d8 initial $display("@12_8 def"); end endcase
  case (0) (!(P65 & P65)): begin : h9 initial $display("@12_9 hit"); end default: begin : d9 initial $display("@12_9 def"); end endcase
  case (64'h0) (!(P65 & P65)): begin : h10 initial $display("@12_10 hit"); end default: begin : d10 initial $display("@12_10 def"); end endcase
  case (0) (!(P65 & P65)): begin : h11 initial $display("@12_11 hit"); end default: begin : d11 initial $display("@12_11 def"); end endcase
  case (2'h1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h12 initial $display("@12_12 hit"); end default: begin : d12 initial $display("@12_12 def"); end endcase
  case (2'sh1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h13 initial $display("@12_13 hit"); end default: begin : d13 initial $display("@12_13 def"); end endcase
  case (5'h1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h14 initial $display("@12_14 hit"); end default: begin : d14 initial $display("@12_14 def"); end endcase
  case (1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h15 initial $display("@12_15 hit"); end default: begin : d15 initial $display("@12_15 def"); end endcase
  case (64'h1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h16 initial $display("@12_16 hit"); end default: begin : d16 initial $display("@12_16 def"); end endcase
  case (1) ($signed((-2'h3)) << ({1'(31'sh5aee796d), 4'(3'h0)} / {1'(S4), 4'hB})): begin : h17 initial $display("@12_17 hit"); end default: begin : d17 initial $display("@12_17 def"); end endcase
  case (4'h3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h18 initial $display("@12_18 hit"); end default: begin : d18 initial $display("@12_18 def"); end endcase
  case (4'sh3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h19 initial $display("@12_19 hit"); end default: begin : d19 initial $display("@12_19 def"); end endcase
  case (7'h3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h20 initial $display("@12_20 hit"); end default: begin : d20 initial $display("@12_20 def"); end endcase
  case (3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h21 initial $display("@12_21 hit"); end default: begin : d21 initial $display("@12_21 def"); end endcase
  case (64'h3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h22 initial $display("@12_22 hit"); end default: begin : d22 initial $display("@12_22 def"); end endcase
  case (3) ({2{S4}} ? (-S4) : (UN >= 8'sh70)): begin : h23 initial $display("@12_23 hit"); end default: begin : d23 initial $display("@12_23 def"); end endcase
  case (8'hf0) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h24 initial $display("@12_24 hit"); end default: begin : d24 initial $display("@12_24 def"); end endcase
endmodule
