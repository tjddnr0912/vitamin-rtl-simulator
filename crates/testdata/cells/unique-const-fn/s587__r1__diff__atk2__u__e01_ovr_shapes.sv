module child #(parameter real R = 1.0, parameter logic [7:0] S = 0, parameter U = 0,
               parameter [99:0] WD = 0, parameter string STR = "x", parameter int I = 0) ();
  initial #1 $display("%m R=%0.2f S=%h U=%0d bU=%0d WD=%0d STR=%s I=%0d", R, S, U, $bits(U), WD, STR, I);
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  child #(.R(f(2) * 1.5)) a();
  child #(.S(f(2))) b();
  child #(.U(f(2))) c();
  child #(.WD(f(2))) d();
  child #(.STR(f(2) == 7 ? "yes" : "no")) e();
  child g();
  defparam g.I = f(2);
  initial #5 $finish;
endmodule
