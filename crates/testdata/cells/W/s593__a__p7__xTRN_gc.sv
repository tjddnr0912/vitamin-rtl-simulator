`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction
  localparam bit C = 1;
  case (1'b1)
    ((C ? fxs(2) - 4'sd4 : 4'sd0) ==? 4'sb1?00): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
