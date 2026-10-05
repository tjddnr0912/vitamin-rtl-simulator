module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial begin : b
    string s [f(2)] = '{"a","b","c","d","e","f","g"};
    #1 $display("n=%0d s6=%s", $size(s), s[6]); $finish;
  end
endmodule
