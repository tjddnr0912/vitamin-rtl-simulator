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
  case (0) (&$unsigned((!4'sh8))): begin : h0 initial $display("@Rd4_0 hit"); end default: begin : d0 initial $display("@Rd4_0 def"); end endcase
  case (32'hfffffff0) U32: begin : h1 initial $display("@Rd4_1 hit"); end default: begin : d1 initial $display("@Rd4_1 def"); end endcase
  case (32'shfffffff0) U32: begin : h2 initial $display("@Rd4_2 hit"); end default: begin : d2 initial $display("@Rd4_2 def"); end endcase
  case (35'hfffffff0) U32: begin : h3 initial $display("@Rd4_3 hit"); end default: begin : d3 initial $display("@Rd4_3 def"); end endcase
  case (64'hfffffffffffffff0) U32: begin : h4 initial $display("@Rd4_4 hit"); end default: begin : d4 initial $display("@Rd4_4 def"); end endcase
  case ((-16)) U32: begin : h5 initial $display("@Rd4_5 hit"); end default: begin : d5 initial $display("@Rd4_5 def"); end endcase
  case (6'h1a) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h6 initial $display("@Rd4_6 hit"); end default: begin : d6 initial $display("@Rd4_6 def"); end endcase
  case (6'sh1a) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h7 initial $display("@Rd4_7 hit"); end default: begin : d7 initial $display("@Rd4_7 def"); end endcase
  case (9'h1a) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h8 initial $display("@Rd4_8 hit"); end default: begin : d8 initial $display("@Rd4_8 def"); end endcase
  case (64'h1a) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h9 initial $display("@Rd4_9 hit"); end default: begin : d9 initial $display("@Rd4_9 def"); end endcase
  case (26) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h10 initial $display("@Rd4_10 hit"); end default: begin : d10 initial $display("@Rd4_10 def"); end endcase
  case (26) 6'((signed'(64'hc8be424831159b7a) * signed'(33'h1c556efb1))): begin : h11 initial $display("@Rd4_11 hit"); end default: begin : d11 initial $display("@Rd4_11 def"); end endcase
  case (1'h0) $onehot(65'(8'shed)): begin : h12 initial $display("@Rd4_12 hit"); end default: begin : d12 initial $display("@Rd4_12 def"); end endcase
  case (1'sh0) $onehot(65'(8'shed)): begin : h13 initial $display("@Rd4_13 hit"); end default: begin : d13 initial $display("@Rd4_13 def"); end endcase
  case (4'h0) $onehot(65'(8'shed)): begin : h14 initial $display("@Rd4_14 hit"); end default: begin : d14 initial $display("@Rd4_14 def"); end endcase
  case (64'h0) $onehot(65'(8'shed)): begin : h15 initial $display("@Rd4_15 hit"); end default: begin : d15 initial $display("@Rd4_15 def"); end endcase
  case (0) $onehot(65'(8'shed)): begin : h16 initial $display("@Rd4_16 hit"); end default: begin : d16 initial $display("@Rd4_16 def"); end endcase
  case (0) $onehot(65'(8'shed)): begin : h17 initial $display("@Rd4_17 hit"); end default: begin : d17 initial $display("@Rd4_17 def"); end endcase
  case (41'h1ffffffffcd) {33'((s6_t'(32'hb9c5233f) >>> (^P128))), 8'($signed({S8, 4'(65'sh1d1451bc51905960d)}))}: begin : h18 initial $display("@Rd4_18 hit"); end default: begin : d18 initial $display("@Rd4_18 def"); end endcase
  case (41'sh1ffffffffcd) {33'((s6_t'(32'hb9c5233f) >>> (^P128))), 8'($signed({S8, 4'(65'sh1d1451bc51905960d)}))}: begin : h19 initial $display("@Rd4_19 hit"); end default: begin : d19 initial $display("@Rd4_19 def"); end endcase
  case (44'h1ffffffffcd) {33'((s6_t'(32'hb9c5233f) >>> (^P128))), 8'($signed({S8, 4'(65'sh1d1451bc51905960d)}))}: begin : h20 initial $display("@Rd4_20 hit"); end default: begin : d20 initial $display("@Rd4_20 def"); end endcase
  case (64'hffffffffffffffcd) {33'((s6_t'(32'hb9c5233f) >>> (^P128))), 8'($signed({S8, 4'(65'sh1d1451bc51905960d)}))}: begin : h21 initial $display("@Rd4_21 hit"); end default: begin : d21 initial $display("@Rd4_21 def"); end endcase
  case ((-51)) {33'((s6_t'(32'hb9c5233f) >>> (^P128))), 8'($signed({S8, 4'(65'sh1d1451bc51905960d)}))}: begin : h22 initial $display("@Rd4_22 hit"); end default: begin : d22 initial $display("@Rd4_22 def"); end endcase
  case (11'h783) {P8, 3'(W6'(3'h3))}: begin : h23 initial $display("@Rd4_23 hit"); end default: begin : d23 initial $display("@Rd4_23 def"); end endcase
  case (11'sh783) {P8, 3'(W6'(3'h3))}: begin : h24 initial $display("@Rd4_24 hit"); end default: begin : d24 initial $display("@Rd4_24 def"); end endcase
endmodule
