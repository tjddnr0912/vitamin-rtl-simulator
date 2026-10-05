`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] A [0:1] = '{-4, 2};
  case (1'b1)
    (A[0] ==? 8'b1111_1?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
