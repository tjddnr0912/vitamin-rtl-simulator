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
  case (64'hfffffffffffffffe) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h0 initial $display("@Rd20_0 hit"); end default: begin : d0 initial $display("@Rd20_0 def"); end endcase
  case ((-2)) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h1 initial $display("@Rd20_1 hit"); end default: begin : d1 initial $display("@Rd20_1 def"); end endcase
  case (254) ({2{S4}} ? (-3'sh2) : {S4, S4}): begin : h2 initial $display("@Rd20_2 hit"); end default: begin : d2 initial $display("@Rd20_2 def"); end endcase
  case (12'hfdd) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h3 initial $display("@Rd20_3 hit"); end default: begin : d3 initial $display("@Rd20_3 def"); end endcase
  case (12'shfdd) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h4 initial $display("@Rd20_4 hit"); end default: begin : d4 initial $display("@Rd20_4 def"); end endcase
  case (15'hfdd) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h5 initial $display("@Rd20_5 hit"); end default: begin : d5 initial $display("@Rd20_5 def"); end endcase
  case (64'hffffffffffffffdd) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h6 initial $display("@Rd20_6 hit"); end default: begin : d6 initial $display("@Rd20_6 def"); end endcase
  case ((-35)) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h7 initial $display("@Rd20_7 hit"); end default: begin : d7 initial $display("@Rd20_7 def"); end endcase
  case (4061) ((8'h32 & (5'h17 || 1'h1)) | ($signed(S8) ? {8'(S4), S4} : (P4 >> 5))): begin : h8 initial $display("@Rd20_8 hit"); end default: begin : d8 initial $display("@Rd20_8 def"); end endcase
  case (1'h0) (32'hebc93a79 == (-1)): begin : h9 initial $display("@Rd20_9 hit"); end default: begin : d9 initial $display("@Rd20_9 def"); end endcase
  case (1'sh0) (32'hebc93a79 == (-1)): begin : h10 initial $display("@Rd20_10 hit"); end default: begin : d10 initial $display("@Rd20_10 def"); end endcase
  case (4'h0) (32'hebc93a79 == (-1)): begin : h11 initial $display("@Rd20_11 hit"); end default: begin : d11 initial $display("@Rd20_11 def"); end endcase
  case (64'h0) (32'hebc93a79 == (-1)): begin : h12 initial $display("@Rd20_12 hit"); end default: begin : d12 initial $display("@Rd20_12 def"); end endcase
  case (0) (32'hebc93a79 == (-1)): begin : h13 initial $display("@Rd20_13 hit"); end default: begin : d13 initial $display("@Rd20_13 def"); end endcase
  case (0) (32'hebc93a79 == (-1)): begin : h14 initial $display("@Rd20_14 hit"); end default: begin : d14 initial $display("@Rd20_14 def"); end endcase
  case (41'h13800000019) {S8, 33'(unsigned'(5'sh19))}: begin : h15 initial $display("@Rd20_15 hit"); end default: begin : d15 initial $display("@Rd20_15 def"); end endcase
  case (41'sh13800000019) {S8, 33'(unsigned'(5'sh19))}: begin : h16 initial $display("@Rd20_16 hit"); end default: begin : d16 initial $display("@Rd20_16 def"); end endcase
  case (44'h13800000019) {S8, 33'(unsigned'(5'sh19))}: begin : h17 initial $display("@Rd20_17 hit"); end default: begin : d17 initial $display("@Rd20_17 def"); end endcase
  case (64'hffffff3800000019) {S8, 33'(unsigned'(5'sh19))}: begin : h18 initial $display("@Rd20_18 hit"); end default: begin : d18 initial $display("@Rd20_18 def"); end endcase
  case (34'h2000000f0) {1'({P65, S4}), 33'((U32 ? P8 : 5'sh1f))}: begin : h19 initial $display("@Rd20_19 hit"); end default: begin : d19 initial $display("@Rd20_19 def"); end endcase
  case (34'sh2000000f0) {1'({P65, S4}), 33'((U32 ? P8 : 5'sh1f))}: begin : h20 initial $display("@Rd20_20 hit"); end default: begin : d20 initial $display("@Rd20_20 def"); end endcase
  case (37'h2000000f0) {1'({P65, S4}), 33'((U32 ? P8 : 5'sh1f))}: begin : h21 initial $display("@Rd20_21 hit"); end default: begin : d21 initial $display("@Rd20_21 def"); end endcase
  case (64'hfffffffe000000f0) {1'({P65, S4}), 33'((U32 ? P8 : 5'sh1f))}: begin : h22 initial $display("@Rd20_22 hit"); end default: begin : d22 initial $display("@Rd20_22 def"); end endcase
  case (4'ha) UN: begin : h23 initial $display("@Rd20_23 hit"); end default: begin : d23 initial $display("@Rd20_23 def"); end endcase
  case (4'sha) UN: begin : h24 initial $display("@Rd20_24 hit"); end default: begin : d24 initial $display("@Rd20_24 def"); end endcase
endmodule
