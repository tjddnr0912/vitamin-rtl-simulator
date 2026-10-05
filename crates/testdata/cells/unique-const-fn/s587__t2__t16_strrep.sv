module top;
  function automatic int f(input int a);
    f = 2;
    unique if (a == 1) f = 10;
  endfunction
  string s;
  initial begin #1 s = {f(2){"ab"}}; $display("s=%s", s); $finish; end
endmodule
