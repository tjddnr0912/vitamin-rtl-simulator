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
  case ((-1)) (63'sh60bd27c0de27a24e < (-P4)): begin : h0 initial $display("@Ra18_0 hit"); end default: begin : d0 initial $display("@Ra18_0 def"); end endcase
  case (1) (63'sh60bd27c0de27a24e < (-P4)): begin : h1 initial $display("@Ra18_1 hit"); end default: begin : d1 initial $display("@Ra18_1 def"); end endcase
  case (37'h100000001c) {33'h1_0000_0001, P4}: begin : h2 initial $display("@Ra18_2 hit"); end default: begin : d2 initial $display("@Ra18_2 def"); end endcase
  case (37'sh100000001c) {33'h1_0000_0001, P4}: begin : h3 initial $display("@Ra18_3 hit"); end default: begin : d3 initial $display("@Ra18_3 def"); end endcase
  case (40'h100000001c) {33'h1_0000_0001, P4}: begin : h4 initial $display("@Ra18_4 hit"); end default: begin : d4 initial $display("@Ra18_4 def"); end endcase
  case (64'hfffffff00000001c) {33'h1_0000_0001, P4}: begin : h5 initial $display("@Ra18_5 hit"); end default: begin : d5 initial $display("@Ra18_5 def"); end endcase
  case (1'h1) (!{2{4'((!S4))}}): begin : h6 initial $display("@Ra18_6 hit"); end default: begin : d6 initial $display("@Ra18_6 def"); end endcase
  case (1'sh1) (!{2{4'((!S4))}}): begin : h7 initial $display("@Ra18_7 hit"); end default: begin : d7 initial $display("@Ra18_7 def"); end endcase
  case (4'h1) (!{2{4'((!S4))}}): begin : h8 initial $display("@Ra18_8 hit"); end default: begin : d8 initial $display("@Ra18_8 def"); end endcase
  case (64'hffffffffffffffff) (!{2{4'((!S4))}}): begin : h9 initial $display("@Ra18_9 hit"); end default: begin : d9 initial $display("@Ra18_9 def"); end endcase
  case ((-1)) (!{2{4'((!S4))}}): begin : h10 initial $display("@Ra18_10 hit"); end default: begin : d10 initial $display("@Ra18_10 def"); end endcase
  case (1) (!{2{4'((!S4))}}): begin : h11 initial $display("@Ra18_11 hit"); end default: begin : d11 initial $display("@Ra18_11 def"); end endcase
  case (4'h5) ((-P4) + (P8 || (-4))): begin : h12 initial $display("@Ra18_12 hit"); end default: begin : d12 initial $display("@Ra18_12 def"); end endcase
  case (4'sh5) ((-P4) + (P8 || (-4))): begin : h13 initial $display("@Ra18_13 hit"); end default: begin : d13 initial $display("@Ra18_13 def"); end endcase
  case (7'h5) ((-P4) + (P8 || (-4))): begin : h14 initial $display("@Ra18_14 hit"); end default: begin : d14 initial $display("@Ra18_14 def"); end endcase
  case (64'h5) ((-P4) + (P8 || (-4))): begin : h15 initial $display("@Ra18_15 hit"); end default: begin : d15 initial $display("@Ra18_15 def"); end endcase
  case (5) ((-P4) + (P8 || (-4))): begin : h16 initial $display("@Ra18_16 hit"); end default: begin : d16 initial $display("@Ra18_16 def"); end endcase
  case (5) ((-P4) + (P8 || (-4))): begin : h17 initial $display("@Ra18_17 hit"); end default: begin : d17 initial $display("@Ra18_17 def"); end endcase
  case (8'hdf) (8'sh43 ^ S8): begin : h18 initial $display("@Ra18_18 hit"); end default: begin : d18 initial $display("@Ra18_18 def"); end endcase
  case (8'shdf) (8'sh43 ^ S8): begin : h19 initial $display("@Ra18_19 hit"); end default: begin : d19 initial $display("@Ra18_19 def"); end endcase
  case (11'hdf) (8'sh43 ^ S8): begin : h20 initial $display("@Ra18_20 hit"); end default: begin : d20 initial $display("@Ra18_20 def"); end endcase
  case (64'hffffffffffffffdf) (8'sh43 ^ S8): begin : h21 initial $display("@Ra18_21 hit"); end default: begin : d21 initial $display("@Ra18_21 def"); end endcase
  case ((-33)) (8'sh43 ^ S8): begin : h22 initial $display("@Ra18_22 hit"); end default: begin : d22 initial $display("@Ra18_22 def"); end endcase
  case (223) (8'sh43 ^ S8): begin : h23 initial $display("@Ra18_23 hit"); end default: begin : d23 initial $display("@Ra18_23 def"); end endcase
  case (1'h1) ($signed((P8 && 64'sh7055114e76917752)) >>> $signed((64'sh4479c074310afae0 | 4'h9))): begin : h24 initial $display("@Ra18_24 hit"); end default: begin : d24 initial $display("@Ra18_24 def"); end endcase
endmodule
