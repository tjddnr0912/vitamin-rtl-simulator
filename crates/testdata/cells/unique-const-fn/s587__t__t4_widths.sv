module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [31:0] r1, r2, r3;
  logic [15:0] v = 16'hABCD, vw;
  logic [15:0][3:0] m = 64'h0123456789ABCDEF;
  assign r2 = {f(2){1'b1}};
  initial begin
    #1 r1 = {f(2){1'b1}};
    r3 = {f(2){2'b01}};
    $display("r1=%h r2=%h r3=%h", r1, r2, r3);
    $display("ps=%h pr=%h m=%h", v[0 +: f(2)], v[f(2):0], m[f(2):0]);
    vw = v;
    vw[0 +: f(2)] = '0;
    $display("psw=%h", vw);
    vw = v;
    vw[f(2):0] = '0;
    $display("prw=%h", vw);
    #1 $finish;
  end
endmodule
