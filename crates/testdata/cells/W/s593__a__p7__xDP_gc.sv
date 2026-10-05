`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  case (1'b1)
    (P ==? 4'sb1?00): begin : gc initial #1 $display("GC=item"); end
    default: begin : gc initial #1 $display("GC=def"); end
  endcase
endmodule
module t;
  function automatic logic signed [63:0] fxs64(input int a); logic signed [63:0] t; return t - 64'sd4; endfunction
  sub u();
  defparam u.P = fxs64(2);
  initial #5 $finish;
endmodule
