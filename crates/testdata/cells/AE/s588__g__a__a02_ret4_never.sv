module top;
  function automatic logic [3:0] fx(input int a);
    int k;
    k = a;
  endfunction
  localparam logic [3:0] P = fx(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
