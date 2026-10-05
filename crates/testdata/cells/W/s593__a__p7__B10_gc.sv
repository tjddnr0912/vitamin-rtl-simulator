`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    case (1'b1)
      (GS ==? 4'sb1?00): begin : gc initial #1 $display("GC=item"); end
      default: begin : gc initial #1 $display("GC=def"); end
    endcase
  end
  initial #5 $finish;
endmodule
