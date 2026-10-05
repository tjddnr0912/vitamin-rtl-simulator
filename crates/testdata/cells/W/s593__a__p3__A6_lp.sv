`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  localparam L = (fr(2) inside {4'b0?00});
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
