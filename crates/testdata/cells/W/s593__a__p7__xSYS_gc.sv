`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  case (1'b1)
    ($clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?}): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
