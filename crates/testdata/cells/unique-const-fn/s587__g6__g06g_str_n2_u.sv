module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  string s2 [f(2)] = '{"a","b","c","d","e","f","g"};
  initial begin #1 $display("s6=%s s0=%s", s2[6], s2[0]); $finish; end
endmodule
