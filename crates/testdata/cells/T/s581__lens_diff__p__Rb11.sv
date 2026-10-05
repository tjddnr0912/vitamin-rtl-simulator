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
  case (64'hfffffffffffffff7) (((~(-2)) >= 1'sh1) | 5'sh16): begin : h0 initial $display("@Rb11_0 hit"); end default: begin : d0 initial $display("@Rb11_0 def"); end endcase
  case ((-9)) (((~(-2)) >= 1'sh1) | 5'sh16): begin : h1 initial $display("@Rb11_1 hit"); end default: begin : d1 initial $display("@Rb11_1 def"); end endcase
  case (23) (((~(-2)) >= 1'sh1) | 5'sh16): begin : h2 initial $display("@Rb11_2 hit"); end default: begin : d2 initial $display("@Rb11_2 def"); end endcase
  case (2'h3) {2{1'(29)}}: begin : h3 initial $display("@Rb11_3 hit"); end default: begin : d3 initial $display("@Rb11_3 def"); end endcase
  case (2'sh3) {2{1'(29)}}: begin : h4 initial $display("@Rb11_4 hit"); end default: begin : d4 initial $display("@Rb11_4 def"); end endcase
  case (5'h3) {2{1'(29)}}: begin : h5 initial $display("@Rb11_5 hit"); end default: begin : d5 initial $display("@Rb11_5 def"); end endcase
  case (64'hffffffffffffffff) {2{1'(29)}}: begin : h6 initial $display("@Rb11_6 hit"); end default: begin : d6 initial $display("@Rb11_6 def"); end endcase
  case ((-1)) {2{1'(29)}}: begin : h7 initial $display("@Rb11_7 hit"); end default: begin : d7 initial $display("@Rb11_7 def"); end endcase
  case (3) {2{1'(29)}}: begin : h8 initial $display("@Rb11_8 hit"); end default: begin : d8 initial $display("@Rb11_8 def"); end endcase
  case (1'h1) ((U32 ? P4 : 13) < U32): begin : h9 initial $display("@Rb11_9 hit"); end default: begin : d9 initial $display("@Rb11_9 def"); end endcase
  case (1'sh1) ((U32 ? P4 : 13) < U32): begin : h10 initial $display("@Rb11_10 hit"); end default: begin : d10 initial $display("@Rb11_10 def"); end endcase
  case (4'h1) ((U32 ? P4 : 13) < U32): begin : h11 initial $display("@Rb11_11 hit"); end default: begin : d11 initial $display("@Rb11_11 def"); end endcase
  case (64'hffffffffffffffff) ((U32 ? P4 : 13) < U32): begin : h12 initial $display("@Rb11_12 hit"); end default: begin : d12 initial $display("@Rb11_12 def"); end endcase
  case ((-1)) ((U32 ? P4 : 13) < U32): begin : h13 initial $display("@Rb11_13 hit"); end default: begin : d13 initial $display("@Rb11_13 def"); end endcase
  case (1) ((U32 ? P4 : 13) < U32): begin : h14 initial $display("@Rb11_14 hit"); end default: begin : d14 initial $display("@Rb11_14 def"); end endcase
  case (32'h5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h15 initial $display("@Rb11_15 hit"); end default: begin : d15 initial $display("@Rb11_15 def"); end endcase
  case (32'sh5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h16 initial $display("@Rb11_16 hit"); end default: begin : d16 initial $display("@Rb11_16 def"); end endcase
  case (35'h5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h17 initial $display("@Rb11_17 hit"); end default: begin : d17 initial $display("@Rb11_17 def"); end endcase
  case (64'h5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h18 initial $display("@Rb11_18 hit"); end default: begin : d18 initial $display("@Rb11_18 def"); end endcase
  case (5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h19 initial $display("@Rb11_19 hit"); end default: begin : d19 initial $display("@Rb11_19 def"); end endcase
  case (5) ((I % 5'sh13) / (5 ? 2'sh3 : S8)): begin : h20 initial $display("@Rb11_20 hit"); end default: begin : d20 initial $display("@Rb11_20 def"); end endcase
  case (32'hfffffff) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h21 initial $display("@Rb11_21 hit"); end default: begin : d21 initial $display("@Rb11_21 def"); end endcase
  case (32'shfffffff) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h22 initial $display("@Rb11_22 hit"); end default: begin : d22 initial $display("@Rb11_22 def"); end endcase
  case (35'hfffffff) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h23 initial $display("@Rb11_23 hit"); end default: begin : d23 initial $display("@Rb11_23 def"); end endcase
  case (64'hfffffff) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h24 initial $display("@Rb11_24 hit"); end default: begin : d24 initial $display("@Rb11_24 def"); end endcase
endmodule
