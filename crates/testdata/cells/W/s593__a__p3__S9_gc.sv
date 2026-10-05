`timescale 1ns/1ns
module t;
  localparam int I = 12;
  case (1'b1)
    (I inside {4'b1?00, 4'b0011}): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
