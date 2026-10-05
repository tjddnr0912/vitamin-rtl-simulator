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
  case (5'h19) {P4, 1'(32'h98d57e03)}: begin : h0 initial $display("@Rd26_0 hit"); end default: begin : d0 initial $display("@Rd26_0 def"); end endcase
  case (5'sh19) {P4, 1'(32'h98d57e03)}: begin : h1 initial $display("@Rd26_1 hit"); end default: begin : d1 initial $display("@Rd26_1 def"); end endcase
  case (8'h19) {P4, 1'(32'h98d57e03)}: begin : h2 initial $display("@Rd26_2 hit"); end default: begin : d2 initial $display("@Rd26_2 def"); end endcase
  case (64'hfffffffffffffff9) {P4, 1'(32'h98d57e03)}: begin : h3 initial $display("@Rd26_3 hit"); end default: begin : d3 initial $display("@Rd26_3 def"); end endcase
  case ((-7)) {P4, 1'(32'h98d57e03)}: begin : h4 initial $display("@Rd26_4 hit"); end default: begin : d4 initial $display("@Rd26_4 def"); end endcase
  case (25) {P4, 1'(32'h98d57e03)}: begin : h5 initial $display("@Rd26_5 hit"); end default: begin : d5 initial $display("@Rd26_5 def"); end endcase
  case (1'h1) $signed((|63'sh42e095433da0453b)): begin : h6 initial $display("@Rd26_6 hit"); end default: begin : d6 initial $display("@Rd26_6 def"); end endcase
  case (1'sh1) $signed((|63'sh42e095433da0453b)): begin : h7 initial $display("@Rd26_7 hit"); end default: begin : d7 initial $display("@Rd26_7 def"); end endcase
  case (4'h1) $signed((|63'sh42e095433da0453b)): begin : h8 initial $display("@Rd26_8 hit"); end default: begin : d8 initial $display("@Rd26_8 def"); end endcase
  case (64'hffffffffffffffff) $signed((|63'sh42e095433da0453b)): begin : h9 initial $display("@Rd26_9 hit"); end default: begin : d9 initial $display("@Rd26_9 def"); end endcase
  case ((-1)) $signed((|63'sh42e095433da0453b)): begin : h10 initial $display("@Rd26_10 hit"); end default: begin : d10 initial $display("@Rd26_10 def"); end endcase
  case (1) $signed((|63'sh42e095433da0453b)): begin : h11 initial $display("@Rd26_11 hit"); end default: begin : d11 initial $display("@Rd26_11 def"); end endcase
  case (32'h0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h12 initial $display("@Rd26_12 hit"); end default: begin : d12 initial $display("@Rd26_12 def"); end endcase
  case (32'sh0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h13 initial $display("@Rd26_13 hit"); end default: begin : d13 initial $display("@Rd26_13 def"); end endcase
  case (35'h0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h14 initial $display("@Rd26_14 hit"); end default: begin : d14 initial $display("@Rd26_14 def"); end endcase
  case (64'h0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h15 initial $display("@Rd26_15 hit"); end default: begin : d15 initial $display("@Rd26_15 def"); end endcase
  case (0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h16 initial $display("@Rd26_16 hit"); end default: begin : d16 initial $display("@Rd26_16 def"); end endcase
  case (0) ((~&(S65 ** 1)) ? unsigned'((~|P8)) : $signed(32'h62a615c4)): begin : h17 initial $display("@Rd26_17 hit"); end default: begin : d17 initial $display("@Rd26_17 def"); end endcase
endmodule
