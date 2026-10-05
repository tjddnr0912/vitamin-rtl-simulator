`timescale 1ns/1ns
module t;
  case (1'b1)
    ((4'd15 + 4'd1) ==? 5'b1?000): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
