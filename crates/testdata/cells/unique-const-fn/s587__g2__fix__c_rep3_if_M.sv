module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] r;
  initial begin #1 r = {f(2){2'b01}}; $display("r=%h", r); $finish; end
endmodule
