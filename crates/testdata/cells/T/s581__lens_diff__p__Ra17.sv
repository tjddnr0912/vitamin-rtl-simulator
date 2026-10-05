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
  case (4'h0) $signed((!I)): begin : h0 initial $display("@Ra17_0 hit"); end default: begin : d0 initial $display("@Ra17_0 def"); end endcase
  case (64'h0) $signed((!I)): begin : h1 initial $display("@Ra17_1 hit"); end default: begin : d1 initial $display("@Ra17_1 def"); end endcase
  case (0) $signed((!I)): begin : h2 initial $display("@Ra17_2 hit"); end default: begin : d2 initial $display("@Ra17_2 def"); end endcase
  case (0) $signed((!I)): begin : h3 initial $display("@Ra17_3 hit"); end default: begin : d3 initial $display("@Ra17_3 def"); end endcase
  case (32'he) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h4 initial $display("@Ra17_4 hit"); end default: begin : d4 initial $display("@Ra17_4 def"); end endcase
  case (32'she) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h5 initial $display("@Ra17_5 hit"); end default: begin : d5 initial $display("@Ra17_5 def"); end endcase
  case (35'he) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h6 initial $display("@Ra17_6 hit"); end default: begin : d6 initial $display("@Ra17_6 def"); end endcase
  case (64'he) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h7 initial $display("@Ra17_7 hit"); end default: begin : d7 initial $display("@Ra17_7 def"); end endcase
  case (14) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h8 initial $display("@Ra17_8 hit"); end default: begin : d8 initial $display("@Ra17_8 def"); end endcase
  case (14) ((U32 << 1) ? $unsigned(14) : $unsigned(16'h13ea)): begin : h9 initial $display("@Ra17_9 hit"); end default: begin : d9 initial $display("@Ra17_9 def"); end endcase
  case (8'hf0) (-(-P8)): begin : h10 initial $display("@Ra17_10 hit"); end default: begin : d10 initial $display("@Ra17_10 def"); end endcase
  case (8'shf0) (-(-P8)): begin : h11 initial $display("@Ra17_11 hit"); end default: begin : d11 initial $display("@Ra17_11 def"); end endcase
  case (11'hf0) (-(-P8)): begin : h12 initial $display("@Ra17_12 hit"); end default: begin : d12 initial $display("@Ra17_12 def"); end endcase
  case (64'hfffffffffffffff0) (-(-P8)): begin : h13 initial $display("@Ra17_13 hit"); end default: begin : d13 initial $display("@Ra17_13 def"); end endcase
  case ((-16)) (-(-P8)): begin : h14 initial $display("@Ra17_14 hit"); end default: begin : d14 initial $display("@Ra17_14 def"); end endcase
  case (240) (-(-P8)): begin : h15 initial $display("@Ra17_15 hit"); end default: begin : d15 initial $display("@Ra17_15 def"); end endcase
  case (32'hffffffdb) (-37): begin : h16 initial $display("@Ra17_16 hit"); end default: begin : d16 initial $display("@Ra17_16 def"); end endcase
  case (32'shffffffdb) (-37): begin : h17 initial $display("@Ra17_17 hit"); end default: begin : d17 initial $display("@Ra17_17 def"); end endcase
  case (35'hffffffdb) (-37): begin : h18 initial $display("@Ra17_18 hit"); end default: begin : d18 initial $display("@Ra17_18 def"); end endcase
  case (64'hffffffffffffffdb) (-37): begin : h19 initial $display("@Ra17_19 hit"); end default: begin : d19 initial $display("@Ra17_19 def"); end endcase
  case ((-37)) (-37): begin : h20 initial $display("@Ra17_20 hit"); end default: begin : d20 initial $display("@Ra17_20 def"); end endcase
  case (1'h1) (63'sh60bd27c0de27a24e < (-P4)): begin : h21 initial $display("@Ra17_21 hit"); end default: begin : d21 initial $display("@Ra17_21 def"); end endcase
  case (1'sh1) (63'sh60bd27c0de27a24e < (-P4)): begin : h22 initial $display("@Ra17_22 hit"); end default: begin : d22 initial $display("@Ra17_22 def"); end endcase
  case (4'h1) (63'sh60bd27c0de27a24e < (-P4)): begin : h23 initial $display("@Ra17_23 hit"); end default: begin : d23 initial $display("@Ra17_23 def"); end endcase
  case (64'hffffffffffffffff) (63'sh60bd27c0de27a24e < (-P4)): begin : h24 initial $display("@Ra17_24 hit"); end default: begin : d24 initial $display("@Ra17_24 def"); end endcase
endmodule
