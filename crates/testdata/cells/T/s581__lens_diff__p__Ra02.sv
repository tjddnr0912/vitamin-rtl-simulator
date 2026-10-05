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
  case (4) 3'h4: begin : h0 initial $display("@Ra2_0 hit"); end default: begin : d0 initial $display("@Ra2_0 def"); end endcase
  case (33'h0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h1 initial $display("@Ra2_1 hit"); end default: begin : d1 initial $display("@Ra2_1 def"); end endcase
  case (33'sh0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h2 initial $display("@Ra2_2 hit"); end default: begin : d2 initial $display("@Ra2_2 def"); end endcase
  case (36'h0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h3 initial $display("@Ra2_3 hit"); end default: begin : d3 initial $display("@Ra2_3 def"); end endcase
  case (64'h0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h4 initial $display("@Ra2_4 hit"); end default: begin : d4 initial $display("@Ra2_4 def"); end endcase
  case (0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h5 initial $display("@Ra2_5 hit"); end default: begin : d5 initial $display("@Ra2_5 def"); end endcase
  case (0) ((P65 ? (-3) : S8) ? (!I) : (~33'hf3fe39c0)): begin : h6 initial $display("@Ra2_6 hit"); end default: begin : d6 initial $display("@Ra2_6 def"); end endcase
  case (63'h1121) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h7 initial $display("@Ra2_7 hit"); end default: begin : d7 initial $display("@Ra2_7 def"); end endcase
  case (63'sh1121) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h8 initial $display("@Ra2_8 hit"); end default: begin : d8 initial $display("@Ra2_8 def"); end endcase
  case (64'h1121) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h9 initial $display("@Ra2_9 hit"); end default: begin : d9 initial $display("@Ra2_9 def"); end endcase
  case (64'h1121) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h10 initial $display("@Ra2_10 hit"); end default: begin : d10 initial $display("@Ra2_10 def"); end endcase
  case (4385) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h11 initial $display("@Ra2_11 hit"); end default: begin : d11 initial $display("@Ra2_11 def"); end endcase
  case (4385) (63'h32f3f2116472f1a3 & 16'sh113d): begin : h12 initial $display("@Ra2_12 hit"); end default: begin : d12 initial $display("@Ra2_12 def"); end endcase
  case (1'h0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h13 initial $display("@Ra2_13 hit"); end default: begin : d13 initial $display("@Ra2_13 def"); end endcase
  case (1'sh0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h14 initial $display("@Ra2_14 hit"); end default: begin : d14 initial $display("@Ra2_14 def"); end endcase
  case (4'h0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h15 initial $display("@Ra2_15 hit"); end default: begin : d15 initial $display("@Ra2_15 def"); end endcase
  case (64'h0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h16 initial $display("@Ra2_16 hit"); end default: begin : d16 initial $display("@Ra2_16 def"); end endcase
  case (0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h17 initial $display("@Ra2_17 hit"); end default: begin : d17 initial $display("@Ra2_17 def"); end endcase
  case (0) (((-32'sh5d158a2f) * (3'h7 ? S4 : S65)) && ((U32 >>> 4) == (1'h0 * (-4)))): begin : h18 initial $display("@Ra2_18 hit"); end default: begin : d18 initial $display("@Ra2_18 def"); end endcase
  case (8'h0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h19 initial $display("@Ra2_19 hit"); end default: begin : d19 initial $display("@Ra2_19 def"); end endcase
  case (8'sh0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h20 initial $display("@Ra2_20 hit"); end default: begin : d20 initial $display("@Ra2_20 def"); end endcase
  case (11'h0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h21 initial $display("@Ra2_21 hit"); end default: begin : d21 initial $display("@Ra2_21 def"); end endcase
  case (64'h0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h22 initial $display("@Ra2_22 hit"); end default: begin : d22 initial $display("@Ra2_22 def"); end endcase
  case (0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h23 initial $display("@Ra2_23 hit"); end default: begin : d23 initial $display("@Ra2_23 def"); end endcase
  case (0) $signed((64'h8483f8b8332dd331 ? 1'h0 : 8'hef)): begin : h24 initial $display("@Ra2_24 hit"); end default: begin : d24 initial $display("@Ra2_24 def"); end endcase
endmodule
