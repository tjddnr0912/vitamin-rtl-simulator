module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  int n;
  initial begin n = 0; #1 repeat (f(2)) n++; $display("n=%0d", n); $finish; end
endmodule
