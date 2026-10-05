`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  if (1) begin : g localparam T Y = -4; end
  case (1'b1)
    (g.Y ==? 8'b1111_1?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
