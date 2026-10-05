module top;
  function automatic int g(input int a);
    g = 7;
    unique case (a) 1: g = 10; 3: g = 30; endcase
  endfunction
  function automatic int f(input int a);
    f = g(a) + 1;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
