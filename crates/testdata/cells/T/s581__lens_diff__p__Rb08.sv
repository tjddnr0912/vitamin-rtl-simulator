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
  case (0) ((I || 20) >> (63'h5bf3229f6cb63ce1 ? U32 : 64'h88adb3dccc7e03bb)): begin : h0 initial $display("@Rb8_0 hit"); end default: begin : d0 initial $display("@Rb8_0 def"); end endcase
  case (32'h5) (~(-6)): begin : h1 initial $display("@Rb8_1 hit"); end default: begin : d1 initial $display("@Rb8_1 def"); end endcase
  case (32'sh5) (~(-6)): begin : h2 initial $display("@Rb8_2 hit"); end default: begin : d2 initial $display("@Rb8_2 def"); end endcase
  case (35'h5) (~(-6)): begin : h3 initial $display("@Rb8_3 hit"); end default: begin : d3 initial $display("@Rb8_3 def"); end endcase
  case (64'h5) (~(-6)): begin : h4 initial $display("@Rb8_4 hit"); end default: begin : d4 initial $display("@Rb8_4 def"); end endcase
  case (5) (~(-6)): begin : h5 initial $display("@Rb8_5 hit"); end default: begin : d5 initial $display("@Rb8_5 def"); end endcase
  case (5) (~(-6)): begin : h6 initial $display("@Rb8_6 hit"); end default: begin : d6 initial $display("@Rb8_6 def"); end endcase
  case (5'h11) {4'(40), 1'(L64)}: begin : h7 initial $display("@Rb8_7 hit"); end default: begin : d7 initial $display("@Rb8_7 def"); end endcase
  case (5'sh11) {4'(40), 1'(L64)}: begin : h8 initial $display("@Rb8_8 hit"); end default: begin : d8 initial $display("@Rb8_8 def"); end endcase
  case (8'h11) {4'(40), 1'(L64)}: begin : h9 initial $display("@Rb8_9 hit"); end default: begin : d9 initial $display("@Rb8_9 def"); end endcase
  case (64'hfffffffffffffff1) {4'(40), 1'(L64)}: begin : h10 initial $display("@Rb8_10 hit"); end default: begin : d10 initial $display("@Rb8_10 def"); end endcase
  case ((-15)) {4'(40), 1'(L64)}: begin : h11 initial $display("@Rb8_11 hit"); end default: begin : d11 initial $display("@Rb8_11 def"); end endcase
  case (17) {4'(40), 1'(L64)}: begin : h12 initial $display("@Rb8_12 hit"); end default: begin : d12 initial $display("@Rb8_12 def"); end endcase
  case (1'h0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h13 initial $display("@Rb8_13 hit"); end default: begin : d13 initial $display("@Rb8_13 def"); end endcase
  case (1'sh0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h14 initial $display("@Rb8_14 hit"); end default: begin : d14 initial $display("@Rb8_14 def"); end endcase
  case (4'h0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h15 initial $display("@Rb8_15 hit"); end default: begin : d15 initial $display("@Rb8_15 def"); end endcase
  case (64'h0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h16 initial $display("@Rb8_16 hit"); end default: begin : d16 initial $display("@Rb8_16 def"); end endcase
  case (0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h17 initial $display("@Rb8_17 hit"); end default: begin : d17 initial $display("@Rb8_17 def"); end endcase
  case (0) ($unsigned((0 == UN)) >> $unsigned(P8)): begin : h18 initial $display("@Rb8_18 hit"); end default: begin : d18 initial $display("@Rb8_18 def"); end endcase
  case (6'h39) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h19 initial $display("@Rb8_19 hit"); end default: begin : d19 initial $display("@Rb8_19 def"); end endcase
  case (6'sh39) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h20 initial $display("@Rb8_20 hit"); end default: begin : d20 initial $display("@Rb8_20 def"); end endcase
  case (9'h39) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h21 initial $display("@Rb8_21 hit"); end default: begin : d21 initial $display("@Rb8_21 def"); end endcase
  case (64'hfffffffffffffff9) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h22 initial $display("@Rb8_22 hit"); end default: begin : d22 initial $display("@Rb8_22 def"); end endcase
  case ((-7)) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h23 initial $display("@Rb8_23 hit"); end default: begin : d23 initial $display("@Rb8_23 def"); end endcase
  case (57) {3'({33'h1_0000_0001, 4'(L64)}), 3'((U32 + 4'h1))}: begin : h24 initial $display("@Rb8_24 hit"); end default: begin : d24 initial $display("@Rb8_24 def"); end endcase
endmodule
