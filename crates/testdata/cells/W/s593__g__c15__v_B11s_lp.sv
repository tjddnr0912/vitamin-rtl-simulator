module top;
  function automatic logic signed [63:0] fs64(input int a); return -a; endfunction
  localparam R = (fs64(4) ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
