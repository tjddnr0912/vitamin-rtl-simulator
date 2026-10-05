module top;
  function automatic logic [39:0] f40(input int a); return 40'h10_0000_0000 + a; endfunction
  localparam R = (f40(12) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
