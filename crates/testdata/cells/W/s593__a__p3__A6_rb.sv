`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  logic [(fr(2) inside {4'b0?00})+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
