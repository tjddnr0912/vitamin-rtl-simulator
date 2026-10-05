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
  case (64'h1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h0 initial $display("@Ra29_0 hit"); end default: begin : d0 initial $display("@Ra29_0 def"); end endcase
  case (1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h1 initial $display("@Ra29_1 hit"); end default: begin : d1 initial $display("@Ra29_1 def"); end endcase
  case (1) ((-(2'sh3 >>> 3)) | (((-2) / 33'h138bbd462) == (21 >> 4))): begin : h2 initial $display("@Ra29_2 hit"); end default: begin : d2 initial $display("@Ra29_2 def"); end endcase
  case (1'h0) (~(U32 || (35 * 8'hf))): begin : h3 initial $display("@Ra29_3 hit"); end default: begin : d3 initial $display("@Ra29_3 def"); end endcase
  case (1'sh0) (~(U32 || (35 * 8'hf))): begin : h4 initial $display("@Ra29_4 hit"); end default: begin : d4 initial $display("@Ra29_4 def"); end endcase
  case (4'h0) (~(U32 || (35 * 8'hf))): begin : h5 initial $display("@Ra29_5 hit"); end default: begin : d5 initial $display("@Ra29_5 def"); end endcase
  case (64'h0) (~(U32 || (35 * 8'hf))): begin : h6 initial $display("@Ra29_6 hit"); end default: begin : d6 initial $display("@Ra29_6 def"); end endcase
  case (0) (~(U32 || (35 * 8'hf))): begin : h7 initial $display("@Ra29_7 hit"); end default: begin : d7 initial $display("@Ra29_7 def"); end endcase
  case (0) (~(U32 || (35 * 8'hf))): begin : h8 initial $display("@Ra29_8 hit"); end default: begin : d8 initial $display("@Ra29_8 def"); end endcase
endmodule
