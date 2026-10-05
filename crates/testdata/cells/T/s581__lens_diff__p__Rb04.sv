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
  case (7'hc) {1'((P8 + 5'hd)), 3'((S8 >>> 0))}: begin : h0 initial $display("@Rb4_0 hit"); end default: begin : d0 initial $display("@Rb4_0 def"); end endcase
  case (64'hfffffffffffffffc) {1'((P8 + 5'hd)), 3'((S8 >>> 0))}: begin : h1 initial $display("@Rb4_1 hit"); end default: begin : d1 initial $display("@Rb4_1 def"); end endcase
  case ((-4)) {1'((P8 + 5'hd)), 3'((S8 >>> 0))}: begin : h2 initial $display("@Rb4_2 hit"); end default: begin : d2 initial $display("@Rb4_2 def"); end endcase
  case (12) {1'((P8 + 5'hd)), 3'((S8 >>> 0))}: begin : h3 initial $display("@Rb4_3 hit"); end default: begin : d3 initial $display("@Rb4_3 def"); end endcase
  case (32'h7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h4 initial $display("@Rb4_4 hit"); end default: begin : d4 initial $display("@Rb4_4 def"); end endcase
  case (32'sh7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h5 initial $display("@Rb4_5 hit"); end default: begin : d5 initial $display("@Rb4_5 def"); end endcase
  case (35'h7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h6 initial $display("@Rb4_6 hit"); end default: begin : d6 initial $display("@Rb4_6 def"); end endcase
  case (64'h7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h7 initial $display("@Rb4_7 hit"); end default: begin : d7 initial $display("@Rb4_7 def"); end endcase
  case (7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h8 initial $display("@Rb4_8 hit"); end default: begin : d8 initial $display("@Rb4_8 def"); end endcase
  case (7) (-((63'sh70a90ba0fd3c5494 >= P4) ? ((-7) | (-8)) : (16'he7fe | 4'sha))): begin : h9 initial $display("@Rb4_9 hit"); end default: begin : d9 initial $display("@Rb4_9 def"); end endcase
  case (63'h10c7a9b8594eba94) (63'h431ea6e1653aea52 >> 2): begin : h10 initial $display("@Rb4_10 hit"); end default: begin : d10 initial $display("@Rb4_10 def"); end endcase
  case (63'sh10c7a9b8594eba94) (63'h431ea6e1653aea52 >> 2): begin : h11 initial $display("@Rb4_11 hit"); end default: begin : d11 initial $display("@Rb4_11 def"); end endcase
  case (64'h10c7a9b8594eba94) (63'h431ea6e1653aea52 >> 2): begin : h12 initial $display("@Rb4_12 hit"); end default: begin : d12 initial $display("@Rb4_12 def"); end endcase
  case (64'h10c7a9b8594eba94) (63'h431ea6e1653aea52 >> 2): begin : h13 initial $display("@Rb4_13 hit"); end default: begin : d13 initial $display("@Rb4_13 def"); end endcase
  case (32'hc4) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h14 initial $display("@Rb4_14 hit"); end default: begin : d14 initial $display("@Rb4_14 def"); end endcase
  case (32'shc4) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h15 initial $display("@Rb4_15 hit"); end default: begin : d15 initial $display("@Rb4_15 def"); end endcase
  case (35'hc4) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h16 initial $display("@Rb4_16 hit"); end default: begin : d16 initial $display("@Rb4_16 def"); end endcase
  case (64'hc4) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h17 initial $display("@Rb4_17 hit"); end default: begin : d17 initial $display("@Rb4_17 def"); end endcase
  case (196) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h18 initial $display("@Rb4_18 hit"); end default: begin : d18 initial $display("@Rb4_18 def"); end endcase
  case (196) ({2{4'hB}} - (3'sh7 + $unsigned(U32))): begin : h19 initial $display("@Rb4_19 hit"); end default: begin : d19 initial $display("@Rb4_19 def"); end endcase
  case (32'hfffffffb) (I >>> 0): begin : h20 initial $display("@Rb4_20 hit"); end default: begin : d20 initial $display("@Rb4_20 def"); end endcase
  case (32'shfffffffb) (I >>> 0): begin : h21 initial $display("@Rb4_21 hit"); end default: begin : d21 initial $display("@Rb4_21 def"); end endcase
  case (35'hfffffffb) (I >>> 0): begin : h22 initial $display("@Rb4_22 hit"); end default: begin : d22 initial $display("@Rb4_22 def"); end endcase
  case (64'hfffffffffffffffb) (I >>> 0): begin : h23 initial $display("@Rb4_23 hit"); end default: begin : d23 initial $display("@Rb4_23 def"); end endcase
  case ((-5)) (I >>> 0): begin : h24 initial $display("@Rb4_24 hit"); end default: begin : d24 initial $display("@Rb4_24 def"); end endcase
endmodule
