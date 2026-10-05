module top;

  function automatic int fwc2(); return (4'd12 ==? 4'b1?00); endfunction
  localparam int R = fwc2();
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
