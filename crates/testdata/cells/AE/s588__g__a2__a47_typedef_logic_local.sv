module top;
  typedef logic [3:0] lt;
  function automatic logic [3:0] f(input int a);
    lt t;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
