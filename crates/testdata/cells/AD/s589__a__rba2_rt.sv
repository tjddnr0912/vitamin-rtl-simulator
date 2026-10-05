package q;
  function automatic int f(input int a);
    case (a)
      2: f = 3;
      default: f = 9;
    endcase
  endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); return 3; endfunction
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
  initial #50 $finish;
endmodule
