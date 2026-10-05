module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic u [f(2):0];
  initial begin
    u = '{1'b1,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0,1'b0};
    #1 $display("l=%0d r=%0d i=%0d u7=%b u0=%b", $left(u), $right(u), $increment(u), u[7], u[0]); $finish;
  end
endmodule
