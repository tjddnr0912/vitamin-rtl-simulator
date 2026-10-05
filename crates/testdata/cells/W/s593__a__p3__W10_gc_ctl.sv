`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  case (1'b1)
    ((X >>> 1) ==? 8'b1111_11?0): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
endmodule
module t;
  m u();
  initial #5 $finish;
endmodule
