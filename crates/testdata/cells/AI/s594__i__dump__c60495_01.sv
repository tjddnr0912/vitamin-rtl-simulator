`timescale 1ns/1ns
module t;
  function automatic int f2(input int a); return a; endfunction
  initial #1 $display("rep=%h", {({f2(2){1'b0}} + 2'd3){4'hF}});
  initial #50 $finish;
endmodule
