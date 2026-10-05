`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  case (1'b1)
    ((SA + 8'sd0) ==? 4'sb1?00): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
