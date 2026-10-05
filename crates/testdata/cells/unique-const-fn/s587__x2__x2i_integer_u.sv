module top;
  function automatic integer f(input int a);
    integer t;
    unique if (a == 1) t = 10;
    f = t;
  endfunction
  localparam integer P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
