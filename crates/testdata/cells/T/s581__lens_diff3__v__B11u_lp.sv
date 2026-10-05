module top;
  function automatic logic [64:0] f65(input int a); return a; endfunction
  localparam R = (f65(12) ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
