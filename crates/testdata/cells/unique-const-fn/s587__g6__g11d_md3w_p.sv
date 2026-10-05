module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [3:0][7:0][3:0] m3 = 128'h0123456789ABCDEF_FEDCBA9876543210;
  initial begin #1 m3[1][f(2):0] = '0; $display("m=%h", m3); $finish; end
endmodule
