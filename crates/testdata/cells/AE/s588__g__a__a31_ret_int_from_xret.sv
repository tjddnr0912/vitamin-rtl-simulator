module top;
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int P = g(3);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
