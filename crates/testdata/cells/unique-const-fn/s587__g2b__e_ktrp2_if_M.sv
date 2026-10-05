module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int n;
  task automatic w;
    repeat (f(2)) n++;
  endtask
  initial begin n = 0; #1 w(); $display("n=%0d", n); $finish; end
endmodule
