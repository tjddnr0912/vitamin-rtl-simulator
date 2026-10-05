module top;

  function automatic int fwc3(input int a); return a + (4'd12 ==? 4'b1?00); endfunction
  localparam int R = fwc3(4);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
