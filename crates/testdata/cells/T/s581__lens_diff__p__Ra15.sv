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
  case (64'h0) (!63'h34f43ee22dd113c): begin : h0 initial $display("@Ra15_0 hit"); end default: begin : d0 initial $display("@Ra15_0 def"); end endcase
  case (0) (!63'h34f43ee22dd113c): begin : h1 initial $display("@Ra15_1 hit"); end default: begin : d1 initial $display("@Ra15_1 def"); end endcase
  case (0) (!63'h34f43ee22dd113c): begin : h2 initial $display("@Ra15_2 hit"); end default: begin : d2 initial $display("@Ra15_2 def"); end endcase
  case (64'hd541da56770b0501) (31'sh67ceae82 ^ 64'hd541da5610c5ab83): begin : h3 initial $display("@Ra15_3 hit"); end default: begin : d3 initial $display("@Ra15_3 def"); end endcase
  case (64'shd541da56770b0501) (31'sh67ceae82 ^ 64'hd541da5610c5ab83): begin : h4 initial $display("@Ra15_4 hit"); end default: begin : d4 initial $display("@Ra15_4 def"); end endcase
  case (64'hd541da56770b0501) (31'sh67ceae82 ^ 64'hd541da5610c5ab83): begin : h5 initial $display("@Ra15_5 hit"); end default: begin : d5 initial $display("@Ra15_5 def"); end endcase
  case (64'hd541da56770b0501) (31'sh67ceae82 ^ 64'hd541da5610c5ab83): begin : h6 initial $display("@Ra15_6 hit"); end default: begin : d6 initial $display("@Ra15_6 def"); end endcase
  case (31'h35c1e9d7) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h7 initial $display("@Ra15_7 hit"); end default: begin : d7 initial $display("@Ra15_7 def"); end endcase
  case (31'sh35c1e9d7) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h8 initial $display("@Ra15_8 hit"); end default: begin : d8 initial $display("@Ra15_8 def"); end endcase
  case (34'h35c1e9d7) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h9 initial $display("@Ra15_9 hit"); end default: begin : d9 initial $display("@Ra15_9 def"); end endcase
  case (64'h35c1e9d7) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h10 initial $display("@Ra15_10 hit"); end default: begin : d10 initial $display("@Ra15_10 def"); end endcase
  case (901900759) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h11 initial $display("@Ra15_11 hit"); end default: begin : d11 initial $display("@Ra15_11 def"); end endcase
  case (901900759) ($unsigned(16'shc8ed) - $unsigned(31'h4a3edf16)): begin : h12 initial $display("@Ra15_12 hit"); end default: begin : d12 initial $display("@Ra15_12 def"); end endcase
  case (1'h0) (16'h5f18 == 8'sh14): begin : h13 initial $display("@Ra15_13 hit"); end default: begin : d13 initial $display("@Ra15_13 def"); end endcase
  case (1'sh0) (16'h5f18 == 8'sh14): begin : h14 initial $display("@Ra15_14 hit"); end default: begin : d14 initial $display("@Ra15_14 def"); end endcase
  case (4'h0) (16'h5f18 == 8'sh14): begin : h15 initial $display("@Ra15_15 hit"); end default: begin : d15 initial $display("@Ra15_15 def"); end endcase
  case (64'h0) (16'h5f18 == 8'sh14): begin : h16 initial $display("@Ra15_16 hit"); end default: begin : d16 initial $display("@Ra15_16 def"); end endcase
  case (0) (16'h5f18 == 8'sh14): begin : h17 initial $display("@Ra15_17 hit"); end default: begin : d17 initial $display("@Ra15_17 def"); end endcase
  case (0) (16'h5f18 == 8'sh14): begin : h18 initial $display("@Ra15_18 hit"); end default: begin : d18 initial $display("@Ra15_18 def"); end endcase
  case (64'h1f) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h19 initial $display("@Ra15_19 hit"); end default: begin : d19 initial $display("@Ra15_19 def"); end endcase
  case (64'sh1f) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h20 initial $display("@Ra15_20 hit"); end default: begin : d20 initial $display("@Ra15_20 def"); end endcase
  case (64'h1f) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h21 initial $display("@Ra15_21 hit"); end default: begin : d21 initial $display("@Ra15_21 def"); end endcase
  case (64'h1f) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h22 initial $display("@Ra15_22 hit"); end default: begin : d22 initial $display("@Ra15_22 def"); end endcase
  case (31) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h23 initial $display("@Ra15_23 hit"); end default: begin : d23 initial $display("@Ra15_23 def"); end endcase
  case (31) $signed((5'h1f % 64'sh4a7d1dbc263cc4dc)): begin : h24 initial $display("@Ra15_24 hit"); end default: begin : d24 initial $display("@Ra15_24 def"); end endcase
endmodule
