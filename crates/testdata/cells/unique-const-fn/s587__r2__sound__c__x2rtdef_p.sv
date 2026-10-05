module top;
  typedef logic [3:0] T;
  function automatic T f(input int a);
    if (a == 1) f = 1;
  endfunction
  localparam logic [31:0] P = f(2);
  initial begin $display("P=%h", P); #1 $finish; end
endmodule
