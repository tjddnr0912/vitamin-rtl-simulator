module top;
  function automatic logic [f(2):0] f(input int a); return a; endfunction
  localparam int P = f(3);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
