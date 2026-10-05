`timescale 1ns/1ns
module t;
  case (1'b1)
    (8'sb1111_1100 ==? 4'sb1?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
