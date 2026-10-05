module top;
  function automatic int f(input int a);
    f = 2;
    unique if (a == 1) f = 10;
  endfunction
  string s;
  logic [15:0] v = 16'hABCD;
  logic [3:0] r1, r2;
  initial begin
    s = {f(2){"ab"}};
    r1 = v[f(2) +: 4];
    r2 = v[f(3)*2 +: 4];
    #1 $display("s=%s r1=%h r2=%h", s, r1, r2);
    $info("i=%0d", f(2));
    $finish;
  end
endmodule
