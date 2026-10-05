module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  string s [f(2)];
  initial begin s[6] = "z"; #1 $display("s6=%s", s[6]); $finish; end
endmodule
