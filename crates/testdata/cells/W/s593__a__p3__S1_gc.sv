`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  case (1'b1)
    (SN ==? 4'b?100): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
