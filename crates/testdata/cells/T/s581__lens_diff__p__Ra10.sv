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
  case (1446035121) ((64'hc3c9f7e3d8b4c831 * (-1)) / 32'hb2d643a2): begin : h0 initial $display("@Ra10_0 hit"); end default: begin : d0 initial $display("@Ra10_0 def"); end endcase
  case (1'h0) (!4'sh3): begin : h1 initial $display("@Ra10_1 hit"); end default: begin : d1 initial $display("@Ra10_1 def"); end endcase
  case (1'sh0) (!4'sh3): begin : h2 initial $display("@Ra10_2 hit"); end default: begin : d2 initial $display("@Ra10_2 def"); end endcase
  case (4'h0) (!4'sh3): begin : h3 initial $display("@Ra10_3 hit"); end default: begin : d3 initial $display("@Ra10_3 def"); end endcase
  case (64'h0) (!4'sh3): begin : h4 initial $display("@Ra10_4 hit"); end default: begin : d4 initial $display("@Ra10_4 def"); end endcase
  case (0) (!4'sh3): begin : h5 initial $display("@Ra10_5 hit"); end default: begin : d5 initial $display("@Ra10_5 def"); end endcase
  case (0) (!4'sh3): begin : h6 initial $display("@Ra10_6 hit"); end default: begin : d6 initial $display("@Ra10_6 def"); end endcase
  case (5'hb) (~5'h14): begin : h7 initial $display("@Ra10_7 hit"); end default: begin : d7 initial $display("@Ra10_7 def"); end endcase
  case (5'shb) (~5'h14): begin : h8 initial $display("@Ra10_8 hit"); end default: begin : d8 initial $display("@Ra10_8 def"); end endcase
  case (8'hb) (~5'h14): begin : h9 initial $display("@Ra10_9 hit"); end default: begin : d9 initial $display("@Ra10_9 def"); end endcase
  case (64'hb) (~5'h14): begin : h10 initial $display("@Ra10_10 hit"); end default: begin : d10 initial $display("@Ra10_10 def"); end endcase
  case (11) (~5'h14): begin : h11 initial $display("@Ra10_11 hit"); end default: begin : d11 initial $display("@Ra10_11 def"); end endcase
  case (11) (~5'h14): begin : h12 initial $display("@Ra10_12 hit"); end default: begin : d12 initial $display("@Ra10_12 def"); end endcase
  case (1'h1) ((P65 / S8) || 2'sh2): begin : h13 initial $display("@Ra10_13 hit"); end default: begin : d13 initial $display("@Ra10_13 def"); end endcase
  case (1'sh1) ((P65 / S8) || 2'sh2): begin : h14 initial $display("@Ra10_14 hit"); end default: begin : d14 initial $display("@Ra10_14 def"); end endcase
  case (4'h1) ((P65 / S8) || 2'sh2): begin : h15 initial $display("@Ra10_15 hit"); end default: begin : d15 initial $display("@Ra10_15 def"); end endcase
  case (64'hffffffffffffffff) ((P65 / S8) || 2'sh2): begin : h16 initial $display("@Ra10_16 hit"); end default: begin : d16 initial $display("@Ra10_16 def"); end endcase
  case ((-1)) ((P65 / S8) || 2'sh2): begin : h17 initial $display("@Ra10_17 hit"); end default: begin : d17 initial $display("@Ra10_17 def"); end endcase
  case (1) ((P65 / S8) || 2'sh2): begin : h18 initial $display("@Ra10_18 hit"); end default: begin : d18 initial $display("@Ra10_18 def"); end endcase
  case (32'h26) ((~18) * ((-4) >>> 1)): begin : h19 initial $display("@Ra10_19 hit"); end default: begin : d19 initial $display("@Ra10_19 def"); end endcase
  case (32'sh26) ((~18) * ((-4) >>> 1)): begin : h20 initial $display("@Ra10_20 hit"); end default: begin : d20 initial $display("@Ra10_20 def"); end endcase
  case (35'h26) ((~18) * ((-4) >>> 1)): begin : h21 initial $display("@Ra10_21 hit"); end default: begin : d21 initial $display("@Ra10_21 def"); end endcase
  case (64'h26) ((~18) * ((-4) >>> 1)): begin : h22 initial $display("@Ra10_22 hit"); end default: begin : d22 initial $display("@Ra10_22 def"); end endcase
  case (38) ((~18) * ((-4) >>> 1)): begin : h23 initial $display("@Ra10_23 hit"); end default: begin : d23 initial $display("@Ra10_23 def"); end endcase
  case (38) ((~18) * ((-4) >>> 1)): begin : h24 initial $display("@Ra10_24 hit"); end default: begin : d24 initial $display("@Ra10_24 def"); end endcase
endmodule
