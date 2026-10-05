module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  string s [f(2)];
  string s2 [f(2)] = '{"a","b","c","d","e","f","g"};
  initial begin #1 $display("n=%0d n2=%0d s6=%s", $size(s), $size(s2), s2[6]); $finish; end
endmodule
