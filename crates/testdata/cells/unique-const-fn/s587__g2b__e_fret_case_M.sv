module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  function automatic logic [f(2):0] g();
    g = '1;
  endfunction
  initial begin #1 $display("b=%0d", $bits(g())); $finish; end
endmodule
