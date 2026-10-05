module top;
  function automatic logic [31:0] h(input int a);
    int t = int'(2.5);
    t[0] = 1'b0;
    h = t;
  endfunction
  localparam int P = h(0);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #50 $finish;
endmodule
