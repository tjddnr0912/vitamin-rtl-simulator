module dut;
  logic [15:0] v = 16'hABCD;
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  dut d();
  initial begin #1 $display("r=%h", d.v[f(2):0]); $finish; end
endmodule
