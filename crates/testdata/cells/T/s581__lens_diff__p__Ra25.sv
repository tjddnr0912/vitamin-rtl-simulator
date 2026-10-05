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
  case (64'shdfdc0fcabb88a50) ((-64'hd8799bfef27c07f5) * (2'h0 ? 63'sh55b96f03ebbf2dac : P8)): begin : h0 initial $display("@Ra25_0 hit"); end default: begin : d0 initial $display("@Ra25_0 def"); end endcase
  case (64'hdfdc0fcabb88a50) ((-64'hd8799bfef27c07f5) * (2'h0 ? 63'sh55b96f03ebbf2dac : P8)): begin : h1 initial $display("@Ra25_1 hit"); end default: begin : d1 initial $display("@Ra25_1 def"); end endcase
  case (64'hdfdc0fcabb88a50) ((-64'hd8799bfef27c07f5) * (2'h0 ? 63'sh55b96f03ebbf2dac : P8)): begin : h2 initial $display("@Ra25_2 hit"); end default: begin : d2 initial $display("@Ra25_2 def"); end endcase
  case (4'hf) 4'shf: begin : h3 initial $display("@Ra25_3 hit"); end default: begin : d3 initial $display("@Ra25_3 def"); end endcase
  case (4'shf) 4'shf: begin : h4 initial $display("@Ra25_4 hit"); end default: begin : d4 initial $display("@Ra25_4 def"); end endcase
  case (7'hf) 4'shf: begin : h5 initial $display("@Ra25_5 hit"); end default: begin : d5 initial $display("@Ra25_5 def"); end endcase
  case (64'hffffffffffffffff) 4'shf: begin : h6 initial $display("@Ra25_6 hit"); end default: begin : d6 initial $display("@Ra25_6 def"); end endcase
  case ((-1)) 4'shf: begin : h7 initial $display("@Ra25_7 hit"); end default: begin : d7 initial $display("@Ra25_7 def"); end endcase
  case (15) 4'shf: begin : h8 initial $display("@Ra25_8 hit"); end default: begin : d8 initial $display("@Ra25_8 def"); end endcase
  case (1'h0) (!1'sh1): begin : h9 initial $display("@Ra25_9 hit"); end default: begin : d9 initial $display("@Ra25_9 def"); end endcase
  case (1'sh0) (!1'sh1): begin : h10 initial $display("@Ra25_10 hit"); end default: begin : d10 initial $display("@Ra25_10 def"); end endcase
  case (4'h0) (!1'sh1): begin : h11 initial $display("@Ra25_11 hit"); end default: begin : d11 initial $display("@Ra25_11 def"); end endcase
  case (64'h0) (!1'sh1): begin : h12 initial $display("@Ra25_12 hit"); end default: begin : d12 initial $display("@Ra25_12 def"); end endcase
  case (0) (!1'sh1): begin : h13 initial $display("@Ra25_13 hit"); end default: begin : d13 initial $display("@Ra25_13 def"); end endcase
  case (0) (!1'sh1): begin : h14 initial $display("@Ra25_14 hit"); end default: begin : d14 initial $display("@Ra25_14 def"); end endcase
  case (1'h0) (((-1) ? S8 : UN) == S65): begin : h15 initial $display("@Ra25_15 hit"); end default: begin : d15 initial $display("@Ra25_15 def"); end endcase
  case (1'sh0) (((-1) ? S8 : UN) == S65): begin : h16 initial $display("@Ra25_16 hit"); end default: begin : d16 initial $display("@Ra25_16 def"); end endcase
  case (4'h0) (((-1) ? S8 : UN) == S65): begin : h17 initial $display("@Ra25_17 hit"); end default: begin : d17 initial $display("@Ra25_17 def"); end endcase
  case (64'h0) (((-1) ? S8 : UN) == S65): begin : h18 initial $display("@Ra25_18 hit"); end default: begin : d18 initial $display("@Ra25_18 def"); end endcase
  case (0) (((-1) ? S8 : UN) == S65): begin : h19 initial $display("@Ra25_19 hit"); end default: begin : d19 initial $display("@Ra25_19 def"); end endcase
  case (0) (((-1) ? S8 : UN) == S65): begin : h20 initial $display("@Ra25_20 hit"); end default: begin : d20 initial $display("@Ra25_20 def"); end endcase
  case (4'hc) P4: begin : h21 initial $display("@Ra25_21 hit"); end default: begin : d21 initial $display("@Ra25_21 def"); end endcase
  case (4'shc) P4: begin : h22 initial $display("@Ra25_22 hit"); end default: begin : d22 initial $display("@Ra25_22 def"); end endcase
  case (7'hc) P4: begin : h23 initial $display("@Ra25_23 hit"); end default: begin : d23 initial $display("@Ra25_23 def"); end endcase
  case (64'hfffffffffffffffc) P4: begin : h24 initial $display("@Ra25_24 hit"); end default: begin : d24 initial $display("@Ra25_24 def"); end endcase
endmodule
