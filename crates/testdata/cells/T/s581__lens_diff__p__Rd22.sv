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
  case (64'hffffffffffffffff) ((16'shc644 >>> 0) || {2{P8}}): begin : h0 initial $display("@Rd22_0 hit"); end default: begin : d0 initial $display("@Rd22_0 def"); end endcase
  case ((-1)) ((16'shc644 >>> 0) || {2{P8}}): begin : h1 initial $display("@Rd22_1 hit"); end default: begin : d1 initial $display("@Rd22_1 def"); end endcase
  case (1) ((16'shc644 >>> 0) || {2{P8}}): begin : h2 initial $display("@Rd22_2 hit"); end default: begin : d2 initial $display("@Rd22_2 def"); end endcase
  case (12'hdf9) unsigned'({S4, 8'((-7))}): begin : h3 initial $display("@Rd22_3 hit"); end default: begin : d3 initial $display("@Rd22_3 def"); end endcase
  case (12'shdf9) unsigned'({S4, 8'((-7))}): begin : h4 initial $display("@Rd22_4 hit"); end default: begin : d4 initial $display("@Rd22_4 def"); end endcase
  case (15'hdf9) unsigned'({S4, 8'((-7))}): begin : h5 initial $display("@Rd22_5 hit"); end default: begin : d5 initial $display("@Rd22_5 def"); end endcase
  case (64'hfffffffffffffdf9) unsigned'({S4, 8'((-7))}): begin : h6 initial $display("@Rd22_6 hit"); end default: begin : d6 initial $display("@Rd22_6 def"); end endcase
  case ((-519)) unsigned'({S4, 8'((-7))}): begin : h7 initial $display("@Rd22_7 hit"); end default: begin : d7 initial $display("@Rd22_7 def"); end endcase
  case (3577) unsigned'({S4, 8'((-7))}): begin : h8 initial $display("@Rd22_8 hit"); end default: begin : d8 initial $display("@Rd22_8 def"); end endcase
  case (1'h0) $onehot(65'($unsigned(U32))): begin : h9 initial $display("@Rd22_9 hit"); end default: begin : d9 initial $display("@Rd22_9 def"); end endcase
  case (1'sh0) $onehot(65'($unsigned(U32))): begin : h10 initial $display("@Rd22_10 hit"); end default: begin : d10 initial $display("@Rd22_10 def"); end endcase
  case (4'h0) $onehot(65'($unsigned(U32))): begin : h11 initial $display("@Rd22_11 hit"); end default: begin : d11 initial $display("@Rd22_11 def"); end endcase
  case (64'h0) $onehot(65'($unsigned(U32))): begin : h12 initial $display("@Rd22_12 hit"); end default: begin : d12 initial $display("@Rd22_12 def"); end endcase
  case (0) $onehot(65'($unsigned(U32))): begin : h13 initial $display("@Rd22_13 hit"); end default: begin : d13 initial $display("@Rd22_13 def"); end endcase
  case (0) $onehot(65'($unsigned(U32))): begin : h14 initial $display("@Rd22_14 hit"); end default: begin : d14 initial $display("@Rd22_14 def"); end endcase
  case (6'h4) W6'((S8 >> 5)): begin : h15 initial $display("@Rd22_15 hit"); end default: begin : d15 initial $display("@Rd22_15 def"); end endcase
  case (6'sh4) W6'((S8 >> 5)): begin : h16 initial $display("@Rd22_16 hit"); end default: begin : d16 initial $display("@Rd22_16 def"); end endcase
  case (9'h4) W6'((S8 >> 5)): begin : h17 initial $display("@Rd22_17 hit"); end default: begin : d17 initial $display("@Rd22_17 def"); end endcase
  case (64'h4) W6'((S8 >> 5)): begin : h18 initial $display("@Rd22_18 hit"); end default: begin : d18 initial $display("@Rd22_18 def"); end endcase
  case (4) W6'((S8 >> 5)): begin : h19 initial $display("@Rd22_19 hit"); end default: begin : d19 initial $display("@Rd22_19 def"); end endcase
  case (4) W6'((S8 >> 5)): begin : h20 initial $display("@Rd22_20 hit"); end default: begin : d20 initial $display("@Rd22_20 def"); end endcase
  case (1'h0) $onehot(33'h1_0000_0001): begin : h21 initial $display("@Rd22_21 hit"); end default: begin : d21 initial $display("@Rd22_21 def"); end endcase
  case (1'sh0) $onehot(33'h1_0000_0001): begin : h22 initial $display("@Rd22_22 hit"); end default: begin : d22 initial $display("@Rd22_22 def"); end endcase
  case (4'h0) $onehot(33'h1_0000_0001): begin : h23 initial $display("@Rd22_23 hit"); end default: begin : d23 initial $display("@Rd22_23 def"); end endcase
  case (64'h0) $onehot(33'h1_0000_0001): begin : h24 initial $display("@Rd22_24 hit"); end default: begin : d24 initial $display("@Rd22_24 def"); end endcase
endmodule
