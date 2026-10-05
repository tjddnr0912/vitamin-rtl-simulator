module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  int a2 [4][8];
  initial begin #1 $display("l=%0d", $left(a2, f(2) - 6)); $finish; end
endmodule
