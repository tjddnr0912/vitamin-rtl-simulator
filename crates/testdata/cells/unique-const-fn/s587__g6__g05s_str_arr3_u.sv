module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  string s [0:f(2)];
  initial begin s[7] = "z"; #1 $display("s7=%s", s[7]); $finish; end
endmodule
