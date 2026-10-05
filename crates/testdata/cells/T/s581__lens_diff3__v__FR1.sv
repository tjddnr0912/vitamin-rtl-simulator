module top;

  function automatic int fwc(input int a); return (a ==? 4'b1?00); endfunction
  localparam int R = fwc(12);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
