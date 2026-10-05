module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [f(2):0] early;
  localparam int B = $bits(early);
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
