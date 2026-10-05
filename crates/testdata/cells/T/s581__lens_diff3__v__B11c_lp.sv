module top;
  function automatic int finc(input int a); return a + 5; endfunction
  localparam R = (finc(7) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
