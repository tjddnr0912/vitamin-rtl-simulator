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
  case ((-25444)) {2{8'(S8)}}: begin : h0 initial $display("@Rd9_0 hit"); end default: begin : d0 initial $display("@Rd9_0 def"); end endcase
  case (40092) {2{8'(S8)}}: begin : h1 initial $display("@Rd9_1 hit"); end default: begin : d1 initial $display("@Rd9_1 def"); end endcase
  case (1'h0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h2 initial $display("@Rd9_2 hit"); end default: begin : d2 initial $display("@Rd9_2 def"); end endcase
  case (1'sh0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h3 initial $display("@Rd9_3 hit"); end default: begin : d3 initial $display("@Rd9_3 def"); end endcase
  case (4'h0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h4 initial $display("@Rd9_4 hit"); end default: begin : d4 initial $display("@Rd9_4 def"); end endcase
  case (64'h0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h5 initial $display("@Rd9_5 hit"); end default: begin : d5 initial $display("@Rd9_5 def"); end endcase
  case (0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h6 initial $display("@Rd9_6 hit"); end default: begin : d6 initial $display("@Rd9_6 def"); end endcase
  case (0) ({2{4'hB}} >= ((-8) | 8'h37)): begin : h7 initial $display("@Rd9_7 hit"); end default: begin : d7 initial $display("@Rd9_7 def"); end endcase
  case (5'h7) 5'h7: begin : h8 initial $display("@Rd9_8 hit"); end default: begin : d8 initial $display("@Rd9_8 def"); end endcase
  case (5'sh7) 5'h7: begin : h9 initial $display("@Rd9_9 hit"); end default: begin : d9 initial $display("@Rd9_9 def"); end endcase
  case (8'h7) 5'h7: begin : h10 initial $display("@Rd9_10 hit"); end default: begin : d10 initial $display("@Rd9_10 def"); end endcase
  case (64'h7) 5'h7: begin : h11 initial $display("@Rd9_11 hit"); end default: begin : d11 initial $display("@Rd9_11 def"); end endcase
  case (7) 5'h7: begin : h12 initial $display("@Rd9_12 hit"); end default: begin : d12 initial $display("@Rd9_12 def"); end endcase
  case (7) 5'h7: begin : h13 initial $display("@Rd9_13 hit"); end default: begin : d13 initial $display("@Rd9_13 def"); end endcase
  case (1'h0) (~|UN): begin : h14 initial $display("@Rd9_14 hit"); end default: begin : d14 initial $display("@Rd9_14 def"); end endcase
  case (1'sh0) (~|UN): begin : h15 initial $display("@Rd9_15 hit"); end default: begin : d15 initial $display("@Rd9_15 def"); end endcase
  case (4'h0) (~|UN): begin : h16 initial $display("@Rd9_16 hit"); end default: begin : d16 initial $display("@Rd9_16 def"); end endcase
  case (64'h0) (~|UN): begin : h17 initial $display("@Rd9_17 hit"); end default: begin : d17 initial $display("@Rd9_17 def"); end endcase
  case (0) (~|UN): begin : h18 initial $display("@Rd9_18 hit"); end default: begin : d18 initial $display("@Rd9_18 def"); end endcase
  case (0) (~|UN): begin : h19 initial $display("@Rd9_19 hit"); end default: begin : d19 initial $display("@Rd9_19 def"); end endcase
  case (63'hd) (64'sh189cafb3e52659f2 ? S4 : 63'h5c6596da6af14caf): begin : h20 initial $display("@Rd9_20 hit"); end default: begin : d20 initial $display("@Rd9_20 def"); end endcase
  case (63'shd) (64'sh189cafb3e52659f2 ? S4 : 63'h5c6596da6af14caf): begin : h21 initial $display("@Rd9_21 hit"); end default: begin : d21 initial $display("@Rd9_21 def"); end endcase
  case (64'hd) (64'sh189cafb3e52659f2 ? S4 : 63'h5c6596da6af14caf): begin : h22 initial $display("@Rd9_22 hit"); end default: begin : d22 initial $display("@Rd9_22 def"); end endcase
  case (64'h800000000000000d) (64'sh189cafb3e52659f2 ? S4 : 63'h5c6596da6af14caf): begin : h23 initial $display("@Rd9_23 hit"); end default: begin : d23 initial $display("@Rd9_23 def"); end endcase
  case (13) (64'sh189cafb3e52659f2 ? S4 : 63'h5c6596da6af14caf): begin : h24 initial $display("@Rd9_24 hit"); end default: begin : d24 initial $display("@Rd9_24 def"); end endcase
endmodule
