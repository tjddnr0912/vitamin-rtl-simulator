module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  localparam int B = $bits(late);
  logic [f(2):0] late;
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
