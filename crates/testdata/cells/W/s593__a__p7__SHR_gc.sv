`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  case (1'b1)
    ((SA >>> 1) ==? 4'sb111?): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
