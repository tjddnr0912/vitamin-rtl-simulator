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
  case (4294967280) U32: begin : h0 initial $display("@11_0 hit"); end default: begin : d0 initial $display("@11_0 def"); end endcase
  case (32'h26) 38: begin : h1 initial $display("@11_1 hit"); end default: begin : d1 initial $display("@11_1 def"); end endcase
  case (32'sh26) 38: begin : h2 initial $display("@11_2 hit"); end default: begin : d2 initial $display("@11_2 def"); end endcase
  case (35'h26) 38: begin : h3 initial $display("@11_3 hit"); end default: begin : d3 initial $display("@11_3 def"); end endcase
  case (38) 38: begin : h4 initial $display("@11_4 hit"); end default: begin : d4 initial $display("@11_4 def"); end endcase
  case (64'h26) 38: begin : h5 initial $display("@11_5 hit"); end default: begin : d5 initial $display("@11_5 def"); end endcase
  case (38) 38: begin : h6 initial $display("@11_6 hit"); end default: begin : d6 initial $display("@11_6 def"); end endcase
  case (16'h1dbb) 16'h1dbb: begin : h7 initial $display("@11_7 hit"); end default: begin : d7 initial $display("@11_7 def"); end endcase
  case (16'sh1dbb) 16'h1dbb: begin : h8 initial $display("@11_8 hit"); end default: begin : d8 initial $display("@11_8 def"); end endcase
  case (19'h1dbb) 16'h1dbb: begin : h9 initial $display("@11_9 hit"); end default: begin : d9 initial $display("@11_9 def"); end endcase
  case (7611) 16'h1dbb: begin : h10 initial $display("@11_10 hit"); end default: begin : d10 initial $display("@11_10 def"); end endcase
  case (64'h1dbb) 16'h1dbb: begin : h11 initial $display("@11_11 hit"); end default: begin : d11 initial $display("@11_11 def"); end endcase
  case (7611) 16'h1dbb: begin : h12 initial $display("@11_12 hit"); end default: begin : d12 initial $display("@11_12 def"); end endcase
  case (5'h13) ((5'h12 + 1'h0) ^ (I < S4)): begin : h13 initial $display("@11_13 hit"); end default: begin : d13 initial $display("@11_13 def"); end endcase
  case (5'sh13) ((5'h12 + 1'h0) ^ (I < S4)): begin : h14 initial $display("@11_14 hit"); end default: begin : d14 initial $display("@11_14 def"); end endcase
  case (8'h13) ((5'h12 + 1'h0) ^ (I < S4)): begin : h15 initial $display("@11_15 hit"); end default: begin : d15 initial $display("@11_15 def"); end endcase
  case ((-13)) ((5'h12 + 1'h0) ^ (I < S4)): begin : h16 initial $display("@11_16 hit"); end default: begin : d16 initial $display("@11_16 def"); end endcase
  case (64'hfffffffffffffff3) ((5'h12 + 1'h0) ^ (I < S4)): begin : h17 initial $display("@11_17 hit"); end default: begin : d17 initial $display("@11_17 def"); end endcase
  case (19) ((5'h12 + 1'h0) ^ (I < S4)): begin : h18 initial $display("@11_18 hit"); end default: begin : d18 initial $display("@11_18 def"); end endcase
  case (1'h1) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h19 initial $display("@11_19 hit"); end default: begin : d19 initial $display("@11_19 def"); end endcase
  case (1'sh1) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h20 initial $display("@11_20 hit"); end default: begin : d20 initial $display("@11_20 def"); end endcase
  case (4'h1) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h21 initial $display("@11_21 hit"); end default: begin : d21 initial $display("@11_21 def"); end endcase
  case ((-1)) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h22 initial $display("@11_22 hit"); end default: begin : d22 initial $display("@11_22 def"); end endcase
  case (64'hffffffffffffffff) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h23 initial $display("@11_23 hit"); end default: begin : d23 initial $display("@11_23 def"); end endcase
  case (1) ((UN ^ U32) || (33'h16d7c59a | P4)): begin : h24 initial $display("@11_24 hit"); end default: begin : d24 initial $display("@11_24 def"); end endcase
endmodule
