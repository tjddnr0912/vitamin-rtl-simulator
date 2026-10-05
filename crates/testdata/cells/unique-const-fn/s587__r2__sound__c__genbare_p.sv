module top;
  function automatic int f(input int a);
    if (a == 1) return 1;
    return 7;
  endfunction
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
  end
  initial begin $display("P=%0d", g.P); #1 $finish; end
endmodule
