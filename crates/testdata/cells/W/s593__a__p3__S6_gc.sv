`timescale 1ns/1ns
module t;
  localparam logic [63:0] B = 64'h8000_0000_0000_000C;
  case (1'b1)
    (B ==? 4'b1?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
