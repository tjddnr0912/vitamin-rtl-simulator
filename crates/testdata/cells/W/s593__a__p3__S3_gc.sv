`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  case (1'b1)
    (U inside {4'b1?00}): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
