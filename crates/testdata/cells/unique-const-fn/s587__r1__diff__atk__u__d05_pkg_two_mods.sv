package pk;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
module ma;
  localparam int P = pk::f(2);
  initial #1 $display("ma P=%0d", P);
endmodule
module mb;
  int n = 0;
  initial begin repeat (pk::f(2)) n++; #2 $display("mb n=%0d", n); end
endmodule
module top;
  ma a();
  mb b();
  initial #5 $finish;
endmodule
