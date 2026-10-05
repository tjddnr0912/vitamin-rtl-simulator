`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  case (1'b1)
    (fx(2) ==? 4'b0?00): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
