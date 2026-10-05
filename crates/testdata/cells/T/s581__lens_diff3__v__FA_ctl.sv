module top;
  function automatic int finc(input int a); return a + 5; endfunction
  localparam int R = finc(4'd12 == 4'd12);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
