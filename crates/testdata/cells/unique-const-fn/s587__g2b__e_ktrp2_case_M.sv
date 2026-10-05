module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  int n;
  task automatic w;
    repeat (f(2)) n++;
  endtask
  initial begin n = 0; #1 w(); $display("n=%0d", n); $finish; end
endmodule
