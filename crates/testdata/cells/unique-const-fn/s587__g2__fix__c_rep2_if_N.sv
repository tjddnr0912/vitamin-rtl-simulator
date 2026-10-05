module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] r;
  assign r = {f(1){1'b1}};
  initial begin #5 $display("r=%h", r); $finish; end
endmodule
