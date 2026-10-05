package pk;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
endpackage
module mb;
  int n = 0;
  initial begin repeat (pk::f(2)) n++; #2 $display("mb n=%0d", n); end
endmodule
module ma;
  localparam int P = pk::f(2);
  initial #1 $display("ma P=%0d", P);
endmodule
module top;
  mb b();
  ma a();
  initial #5 $finish;
endmodule
