`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  case (1'b1)
    (SP ==? 4'sb?100): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
