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
  case (64'hffffffffffffffff) (^(I ? (-5) : P8)): begin : h0 initial $display("@Rd18_0 hit"); end default: begin : d0 initial $display("@Rd18_0 def"); end endcase
  case ((-1)) (^(I ? (-5) : P8)): begin : h1 initial $display("@Rd18_1 hit"); end default: begin : d1 initial $display("@Rd18_1 def"); end endcase
  case (1) (^(I ? (-5) : P8)): begin : h2 initial $display("@Rd18_2 hit"); end default: begin : d2 initial $display("@Rd18_2 def"); end endcase
  case (3'h0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h3 initial $display("@Rd18_3 hit"); end default: begin : d3 initial $display("@Rd18_3 def"); end endcase
  case (3'sh0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h4 initial $display("@Rd18_4 hit"); end default: begin : d4 initial $display("@Rd18_4 def"); end endcase
  case (6'h0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h5 initial $display("@Rd18_5 hit"); end default: begin : d5 initial $display("@Rd18_5 def"); end endcase
  case (64'h0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h6 initial $display("@Rd18_6 hit"); end default: begin : d6 initial $display("@Rd18_6 def"); end endcase
  case (0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h7 initial $display("@Rd18_7 hit"); end default: begin : d7 initial $display("@Rd18_7 def"); end endcase
  case (0) ((UN ? 1'h1 : 3'sh1) ^ (UN < 16'ha695)): begin : h8 initial $display("@Rd18_8 hit"); end default: begin : d8 initial $display("@Rd18_8 def"); end endcase
  case (63'h799990325f0df03) (63'h799990325f0df01 ^ 4'sh2): begin : h9 initial $display("@Rd18_9 hit"); end default: begin : d9 initial $display("@Rd18_9 def"); end endcase
  case (63'sh799990325f0df03) (63'h799990325f0df01 ^ 4'sh2): begin : h10 initial $display("@Rd18_10 hit"); end default: begin : d10 initial $display("@Rd18_10 def"); end endcase
  case (64'h799990325f0df03) (63'h799990325f0df01 ^ 4'sh2): begin : h11 initial $display("@Rd18_11 hit"); end default: begin : d11 initial $display("@Rd18_11 def"); end endcase
  case (64'h799990325f0df03) (63'h799990325f0df01 ^ 4'sh2): begin : h12 initial $display("@Rd18_12 hit"); end default: begin : d12 initial $display("@Rd18_12 def"); end endcase
  case (63'h5f3452534f12bff3) 63'h5f3452534f12bff3: begin : h13 initial $display("@Rd18_13 hit"); end default: begin : d13 initial $display("@Rd18_13 def"); end endcase
  case (63'sh5f3452534f12bff3) 63'h5f3452534f12bff3: begin : h14 initial $display("@Rd18_14 hit"); end default: begin : d14 initial $display("@Rd18_14 def"); end endcase
  case (64'h5f3452534f12bff3) 63'h5f3452534f12bff3: begin : h15 initial $display("@Rd18_15 hit"); end default: begin : d15 initial $display("@Rd18_15 def"); end endcase
  case (64'hdf3452534f12bff3) 63'h5f3452534f12bff3: begin : h16 initial $display("@Rd18_16 hit"); end default: begin : d16 initial $display("@Rd18_16 def"); end endcase
  case (6'h24) {2{3'($signed(P4))}}: begin : h17 initial $display("@Rd18_17 hit"); end default: begin : d17 initial $display("@Rd18_17 def"); end endcase
  case (6'sh24) {2{3'($signed(P4))}}: begin : h18 initial $display("@Rd18_18 hit"); end default: begin : d18 initial $display("@Rd18_18 def"); end endcase
  case (9'h24) {2{3'($signed(P4))}}: begin : h19 initial $display("@Rd18_19 hit"); end default: begin : d19 initial $display("@Rd18_19 def"); end endcase
  case (64'hffffffffffffffe4) {2{3'($signed(P4))}}: begin : h20 initial $display("@Rd18_20 hit"); end default: begin : d20 initial $display("@Rd18_20 def"); end endcase
  case ((-28)) {2{3'($signed(P4))}}: begin : h21 initial $display("@Rd18_21 hit"); end default: begin : d21 initial $display("@Rd18_21 def"); end endcase
  case (36) {2{3'($signed(P4))}}: begin : h22 initial $display("@Rd18_22 hit"); end default: begin : d22 initial $display("@Rd18_22 def"); end endcase
  case (1'h0) (^$signed(S8)): begin : h23 initial $display("@Rd18_23 hit"); end default: begin : d23 initial $display("@Rd18_23 def"); end endcase
  case (1'sh0) (^$signed(S8)): begin : h24 initial $display("@Rd18_24 hit"); end default: begin : d24 initial $display("@Rd18_24 def"); end endcase
endmodule
