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
  case (8'shf0) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h0 initial $display("@13_0 hit"); end default: begin : d0 initial $display("@13_0 def"); end endcase
  case (11'hf0) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h1 initial $display("@13_1 hit"); end default: begin : d1 initial $display("@13_1 def"); end endcase
  case ((-16)) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h2 initial $display("@13_2 hit"); end default: begin : d2 initial $display("@13_2 def"); end endcase
  case (64'hfffffffffffffff0) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h3 initial $display("@13_3 hit"); end default: begin : d3 initial $display("@13_3 def"); end endcase
  case (240) ((-S65) ? $signed(P8) : (P8 + 8'sh7d)): begin : h4 initial $display("@13_4 hit"); end default: begin : d4 initial $display("@13_4 def"); end endcase
  case (37'h1fffffffff) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h5 initial $display("@13_5 hit"); end default: begin : d5 initial $display("@13_5 def"); end endcase
  case (37'sh1fffffffff) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h6 initial $display("@13_6 hit"); end default: begin : d6 initial $display("@13_6 def"); end endcase
  case (40'h1fffffffff) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h7 initial $display("@13_7 hit"); end default: begin : d7 initial $display("@13_7 def"); end endcase
  case ((-1)) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h8 initial $display("@13_8 hit"); end default: begin : d8 initial $display("@13_8 def"); end endcase
  case (64'hffffffffffffffff) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h9 initial $display("@13_9 hit"); end default: begin : d9 initial $display("@13_9 def"); end endcase
  case (137438953471) ((-UN) | {4'(5'sh1f), 33'(11)}): begin : h10 initial $display("@13_10 hit"); end default: begin : d10 initial $display("@13_10 def"); end endcase
  case (1'h0) ($unsigned(2'h2) >= (-UN)): begin : h11 initial $display("@13_11 hit"); end default: begin : d11 initial $display("@13_11 def"); end endcase
  case (1'sh0) ($unsigned(2'h2) >= (-UN)): begin : h12 initial $display("@13_12 hit"); end default: begin : d12 initial $display("@13_12 def"); end endcase
  case (4'h0) ($unsigned(2'h2) >= (-UN)): begin : h13 initial $display("@13_13 hit"); end default: begin : d13 initial $display("@13_13 def"); end endcase
  case (0) ($unsigned(2'h2) >= (-UN)): begin : h14 initial $display("@13_14 hit"); end default: begin : d14 initial $display("@13_14 def"); end endcase
  case (64'h0) ($unsigned(2'h2) >= (-UN)): begin : h15 initial $display("@13_15 hit"); end default: begin : d15 initial $display("@13_15 def"); end endcase
  case (0) ($unsigned(2'h2) >= (-UN)): begin : h16 initial $display("@13_16 hit"); end default: begin : d16 initial $display("@13_16 def"); end endcase
endmodule
