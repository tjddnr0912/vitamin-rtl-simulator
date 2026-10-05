module dut;
  logic [15:0] v = 16'hFFFF;
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  dut d();
  logic [7:0] r;
  initial begin #1 r = (d.v[f(2)-1:0] + 7'h01) >> 1; $display("r=%h", r); $finish; end
endmodule
