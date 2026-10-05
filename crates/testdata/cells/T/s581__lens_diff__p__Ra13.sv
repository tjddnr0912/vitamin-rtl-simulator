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
  case (64'hffffffffffffffdd) {2{S4}}: begin : h0 initial $display("@Ra13_0 hit"); end default: begin : d0 initial $display("@Ra13_0 def"); end endcase
  case ((-35)) {2{S4}}: begin : h1 initial $display("@Ra13_1 hit"); end default: begin : d1 initial $display("@Ra13_1 def"); end endcase
  case (221) {2{S4}}: begin : h2 initial $display("@Ra13_2 hit"); end default: begin : d2 initial $display("@Ra13_2 def"); end endcase
  case (32'he) ((S8 >>> 3) + (U32 ? I : P4)): begin : h3 initial $display("@Ra13_3 hit"); end default: begin : d3 initial $display("@Ra13_3 def"); end endcase
  case (32'she) ((S8 >>> 3) + (U32 ? I : P4)): begin : h4 initial $display("@Ra13_4 hit"); end default: begin : d4 initial $display("@Ra13_4 def"); end endcase
  case (35'he) ((S8 >>> 3) + (U32 ? I : P4)): begin : h5 initial $display("@Ra13_5 hit"); end default: begin : d5 initial $display("@Ra13_5 def"); end endcase
  case (64'he) ((S8 >>> 3) + (U32 ? I : P4)): begin : h6 initial $display("@Ra13_6 hit"); end default: begin : d6 initial $display("@Ra13_6 def"); end endcase
  case (14) ((S8 >>> 3) + (U32 ? I : P4)): begin : h7 initial $display("@Ra13_7 hit"); end default: begin : d7 initial $display("@Ra13_7 def"); end endcase
  case (14) ((S8 >>> 3) + (U32 ? I : P4)): begin : h8 initial $display("@Ra13_8 hit"); end default: begin : d8 initial $display("@Ra13_8 def"); end endcase
  case (8'hbb) {4'hB, 4'hB}: begin : h9 initial $display("@Ra13_9 hit"); end default: begin : d9 initial $display("@Ra13_9 def"); end endcase
  case (8'shbb) {4'hB, 4'hB}: begin : h10 initial $display("@Ra13_10 hit"); end default: begin : d10 initial $display("@Ra13_10 def"); end endcase
  case (11'hbb) {4'hB, 4'hB}: begin : h11 initial $display("@Ra13_11 hit"); end default: begin : d11 initial $display("@Ra13_11 def"); end endcase
  case (64'hffffffffffffffbb) {4'hB, 4'hB}: begin : h12 initial $display("@Ra13_12 hit"); end default: begin : d12 initial $display("@Ra13_12 def"); end endcase
  case ((-69)) {4'hB, 4'hB}: begin : h13 initial $display("@Ra13_13 hit"); end default: begin : d13 initial $display("@Ra13_13 def"); end endcase
  case (187) {4'hB, 4'hB}: begin : h14 initial $display("@Ra13_14 hit"); end default: begin : d14 initial $display("@Ra13_14 def"); end endcase
  case (1'h0) (8'h4 == 16'h3423): begin : h15 initial $display("@Ra13_15 hit"); end default: begin : d15 initial $display("@Ra13_15 def"); end endcase
  case (1'sh0) (8'h4 == 16'h3423): begin : h16 initial $display("@Ra13_16 hit"); end default: begin : d16 initial $display("@Ra13_16 def"); end endcase
  case (4'h0) (8'h4 == 16'h3423): begin : h17 initial $display("@Ra13_17 hit"); end default: begin : d17 initial $display("@Ra13_17 def"); end endcase
  case (64'h0) (8'h4 == 16'h3423): begin : h18 initial $display("@Ra13_18 hit"); end default: begin : d18 initial $display("@Ra13_18 def"); end endcase
  case (0) (8'h4 == 16'h3423): begin : h19 initial $display("@Ra13_19 hit"); end default: begin : d19 initial $display("@Ra13_19 def"); end endcase
  case (0) (8'h4 == 16'h3423): begin : h20 initial $display("@Ra13_20 hit"); end default: begin : d20 initial $display("@Ra13_20 def"); end endcase
  case (4'hd) S4: begin : h21 initial $display("@Ra13_21 hit"); end default: begin : d21 initial $display("@Ra13_21 def"); end endcase
  case (4'shd) S4: begin : h22 initial $display("@Ra13_22 hit"); end default: begin : d22 initial $display("@Ra13_22 def"); end endcase
  case (7'hd) S4: begin : h23 initial $display("@Ra13_23 hit"); end default: begin : d23 initial $display("@Ra13_23 def"); end endcase
  case (64'hfffffffffffffffd) S4: begin : h24 initial $display("@Ra13_24 hit"); end default: begin : d24 initial $display("@Ra13_24 def"); end endcase
endmodule
