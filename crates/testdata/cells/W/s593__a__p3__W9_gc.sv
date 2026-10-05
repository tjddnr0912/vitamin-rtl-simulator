`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  function automatic logic [7:0] f(input logic [7:0] v); return v; endfunction
  case (1'b1)
    (f(X) ==? 8'b1111_1?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
