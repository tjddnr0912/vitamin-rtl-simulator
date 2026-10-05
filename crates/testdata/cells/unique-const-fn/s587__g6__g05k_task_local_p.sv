module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  task automatic t;
    logic [f(2):0] x;
    x = '1;
    $display("tb=%0d x=%h", $bits(x), x);
  endtask
  initial begin #1 t(); $finish; end
endmodule
