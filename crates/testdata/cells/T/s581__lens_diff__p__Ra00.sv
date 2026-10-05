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
  case (4'hc) P4: begin : h0 initial $display("@Ra0_0 hit"); end default: begin : d0 initial $display("@Ra0_0 def"); end endcase
  case (4'shc) P4: begin : h1 initial $display("@Ra0_1 hit"); end default: begin : d1 initial $display("@Ra0_1 def"); end endcase
  case (7'hc) P4: begin : h2 initial $display("@Ra0_2 hit"); end default: begin : d2 initial $display("@Ra0_2 def"); end endcase
  case (64'hfffffffffffffffc) P4: begin : h3 initial $display("@Ra0_3 hit"); end default: begin : d3 initial $display("@Ra0_3 def"); end endcase
  case ((-4)) P4: begin : h4 initial $display("@Ra0_4 hit"); end default: begin : d4 initial $display("@Ra0_4 def"); end endcase
  case (12) P4: begin : h5 initial $display("@Ra0_5 hit"); end default: begin : d5 initial $display("@Ra0_5 def"); end endcase
  case (64'hfffffffffffffff7) (UN ? L64 : S4): begin : h6 initial $display("@Ra0_6 hit"); end default: begin : d6 initial $display("@Ra0_6 def"); end endcase
  case (64'shfffffffffffffff7) (UN ? L64 : S4): begin : h7 initial $display("@Ra0_7 hit"); end default: begin : d7 initial $display("@Ra0_7 def"); end endcase
  case (64'hfffffffffffffff7) (UN ? L64 : S4): begin : h8 initial $display("@Ra0_8 hit"); end default: begin : d8 initial $display("@Ra0_8 def"); end endcase
  case (64'hfffffffffffffff7) (UN ? L64 : S4): begin : h9 initial $display("@Ra0_9 hit"); end default: begin : d9 initial $display("@Ra0_9 def"); end endcase
  case ((-9)) (UN ? L64 : S4): begin : h10 initial $display("@Ra0_10 hit"); end default: begin : d10 initial $display("@Ra0_10 def"); end endcase
  case (63'h7fffffffffffff66) (($signed(2'h2) - $unsigned(S8)) - $signed((S4 / 63'sh491d39b494e3bf91))): begin : h11 initial $display("@Ra0_11 hit"); end default: begin : d11 initial $display("@Ra0_11 def"); end endcase
  case (63'sh7fffffffffffff66) (($signed(2'h2) - $unsigned(S8)) - $signed((S4 / 63'sh491d39b494e3bf91))): begin : h12 initial $display("@Ra0_12 hit"); end default: begin : d12 initial $display("@Ra0_12 def"); end endcase
  case (64'h7fffffffffffff66) (($signed(2'h2) - $unsigned(S8)) - $signed((S4 / 63'sh491d39b494e3bf91))): begin : h13 initial $display("@Ra0_13 hit"); end default: begin : d13 initial $display("@Ra0_13 def"); end endcase
  case (64'hffffffffffffff66) (($signed(2'h2) - $unsigned(S8)) - $signed((S4 / 63'sh491d39b494e3bf91))): begin : h14 initial $display("@Ra0_14 hit"); end default: begin : d14 initial $display("@Ra0_14 def"); end endcase
  case ((-154)) (($signed(2'h2) - $unsigned(S8)) - $signed((S4 / 63'sh491d39b494e3bf91))): begin : h15 initial $display("@Ra0_15 hit"); end default: begin : d15 initial $display("@Ra0_15 def"); end endcase
  case (1'h0) (!(-31'h36bb583f)): begin : h16 initial $display("@Ra0_16 hit"); end default: begin : d16 initial $display("@Ra0_16 def"); end endcase
  case (1'sh0) (!(-31'h36bb583f)): begin : h17 initial $display("@Ra0_17 hit"); end default: begin : d17 initial $display("@Ra0_17 def"); end endcase
  case (4'h0) (!(-31'h36bb583f)): begin : h18 initial $display("@Ra0_18 hit"); end default: begin : d18 initial $display("@Ra0_18 def"); end endcase
  case (64'h0) (!(-31'h36bb583f)): begin : h19 initial $display("@Ra0_19 hit"); end default: begin : d19 initial $display("@Ra0_19 def"); end endcase
  case (0) (!(-31'h36bb583f)): begin : h20 initial $display("@Ra0_20 hit"); end default: begin : d20 initial $display("@Ra0_20 def"); end endcase
  case (0) (!(-31'h36bb583f)): begin : h21 initial $display("@Ra0_21 hit"); end default: begin : d21 initial $display("@Ra0_21 def"); end endcase
  case (8'hbb) {2{4'hB}}: begin : h22 initial $display("@Ra0_22 hit"); end default: begin : d22 initial $display("@Ra0_22 def"); end endcase
  case (8'shbb) {2{4'hB}}: begin : h23 initial $display("@Ra0_23 hit"); end default: begin : d23 initial $display("@Ra0_23 def"); end endcase
  case (11'hbb) {2{4'hB}}: begin : h24 initial $display("@Ra0_24 hit"); end default: begin : d24 initial $display("@Ra0_24 def"); end endcase
endmodule
