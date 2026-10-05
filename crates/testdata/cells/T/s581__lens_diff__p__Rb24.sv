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
  case (64'h3cc85af3) 31'sh3cc85af3: begin : h0 initial $display("@Rb24_0 hit"); end default: begin : d0 initial $display("@Rb24_0 def"); end endcase
  case (1019763443) 31'sh3cc85af3: begin : h1 initial $display("@Rb24_1 hit"); end default: begin : d1 initial $display("@Rb24_1 def"); end endcase
  case (1019763443) 31'sh3cc85af3: begin : h2 initial $display("@Rb24_2 hit"); end default: begin : d2 initial $display("@Rb24_2 def"); end endcase
  case (1'h0) $signed($signed((1'sh1 && 1'h0))): begin : h3 initial $display("@Rb24_3 hit"); end default: begin : d3 initial $display("@Rb24_3 def"); end endcase
  case (1'sh0) $signed($signed((1'sh1 && 1'h0))): begin : h4 initial $display("@Rb24_4 hit"); end default: begin : d4 initial $display("@Rb24_4 def"); end endcase
  case (4'h0) $signed($signed((1'sh1 && 1'h0))): begin : h5 initial $display("@Rb24_5 hit"); end default: begin : d5 initial $display("@Rb24_5 def"); end endcase
  case (64'h0) $signed($signed((1'sh1 && 1'h0))): begin : h6 initial $display("@Rb24_6 hit"); end default: begin : d6 initial $display("@Rb24_6 def"); end endcase
  case (0) $signed($signed((1'sh1 && 1'h0))): begin : h7 initial $display("@Rb24_7 hit"); end default: begin : d7 initial $display("@Rb24_7 def"); end endcase
  case (0) $signed($signed((1'sh1 && 1'h0))): begin : h8 initial $display("@Rb24_8 hit"); end default: begin : d8 initial $display("@Rb24_8 def"); end endcase
  case (4'h0) ((11 && 3'h1) / (~P4)): begin : h9 initial $display("@Rb24_9 hit"); end default: begin : d9 initial $display("@Rb24_9 def"); end endcase
  case (4'sh0) ((11 && 3'h1) / (~P4)): begin : h10 initial $display("@Rb24_10 hit"); end default: begin : d10 initial $display("@Rb24_10 def"); end endcase
  case (7'h0) ((11 && 3'h1) / (~P4)): begin : h11 initial $display("@Rb24_11 hit"); end default: begin : d11 initial $display("@Rb24_11 def"); end endcase
  case (64'h0) ((11 && 3'h1) / (~P4)): begin : h12 initial $display("@Rb24_12 hit"); end default: begin : d12 initial $display("@Rb24_12 def"); end endcase
  case (0) ((11 && 3'h1) / (~P4)): begin : h13 initial $display("@Rb24_13 hit"); end default: begin : d13 initial $display("@Rb24_13 def"); end endcase
  case (0) ((11 && 3'h1) / (~P4)): begin : h14 initial $display("@Rb24_14 hit"); end default: begin : d14 initial $display("@Rb24_14 def"); end endcase
  case (37'h1600000001) {4'hB, 33'((S4 && UN))}: begin : h15 initial $display("@Rb24_15 hit"); end default: begin : d15 initial $display("@Rb24_15 def"); end endcase
  case (37'sh1600000001) {4'hB, 33'((S4 && UN))}: begin : h16 initial $display("@Rb24_16 hit"); end default: begin : d16 initial $display("@Rb24_16 def"); end endcase
  case (40'h1600000001) {4'hB, 33'((S4 && UN))}: begin : h17 initial $display("@Rb24_17 hit"); end default: begin : d17 initial $display("@Rb24_17 def"); end endcase
  case (64'hfffffff600000001) {4'hB, 33'((S4 && UN))}: begin : h18 initial $display("@Rb24_18 hit"); end default: begin : d18 initial $display("@Rb24_18 def"); end endcase
  case (32'h5d) (~(S8 - (-6))): begin : h19 initial $display("@Rb24_19 hit"); end default: begin : d19 initial $display("@Rb24_19 def"); end endcase
  case (32'sh5d) (~(S8 - (-6))): begin : h20 initial $display("@Rb24_20 hit"); end default: begin : d20 initial $display("@Rb24_20 def"); end endcase
  case (35'h5d) (~(S8 - (-6))): begin : h21 initial $display("@Rb24_21 hit"); end default: begin : d21 initial $display("@Rb24_21 def"); end endcase
  case (64'h5d) (~(S8 - (-6))): begin : h22 initial $display("@Rb24_22 hit"); end default: begin : d22 initial $display("@Rb24_22 def"); end endcase
  case (93) (~(S8 - (-6))): begin : h23 initial $display("@Rb24_23 hit"); end default: begin : d23 initial $display("@Rb24_23 def"); end endcase
  case (93) (~(S8 - (-6))): begin : h24 initial $display("@Rb24_24 hit"); end default: begin : d24 initial $display("@Rb24_24 def"); end endcase
endmodule
