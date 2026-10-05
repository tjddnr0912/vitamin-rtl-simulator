`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  case (1'b1)
    (fr(2) inside {4'b0?00}): begin : gc initial $display("GC=item"); end
    default: begin : gc initial $display("GC=def"); end
  endcase
  initial #5 $finish;
endmodule
