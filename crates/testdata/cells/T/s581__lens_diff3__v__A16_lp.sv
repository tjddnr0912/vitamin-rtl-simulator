module top;

  localparam R = ($clog2(40) ==? 'sb1?0);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
