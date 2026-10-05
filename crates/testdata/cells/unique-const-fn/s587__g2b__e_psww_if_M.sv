module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] v = 0;
  initial begin #1 v[0 +: f(2)] = '1; $display("pw=%h", v); $finish; end
endmodule
