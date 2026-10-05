module top;
  logic a;
  function automatic logic f(input logic v, input integer id);
    $display("f%0d t=%0t v=%b", id, $time, v);
    f = v;
  endfunction
  function automatic logic [2:0] g3(input logic [2:0] v, input integer id);
    $display("g%0d t=%0t v=%b", id, $time, v);
    g3 = v;
  endfunction
  wire [2:0] v; wire [1:0] s; wire t, u; wire [2:0] R, Q;
  assign R = g3(v, 1);
  assign Q = g3({1'b0, s}, 2);
  assign v[2] = f(u, 4);
  assign v[1] = f(t, 2);
  assign v[0] = f(a, 1);
  assign u = f(t, 5);
  assign t = f(a, 3);
  assign s[1] = f(s[0], 7);
  assign s[0] = f(a, 6);
  initial begin a = 1; #1 $display("t1 v=%b s=%b R=%b Q=%b", v, s, R, Q); $finish; end
  initial #10 $finish;
endmodule
