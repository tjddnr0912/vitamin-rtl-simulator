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
  case (64'hffffffffffffffbb) {2{4'hB}}: begin : h0 initial $display("@Ra1_0 hit"); end default: begin : d0 initial $display("@Ra1_0 def"); end endcase
  case ((-69)) {2{4'hB}}: begin : h1 initial $display("@Ra1_1 hit"); end default: begin : d1 initial $display("@Ra1_1 def"); end endcase
  case (187) {2{4'hB}}: begin : h2 initial $display("@Ra1_2 hit"); end default: begin : d2 initial $display("@Ra1_2 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h3 initial $display("@Ra1_3 hit"); end default: begin : d3 initial $display("@Ra1_3 def"); end endcase
  case (64'shfffffffffffffff7) L64: begin : h4 initial $display("@Ra1_4 hit"); end default: begin : d4 initial $display("@Ra1_4 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h5 initial $display("@Ra1_5 hit"); end default: begin : d5 initial $display("@Ra1_5 def"); end endcase
  case (64'hfffffffffffffff7) L64: begin : h6 initial $display("@Ra1_6 hit"); end default: begin : d6 initial $display("@Ra1_6 def"); end endcase
  case ((-9)) L64: begin : h7 initial $display("@Ra1_7 hit"); end default: begin : d7 initial $display("@Ra1_7 def"); end endcase
  case (1'h1) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h8 initial $display("@Ra1_8 hit"); end default: begin : d8 initial $display("@Ra1_8 def"); end endcase
  case (1'sh1) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h9 initial $display("@Ra1_9 hit"); end default: begin : d9 initial $display("@Ra1_9 def"); end endcase
  case (4'h1) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h10 initial $display("@Ra1_10 hit"); end default: begin : d10 initial $display("@Ra1_10 def"); end endcase
  case (64'hffffffffffffffff) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h11 initial $display("@Ra1_11 hit"); end default: begin : d11 initial $display("@Ra1_11 def"); end endcase
  case ((-1)) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h12 initial $display("@Ra1_12 hit"); end default: begin : d12 initial $display("@Ra1_12 def"); end endcase
  case (1) ((65'h117f5e837d70820fe * I) && (U32 + 8'sh1d)): begin : h13 initial $display("@Ra1_13 hit"); end default: begin : d13 initial $display("@Ra1_13 def"); end endcase
  case (8'hbb) {2{4'hB}}: begin : h14 initial $display("@Ra1_14 hit"); end default: begin : d14 initial $display("@Ra1_14 def"); end endcase
  case (8'shbb) {2{4'hB}}: begin : h15 initial $display("@Ra1_15 hit"); end default: begin : d15 initial $display("@Ra1_15 def"); end endcase
  case (11'hbb) {2{4'hB}}: begin : h16 initial $display("@Ra1_16 hit"); end default: begin : d16 initial $display("@Ra1_16 def"); end endcase
  case (64'hffffffffffffffbb) {2{4'hB}}: begin : h17 initial $display("@Ra1_17 hit"); end default: begin : d17 initial $display("@Ra1_17 def"); end endcase
  case ((-69)) {2{4'hB}}: begin : h18 initial $display("@Ra1_18 hit"); end default: begin : d18 initial $display("@Ra1_18 def"); end endcase
  case (187) {2{4'hB}}: begin : h19 initial $display("@Ra1_19 hit"); end default: begin : d19 initial $display("@Ra1_19 def"); end endcase
  case (3'h4) 3'h4: begin : h20 initial $display("@Ra1_20 hit"); end default: begin : d20 initial $display("@Ra1_20 def"); end endcase
  case (3'sh4) 3'h4: begin : h21 initial $display("@Ra1_21 hit"); end default: begin : d21 initial $display("@Ra1_21 def"); end endcase
  case (6'h4) 3'h4: begin : h22 initial $display("@Ra1_22 hit"); end default: begin : d22 initial $display("@Ra1_22 def"); end endcase
  case (64'hfffffffffffffffc) 3'h4: begin : h23 initial $display("@Ra1_23 hit"); end default: begin : d23 initial $display("@Ra1_23 def"); end endcase
  case ((-4)) 3'h4: begin : h24 initial $display("@Ra1_24 hit"); end default: begin : d24 initial $display("@Ra1_24 def"); end endcase
endmodule
