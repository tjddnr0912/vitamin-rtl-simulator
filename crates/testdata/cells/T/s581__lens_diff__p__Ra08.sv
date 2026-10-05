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
  case (63'h7868d5abe02f9a72) 63'h7868d5abe02f9a72: begin : h0 initial $display("@Ra8_0 hit"); end default: begin : d0 initial $display("@Ra8_0 def"); end endcase
  case (63'sh7868d5abe02f9a72) 63'h7868d5abe02f9a72: begin : h1 initial $display("@Ra8_1 hit"); end default: begin : d1 initial $display("@Ra8_1 def"); end endcase
  case (64'h7868d5abe02f9a72) 63'h7868d5abe02f9a72: begin : h2 initial $display("@Ra8_2 hit"); end default: begin : d2 initial $display("@Ra8_2 def"); end endcase
  case (64'hf868d5abe02f9a72) 63'h7868d5abe02f9a72: begin : h3 initial $display("@Ra8_3 hit"); end default: begin : d3 initial $display("@Ra8_3 def"); end endcase
  case (12'hbf0) {4'hB, P8}: begin : h4 initial $display("@Ra8_4 hit"); end default: begin : d4 initial $display("@Ra8_4 def"); end endcase
  case (12'shbf0) {4'hB, P8}: begin : h5 initial $display("@Ra8_5 hit"); end default: begin : d5 initial $display("@Ra8_5 def"); end endcase
  case (15'hbf0) {4'hB, P8}: begin : h6 initial $display("@Ra8_6 hit"); end default: begin : d6 initial $display("@Ra8_6 def"); end endcase
  case (64'hfffffffffffffbf0) {4'hB, P8}: begin : h7 initial $display("@Ra8_7 hit"); end default: begin : d7 initial $display("@Ra8_7 def"); end endcase
  case ((-1040)) {4'hB, P8}: begin : h8 initial $display("@Ra8_8 hit"); end default: begin : d8 initial $display("@Ra8_8 def"); end endcase
  case (3056) {4'hB, P8}: begin : h9 initial $display("@Ra8_9 hit"); end default: begin : d9 initial $display("@Ra8_9 def"); end endcase
  case (3'h0) (3'h2 >>> {65'(5'h15), S4}): begin : h10 initial $display("@Ra8_10 hit"); end default: begin : d10 initial $display("@Ra8_10 def"); end endcase
  case (3'sh0) (3'h2 >>> {65'(5'h15), S4}): begin : h11 initial $display("@Ra8_11 hit"); end default: begin : d11 initial $display("@Ra8_11 def"); end endcase
  case (6'h0) (3'h2 >>> {65'(5'h15), S4}): begin : h12 initial $display("@Ra8_12 hit"); end default: begin : d12 initial $display("@Ra8_12 def"); end endcase
  case (64'h0) (3'h2 >>> {65'(5'h15), S4}): begin : h13 initial $display("@Ra8_13 hit"); end default: begin : d13 initial $display("@Ra8_13 def"); end endcase
  case (0) (3'h2 >>> {65'(5'h15), S4}): begin : h14 initial $display("@Ra8_14 hit"); end default: begin : d14 initial $display("@Ra8_14 def"); end endcase
  case (0) (3'h2 >>> {65'(5'h15), S4}): begin : h15 initial $display("@Ra8_15 hit"); end default: begin : d15 initial $display("@Ra8_15 def"); end endcase
  case (64'hfffffffffffffff7) ((31'h7f8c7f19 ? L64 : S4) ^ (S8 == UN)): begin : h16 initial $display("@Ra8_16 hit"); end default: begin : d16 initial $display("@Ra8_16 def"); end endcase
  case (64'shfffffffffffffff7) ((31'h7f8c7f19 ? L64 : S4) ^ (S8 == UN)): begin : h17 initial $display("@Ra8_17 hit"); end default: begin : d17 initial $display("@Ra8_17 def"); end endcase
  case (64'hfffffffffffffff7) ((31'h7f8c7f19 ? L64 : S4) ^ (S8 == UN)): begin : h18 initial $display("@Ra8_18 hit"); end default: begin : d18 initial $display("@Ra8_18 def"); end endcase
  case (64'hfffffffffffffff7) ((31'h7f8c7f19 ? L64 : S4) ^ (S8 == UN)): begin : h19 initial $display("@Ra8_19 hit"); end default: begin : d19 initial $display("@Ra8_19 def"); end endcase
  case ((-9)) ((31'h7f8c7f19 ? L64 : S4) ^ (S8 == UN)): begin : h20 initial $display("@Ra8_20 hit"); end default: begin : d20 initial $display("@Ra8_20 def"); end endcase
  case (16'hbeef) 16'shbeef: begin : h21 initial $display("@Ra8_21 hit"); end default: begin : d21 initial $display("@Ra8_21 def"); end endcase
  case (16'shbeef) 16'shbeef: begin : h22 initial $display("@Ra8_22 hit"); end default: begin : d22 initial $display("@Ra8_22 def"); end endcase
  case (19'hbeef) 16'shbeef: begin : h23 initial $display("@Ra8_23 hit"); end default: begin : d23 initial $display("@Ra8_23 def"); end endcase
  case (64'hffffffffffffbeef) 16'shbeef: begin : h24 initial $display("@Ra8_24 hit"); end default: begin : d24 initial $display("@Ra8_24 def"); end endcase
endmodule
