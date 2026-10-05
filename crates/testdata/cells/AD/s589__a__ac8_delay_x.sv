package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x);
    logic [3:0] t;
    h = t;
  endfunction
endpackage
module top;
  reg r = 0; wire w;
  assign #(q::h(2)) w = r;
  initial begin r = 1; #1 $display("t1 w=%b", w); #2 $display("t3 w=%b", w); $finish; end
  initial #50 $finish;
endmodule
