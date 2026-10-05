module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  task automatic t;
    string s [f(2)];
    s[6] = "z";
    #1 $display("n=%0d s6=%s", $size(s), s[6]);
  endtask
  initial begin #1 t(); $finish; end
endmodule
