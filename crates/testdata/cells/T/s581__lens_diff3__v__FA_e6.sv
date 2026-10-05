module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  function automatic int finc(input int a); return a + 5; endfunction
  localparam int R = finc(S64N ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
