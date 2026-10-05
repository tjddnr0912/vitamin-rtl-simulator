package pk;
  function automatic int f(input int a);
    return a + 1;
  endfunction
  localparam int P = f(1);
endpackage
module top;
  initial begin #1 $display("P=%0d", pk::P); $finish; end
endmodule
