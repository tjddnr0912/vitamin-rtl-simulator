`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  case (1'b1)
    (ES ==? 8'sb1111_1?00): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
