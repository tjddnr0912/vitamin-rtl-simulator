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
  typedef logic signed [5:0] s6_t;
  localparam int W6 = 6;
  localparam logic [127:0] P128 = {64'hFFFF_FFFF_FFFF_FFFF, 64'h0123_4567_89AB_CDEF};
  case (4'hc) P4: begin : h0 initial $display("@Rd17_0 hit"); end default: begin : d0 initial $display("@Rd17_0 def"); end endcase
  case (4'shc) P4: begin : h1 initial $display("@Rd17_1 hit"); end default: begin : d1 initial $display("@Rd17_1 def"); end endcase
  case (7'hc) P4: begin : h2 initial $display("@Rd17_2 hit"); end default: begin : d2 initial $display("@Rd17_2 def"); end endcase
  case (64'hfffffffffffffffc) P4: begin : h3 initial $display("@Rd17_3 hit"); end default: begin : d3 initial $display("@Rd17_3 def"); end endcase
  case ((-4)) P4: begin : h4 initial $display("@Rd17_4 hit"); end default: begin : d4 initial $display("@Rd17_4 def"); end endcase
  case (12) P4: begin : h5 initial $display("@Rd17_5 hit"); end default: begin : d5 initial $display("@Rd17_5 def"); end endcase
  case (63'h184f6935923574ba) (5'sh4 ? 63'h184f6935923574ba : S4): begin : h6 initial $display("@Rd17_6 hit"); end default: begin : d6 initial $display("@Rd17_6 def"); end endcase
  case (63'sh184f6935923574ba) (5'sh4 ? 63'h184f6935923574ba : S4): begin : h7 initial $display("@Rd17_7 hit"); end default: begin : d7 initial $display("@Rd17_7 def"); end endcase
  case (64'h184f6935923574ba) (5'sh4 ? 63'h184f6935923574ba : S4): begin : h8 initial $display("@Rd17_8 hit"); end default: begin : d8 initial $display("@Rd17_8 def"); end endcase
  case (64'h184f6935923574ba) (5'sh4 ? 63'h184f6935923574ba : S4): begin : h9 initial $display("@Rd17_9 hit"); end default: begin : d9 initial $display("@Rd17_9 def"); end endcase
  case (8'h11) {2{4'((|((-8) >> 0)))}}: begin : h10 initial $display("@Rd17_10 hit"); end default: begin : d10 initial $display("@Rd17_10 def"); end endcase
  case (8'sh11) {2{4'((|((-8) >> 0)))}}: begin : h11 initial $display("@Rd17_11 hit"); end default: begin : d11 initial $display("@Rd17_11 def"); end endcase
  case (11'h11) {2{4'((|((-8) >> 0)))}}: begin : h12 initial $display("@Rd17_12 hit"); end default: begin : d12 initial $display("@Rd17_12 def"); end endcase
  case (64'h11) {2{4'((|((-8) >> 0)))}}: begin : h13 initial $display("@Rd17_13 hit"); end default: begin : d13 initial $display("@Rd17_13 def"); end endcase
  case (17) {2{4'((|((-8) >> 0)))}}: begin : h14 initial $display("@Rd17_14 hit"); end default: begin : d14 initial $display("@Rd17_14 def"); end endcase
  case (17) {2{4'((|((-8) >> 0)))}}: begin : h15 initial $display("@Rd17_15 hit"); end default: begin : d15 initial $display("@Rd17_15 def"); end endcase
  case (1'h0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h16 initial $display("@Rd17_16 hit"); end default: begin : d16 initial $display("@Rd17_16 def"); end endcase
  case (1'sh0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h17 initial $display("@Rd17_17 hit"); end default: begin : d17 initial $display("@Rd17_17 def"); end endcase
  case (4'h0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h18 initial $display("@Rd17_18 hit"); end default: begin : d18 initial $display("@Rd17_18 def"); end endcase
  case (64'h0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h19 initial $display("@Rd17_19 hit"); end default: begin : d19 initial $display("@Rd17_19 def"); end endcase
  case (0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h20 initial $display("@Rd17_20 hit"); end default: begin : d20 initial $display("@Rd17_20 def"); end endcase
  case (0) ((64'sha725ef3d3075de6c >> 3) == (65'h1ae3216141bcd3cb5 >>> 2)): begin : h21 initial $display("@Rd17_21 hit"); end default: begin : d21 initial $display("@Rd17_21 def"); end endcase
  case (1'h1) (^(I ? (-5) : P8)): begin : h22 initial $display("@Rd17_22 hit"); end default: begin : d22 initial $display("@Rd17_22 def"); end endcase
  case (1'sh1) (^(I ? (-5) : P8)): begin : h23 initial $display("@Rd17_23 hit"); end default: begin : d23 initial $display("@Rd17_23 def"); end endcase
  case (4'h1) (^(I ? (-5) : P8)): begin : h24 initial $display("@Rd17_24 hit"); end default: begin : d24 initial $display("@Rd17_24 def"); end endcase
endmodule
