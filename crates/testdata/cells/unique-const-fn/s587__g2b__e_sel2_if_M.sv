module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [15:0][3:0] m = 0;
  initial begin #1 m[f(2)] = 4'hA; $display("m=%h", m); $finish; end
endmodule
