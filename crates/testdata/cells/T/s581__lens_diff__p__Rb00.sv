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
  case (31'he3c71e0) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h0 initial $display("@Rb0_0 hit"); end default: begin : d0 initial $display("@Rb0_0 def"); end endcase
  case (31'she3c71e0) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h1 initial $display("@Rb0_1 hit"); end default: begin : d1 initial $display("@Rb0_1 def"); end endcase
  case (34'he3c71e0) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h2 initial $display("@Rb0_2 hit"); end default: begin : d2 initial $display("@Rb0_2 def"); end endcase
  case (64'he3c71e0) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h3 initial $display("@Rb0_3 hit"); end default: begin : d3 initial $display("@Rb0_3 def"); end endcase
  case (238842336) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h4 initial $display("@Rb0_4 hit"); end default: begin : d4 initial $display("@Rb0_4 def"); end endcase
  case (238842336) ($signed((5'h10 * 31'sh5c71e38f)) << 1): begin : h5 initial $display("@Rb0_5 hit"); end default: begin : d5 initial $display("@Rb0_5 def"); end endcase
  case (34'ha1fc6502) {33'(65'sh6712489050fe3281), 1'((L64 << $signed(S8)))}: begin : h6 initial $display("@Rb0_6 hit"); end default: begin : d6 initial $display("@Rb0_6 def"); end endcase
  case (34'sha1fc6502) {33'(65'sh6712489050fe3281), 1'((L64 << $signed(S8)))}: begin : h7 initial $display("@Rb0_7 hit"); end default: begin : d7 initial $display("@Rb0_7 def"); end endcase
  case (37'ha1fc6502) {33'(65'sh6712489050fe3281), 1'((L64 << $signed(S8)))}: begin : h8 initial $display("@Rb0_8 hit"); end default: begin : d8 initial $display("@Rb0_8 def"); end endcase
  case (64'ha1fc6502) {33'(65'sh6712489050fe3281), 1'((L64 << $signed(S8)))}: begin : h9 initial $display("@Rb0_9 hit"); end default: begin : d9 initial $display("@Rb0_9 def"); end endcase
  case (12'hf0c) {P8, P4}: begin : h10 initial $display("@Rb0_10 hit"); end default: begin : d10 initial $display("@Rb0_10 def"); end endcase
  case (12'shf0c) {P8, P4}: begin : h11 initial $display("@Rb0_11 hit"); end default: begin : d11 initial $display("@Rb0_11 def"); end endcase
  case (15'hf0c) {P8, P4}: begin : h12 initial $display("@Rb0_12 hit"); end default: begin : d12 initial $display("@Rb0_12 def"); end endcase
  case (64'hffffffffffffff0c) {P8, P4}: begin : h13 initial $display("@Rb0_13 hit"); end default: begin : d13 initial $display("@Rb0_13 def"); end endcase
  case ((-244)) {P8, P4}: begin : h14 initial $display("@Rb0_14 hit"); end default: begin : d14 initial $display("@Rb0_14 def"); end endcase
  case (3852) {P8, P4}: begin : h15 initial $display("@Rb0_15 hit"); end default: begin : d15 initial $display("@Rb0_15 def"); end endcase
  case (16'h9317) (~16'sh6ce8): begin : h16 initial $display("@Rb0_16 hit"); end default: begin : d16 initial $display("@Rb0_16 def"); end endcase
  case (16'sh9317) (~16'sh6ce8): begin : h17 initial $display("@Rb0_17 hit"); end default: begin : d17 initial $display("@Rb0_17 def"); end endcase
  case (19'h9317) (~16'sh6ce8): begin : h18 initial $display("@Rb0_18 hit"); end default: begin : d18 initial $display("@Rb0_18 def"); end endcase
  case (64'hffffffffffff9317) (~16'sh6ce8): begin : h19 initial $display("@Rb0_19 hit"); end default: begin : d19 initial $display("@Rb0_19 def"); end endcase
  case ((-27881)) (~16'sh6ce8): begin : h20 initial $display("@Rb0_20 hit"); end default: begin : d20 initial $display("@Rb0_20 def"); end endcase
  case (37655) (~16'sh6ce8): begin : h21 initial $display("@Rb0_21 hit"); end default: begin : d21 initial $display("@Rb0_21 def"); end endcase
  case (1'h1) 1'sh1: begin : h22 initial $display("@Rb0_22 hit"); end default: begin : d22 initial $display("@Rb0_22 def"); end endcase
  case (1'sh1) 1'sh1: begin : h23 initial $display("@Rb0_23 hit"); end default: begin : d23 initial $display("@Rb0_23 def"); end endcase
  case (4'h1) 1'sh1: begin : h24 initial $display("@Rb0_24 hit"); end default: begin : d24 initial $display("@Rb0_24 def"); end endcase
endmodule
