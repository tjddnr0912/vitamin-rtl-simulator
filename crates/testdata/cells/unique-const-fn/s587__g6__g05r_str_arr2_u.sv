module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  string s [0:f(2)] = '{"a","b","c","d","e","f","g","h"};
  initial begin #1 $display("s7=%s s0=%s", s[7], s[0]); $finish; end
endmodule
