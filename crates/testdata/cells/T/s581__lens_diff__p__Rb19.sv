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
  case (133561409) (4'sh2 ? $unsigned(33'h7f5fc41) : {2{S4}}): begin : h0 initial $display("@Rb19_0 hit"); end default: begin : d0 initial $display("@Rb19_0 def"); end endcase
  case (133561409) (4'sh2 ? $unsigned(33'h7f5fc41) : {2{S4}}): begin : h1 initial $display("@Rb19_1 hit"); end default: begin : d1 initial $display("@Rb19_1 def"); end endcase
  case (63'h2ca06788d1a72632) (63'sh2ca06788d1a72632 << ((4'sh8 * P8) == {P4, 3'(33'h583f4f95)})): begin : h2 initial $display("@Rb19_2 hit"); end default: begin : d2 initial $display("@Rb19_2 def"); end endcase
  case (63'sh2ca06788d1a72632) (63'sh2ca06788d1a72632 << ((4'sh8 * P8) == {P4, 3'(33'h583f4f95)})): begin : h3 initial $display("@Rb19_3 hit"); end default: begin : d3 initial $display("@Rb19_3 def"); end endcase
  case (64'h2ca06788d1a72632) (63'sh2ca06788d1a72632 << ((4'sh8 * P8) == {P4, 3'(33'h583f4f95)})): begin : h4 initial $display("@Rb19_4 hit"); end default: begin : d4 initial $display("@Rb19_4 def"); end endcase
  case (64'h2ca06788d1a72632) (63'sh2ca06788d1a72632 << ((4'sh8 * P8) == {P4, 3'(33'h583f4f95)})): begin : h5 initial $display("@Rb19_5 hit"); end default: begin : d5 initial $display("@Rb19_5 def"); end endcase
  case (31'h672d8256) $unsigned(31'sh672d8256): begin : h6 initial $display("@Rb19_6 hit"); end default: begin : d6 initial $display("@Rb19_6 def"); end endcase
  case (31'sh672d8256) $unsigned(31'sh672d8256): begin : h7 initial $display("@Rb19_7 hit"); end default: begin : d7 initial $display("@Rb19_7 def"); end endcase
  case (34'h672d8256) $unsigned(31'sh672d8256): begin : h8 initial $display("@Rb19_8 hit"); end default: begin : d8 initial $display("@Rb19_8 def"); end endcase
  case (64'hffffffffe72d8256) $unsigned(31'sh672d8256): begin : h9 initial $display("@Rb19_9 hit"); end default: begin : d9 initial $display("@Rb19_9 def"); end endcase
  case ((-416447914)) $unsigned(31'sh672d8256): begin : h10 initial $display("@Rb19_10 hit"); end default: begin : d10 initial $display("@Rb19_10 def"); end endcase
  case (1731035734) $unsigned(31'sh672d8256): begin : h11 initial $display("@Rb19_11 hit"); end default: begin : d11 initial $display("@Rb19_11 def"); end endcase
  case (1'h1) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h12 initial $display("@Rb19_12 hit"); end default: begin : d12 initial $display("@Rb19_12 def"); end endcase
  case (1'sh1) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h13 initial $display("@Rb19_13 hit"); end default: begin : d13 initial $display("@Rb19_13 def"); end endcase
  case (4'h1) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h14 initial $display("@Rb19_14 hit"); end default: begin : d14 initial $display("@Rb19_14 def"); end endcase
  case (64'hffffffffffffffff) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h15 initial $display("@Rb19_15 hit"); end default: begin : d15 initial $display("@Rb19_15 def"); end endcase
  case ((-1)) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h16 initial $display("@Rb19_16 hit"); end default: begin : d16 initial $display("@Rb19_16 def"); end endcase
  case (1) (((I << 4) - (S65 + (-8))) ? ((16'hed26 == 4'hf) < (64'sh1dfa83c9de7fc569 && UN)) : (!63'h5abd27678d5b52cc)): begin : h17 initial $display("@Rb19_17 hit"); end default: begin : d17 initial $display("@Rb19_17 def"); end endcase
  case (1'h0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h18 initial $display("@Rb19_18 hit"); end default: begin : d18 initial $display("@Rb19_18 def"); end endcase
  case (1'sh0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h19 initial $display("@Rb19_19 hit"); end default: begin : d19 initial $display("@Rb19_19 def"); end endcase
  case (4'h0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h20 initial $display("@Rb19_20 hit"); end default: begin : d20 initial $display("@Rb19_20 def"); end endcase
  case (64'h0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h21 initial $display("@Rb19_21 hit"); end default: begin : d21 initial $display("@Rb19_21 def"); end endcase
  case (0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h22 initial $display("@Rb19_22 hit"); end default: begin : d22 initial $display("@Rb19_22 def"); end endcase
  case (0) (!((!8'hef) + 64'shfae98ce83f068f79)): begin : h23 initial $display("@Rb19_23 hit"); end default: begin : d23 initial $display("@Rb19_23 def"); end endcase
  case (4'ha) UN: begin : h24 initial $display("@Rb19_24 hit"); end default: begin : d24 initial $display("@Rb19_24 def"); end endcase
endmodule
