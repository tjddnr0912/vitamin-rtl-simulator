module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [f(1):0] v;
  initial begin #1 $display("bits=%0d", $bits(v)); $finish; end
endmodule
