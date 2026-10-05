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
  case (268435455) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h0 initial $display("@Rb12_0 hit"); end default: begin : d0 initial $display("@Rb12_0 def"); end endcase
  case (268435455) ($signed(3'sh7) ? (U32 >>> 4) : (~S4)): begin : h1 initial $display("@Rb12_1 hit"); end default: begin : d1 initial $display("@Rb12_1 def"); end endcase
  case (4'h3) (-S4): begin : h2 initial $display("@Rb12_2 hit"); end default: begin : d2 initial $display("@Rb12_2 def"); end endcase
  case (4'sh3) (-S4): begin : h3 initial $display("@Rb12_3 hit"); end default: begin : d3 initial $display("@Rb12_3 def"); end endcase
  case (7'h3) (-S4): begin : h4 initial $display("@Rb12_4 hit"); end default: begin : d4 initial $display("@Rb12_4 def"); end endcase
  case (64'h3) (-S4): begin : h5 initial $display("@Rb12_5 hit"); end default: begin : d5 initial $display("@Rb12_5 def"); end endcase
  case (3) (-S4): begin : h6 initial $display("@Rb12_6 hit"); end default: begin : d6 initial $display("@Rb12_6 def"); end endcase
  case (3) (-S4): begin : h7 initial $display("@Rb12_7 hit"); end default: begin : d7 initial $display("@Rb12_7 def"); end endcase
  case (32'h5) (-I): begin : h8 initial $display("@Rb12_8 hit"); end default: begin : d8 initial $display("@Rb12_8 def"); end endcase
  case (32'sh5) (-I): begin : h9 initial $display("@Rb12_9 hit"); end default: begin : d9 initial $display("@Rb12_9 def"); end endcase
  case (35'h5) (-I): begin : h10 initial $display("@Rb12_10 hit"); end default: begin : d10 initial $display("@Rb12_10 def"); end endcase
  case (64'h5) (-I): begin : h11 initial $display("@Rb12_11 hit"); end default: begin : d11 initial $display("@Rb12_11 def"); end endcase
  case (5) (-I): begin : h12 initial $display("@Rb12_12 hit"); end default: begin : d12 initial $display("@Rb12_12 def"); end endcase
  case (5) (-I): begin : h13 initial $display("@Rb12_13 hit"); end default: begin : d13 initial $display("@Rb12_13 def"); end endcase
  case (6'h24) {2{3'($unsigned(S8))}}: begin : h14 initial $display("@Rb12_14 hit"); end default: begin : d14 initial $display("@Rb12_14 def"); end endcase
  case (6'sh24) {2{3'($unsigned(S8))}}: begin : h15 initial $display("@Rb12_15 hit"); end default: begin : d15 initial $display("@Rb12_15 def"); end endcase
  case (9'h24) {2{3'($unsigned(S8))}}: begin : h16 initial $display("@Rb12_16 hit"); end default: begin : d16 initial $display("@Rb12_16 def"); end endcase
  case (64'hffffffffffffffe4) {2{3'($unsigned(S8))}}: begin : h17 initial $display("@Rb12_17 hit"); end default: begin : d17 initial $display("@Rb12_17 def"); end endcase
  case ((-28)) {2{3'($unsigned(S8))}}: begin : h18 initial $display("@Rb12_18 hit"); end default: begin : d18 initial $display("@Rb12_18 def"); end endcase
  case (36) {2{3'($unsigned(S8))}}: begin : h19 initial $display("@Rb12_19 hit"); end default: begin : d19 initial $display("@Rb12_19 def"); end endcase
  case (1'h1) ({P65, S4} || {3'((-5)), P65}): begin : h20 initial $display("@Rb12_20 hit"); end default: begin : d20 initial $display("@Rb12_20 def"); end endcase
  case (1'sh1) ({P65, S4} || {3'((-5)), P65}): begin : h21 initial $display("@Rb12_21 hit"); end default: begin : d21 initial $display("@Rb12_21 def"); end endcase
  case (4'h1) ({P65, S4} || {3'((-5)), P65}): begin : h22 initial $display("@Rb12_22 hit"); end default: begin : d22 initial $display("@Rb12_22 def"); end endcase
  case (64'hffffffffffffffff) ({P65, S4} || {3'((-5)), P65}): begin : h23 initial $display("@Rb12_23 hit"); end default: begin : d23 initial $display("@Rb12_23 def"); end endcase
  case ((-1)) ({P65, S4} || {3'((-5)), P65}): begin : h24 initial $display("@Rb12_24 hit"); end default: begin : d24 initial $display("@Rb12_24 def"); end endcase
endmodule
