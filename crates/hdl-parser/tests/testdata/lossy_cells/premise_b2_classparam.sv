module t;
  class C #(parameter logic [7:0] P = 'x, parameter logic [7:0] K = 8'd1);
    function int wp(); return $bits(P); endfunction
    function int wk(); return $bits(K); endfunction
    function logic [11:0] cp(); return {P, 4'b0000}; endfunction
  endclass
  C c; initial begin c = new; #1 $display("A wp=%0d wk=%0d cp=%b", c.wp(), c.wk(), c.cp()); end
  initial #100 $finish;
endmodule
