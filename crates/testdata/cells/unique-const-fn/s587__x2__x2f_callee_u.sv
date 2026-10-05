module top;
  function automatic int g(input int a);
    g = 7;
    unique if (a == 1) g = 10;
  endfunction
  function automatic logic [3:0] f(input int a);
    if (g(a) == 3) f = 4'd1;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
